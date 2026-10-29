//! Shared activity library. All mutations are scoped to the authenticated owner.
use axum::{
    Extension, Json, body::Bytes, extract::{Path, State},
    http::{StatusCode, header, HeaderMap}, response::IntoResponse,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter, TransactionTrait, Condition};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{access,entities::{catalog_activity, catalog_material, catalog_selection, catalog_result, training_activity}, routes::authentication::login::Claims, state::AppState};

#[derive(Serialize)]
pub struct CatalogEntry {
    #[serde(flatten)]
    pub activity: catalog_activity::Model,
    pub materials: Vec<catalog_material::Model>,
}
#[derive(Deserialize)]
pub struct ActivityPayload {
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
}
#[derive(Deserialize)]
pub struct MaterialPayload {
    pub name: String,
    pub quantity: i32,
}
#[derive(Serialize)]
pub struct ImageResponse { pub image_url: String }

pub fn valid_image(value: Option<&str>) -> bool {
    value.is_none_or(|url| {
        url.is_empty() || url.len() <= 2000 &&
        (url.starts_with("https://") || url.starts_with("http://") || url.starts_with("/catalog/images/"))
    })
}
/// Strip active content before returning HTML to the Dioxus editor/print view.
pub fn sanitize_description(value:&str)->String{
    ammonia::clean(value)
}

pub fn valid_payload(body: &ActivityPayload) -> bool {
    !body.name.trim().is_empty() && body.name.len() <= 150 &&
        body.description.len() <= 20_000 && valid_image(body.image_url.as_deref())
}
pub async fn owned(
    claims: &Claims, state: &AppState, id: Uuid
) -> Result<catalog_activity::Model, StatusCode> {
    let item=catalog_activity::Entity::find_by_id(id).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    if let Some(team_id)=item.team_id{
        access::assert_team(state,claims.sub,team_id,false).await?;
    } else if item.user_id!=claims.sub && access::effective_role(state,claims.sub).await?!=access::Role::Administrator {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(item)
}
pub async fn writable(claims:&Claims,state:&AppState,id:Uuid)->Result<catalog_activity::Model,StatusCode>{
    let item=owned(claims,state,id).await?;
    if let Some(team_id)=item.team_id{access::assert_team(state,claims.sub,team_id,true).await?}
    Ok(item)
}

pub async fn insert(
    claims: &Claims, state: &AppState, body: ActivityPayload
) -> Result<catalog_activity::Model, StatusCode> {
    if !valid_payload(&body) { return Err(StatusCode::BAD_REQUEST); }
    let team_id=access::default_writable_team(state,claims.sub).await?;
    catalog_activity::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(claims.sub),
        team_id: Set(Some(team_id)),
        name: Set(body.name.trim().into()),
        description: Set(sanitize_description(&body.description)),
        image_url: Set(body.image_url.filter(|s| !s.is_empty())),
        ..Default::default()
    }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn list(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<CatalogEntry>>, StatusCode> {
    let teams=access::teams_for(&state,claims.sub,false).await?;
    let activities = catalog_activity::Entity::find()
        .filter(Condition::any()
            .add(catalog_activity::COLUMN.team_id.is_in(teams))
            .add(Condition::all()
                .add(catalog_activity::COLUMN.user_id.eq(claims.sub))
                .add(catalog_activity::COLUMN.team_id.is_null())))
        .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let ids: Vec<Uuid> = activities.iter().map(|a| a.id).collect();
    let materials = if ids.is_empty() { Vec::new() } else {
        catalog_material::Entity::find()
            .filter(catalog_material::COLUMN.activity_id.is_in(ids))
            .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    Ok(Json(activities.into_iter().map(|mut a| {
        let id=a.id;
        a.description=sanitize_description(&a.description);
        CatalogEntry { activity:a,
            materials:materials.iter().filter(|m| m.activity_id==id).cloned().collect() }
    }).collect()))
}

pub async fn create(
    Extension(claims): Extension<Claims>, State(state): State<AppState>, Json(body): Json<ActivityPayload>,
) -> Result<(StatusCode, Json<catalog_activity::Model>), StatusCode> {
    let activity = insert(&claims, &state, body).await?;
    Ok((StatusCode::CREATED, Json(activity)))
}

pub async fn update(
    Extension(claims): Extension<Claims>, State(state): State<AppState>, Path(id): Path<Uuid>, Json(body): Json<ActivityPayload>
) -> Result<StatusCode, StatusCode> {
    if !valid_payload(&body) { return Err(StatusCode::BAD_REQUEST); }
    let activity = writable(&claims, &state, id).await?;
    let mut active: catalog_activity::ActiveModel = activity.into();
    active.name = Set(body.name.trim().into());
    active.description = Set(sanitize_description(&body.description));
    active.image_url = Set(body.image_url.filter(|s| !s.is_empty()));
    active.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    Extension(claims): Extension<Claims>, State(state): State<AppState>, Path(id): Path<Uuid>
) -> Result<StatusCode, StatusCode> {
    let item = writable(&claims, &state, id).await?;
    // Do not orphan existing training and spreadsheet results.
    let in_training = training_activity::Entity::find()
        .filter(training_activity::COLUMN.catalog_id.eq(id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if in_training.is_some() { return Err(StatusCode::CONFLICT); }
    let txn = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    catalog_selection::Entity::delete_many()
        .filter(catalog_selection::COLUMN.user_id.eq(claims.sub))
        .filter(catalog_selection::COLUMN.activity_id.eq(id))
        .exec(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    catalog_result::Entity::delete_many()
        .filter(catalog_result::COLUMN.user_id.eq(claims.sub))
        .filter(catalog_result::COLUMN.activity_id.eq(id))
        .exec(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    catalog_material::Entity::delete_many()
        .filter(catalog_material::COLUMN.activity_id.eq(id))
        .exec(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let active: catalog_activity::ActiveModel = item.into();
    active.delete(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    txn.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_material(
    Extension(claims): Extension<Claims>, State(state): State<AppState>,
    Path(id): Path<Uuid>, Json(body): Json<MaterialPayload>,
) -> Result<(StatusCode, Json<catalog_material::Model>), StatusCode> {
    writable(&claims, &state, id).await?;
    if body.name.trim().is_empty() || body.name.len() > 150 || !(1..=100000).contains(&body.quantity) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let item = catalog_material::ActiveModel {
        id: Set(Uuid::new_v4()), activity_id: Set(id),
        name: Set(body.name.trim().into()), quantity: Set(body.quantity), ..Default::default()
    }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(item)))
}


pub async fn update_material(
    Extension(claims):Extension<Claims>,
    State(state):State<AppState>,
    Path((id,material_id)):Path<(Uuid,Uuid)>,
    Json(body):Json<MaterialPayload>,
)->Result<StatusCode,StatusCode>{
    owned(&claims,&state,id).await?;
    if body.name.trim().is_empty() || body.name.len()>150 ||
       !(1..=100_000).contains(&body.quantity){return Err(StatusCode::BAD_REQUEST)}
    let material=catalog_material::Entity::find_by_id(material_id)
        .filter(catalog_material::COLUMN.activity_id.eq(id))
        .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut model:catalog_material::ActiveModel=material.into();
    model.name=Set(body.name.trim().to_owned());
    model.quantity=Set(body.quantity);
    model.update(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_material(
    Extension(claims): Extension<Claims>, State(state): State<AppState>,
    Path((id, material_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    writable(&claims, &state, id).await?;
    let material = catalog_material::Entity::find_by_id(material_id)
        .filter(catalog_material::COLUMN.activity_id.eq(id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let active: catalog_material::ActiveModel = material.into();
    active.delete(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Only JPEG, PNG, WebP and GIF bytes are accepted, never SVG or HTML.
fn image_kind(data: &[u8]) -> Option<(&'static str, &'static str)> {
    if data.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]) { Some(("png", "image/png")) }
    else if data.starts_with(&[0xff, 0xd8, 0xff]) { Some(("jpg", "image/jpeg")) }
    else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") { Some(("webp", "image/webp")) }
    else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") { Some(("gif", "image/gif")) }
    else { None }
}

pub async fn upload_image(
    Extension(_claims): Extension<Claims>,
    _state: State<AppState>,
    bytes: Bytes,
) -> Result<(StatusCode, Json<ImageResponse>), StatusCode> {
    if bytes.is_empty() || bytes.len() > 4 * 1024 * 1024 { return Err(StatusCode::PAYLOAD_TOO_LARGE); }
    let (extension, _) = image_kind(&bytes).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    tokio::fs::create_dir_all("data/uploads").await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let filename = format!("{}.{}", Uuid::new_v4(), extension);
    tokio::fs::write(format!("data/uploads/{filename}"), &bytes).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(ImageResponse { image_url: format!("/catalog/images/{filename}") })))
}

pub async fn image(Path(filename): Path<String>) -> Result<impl IntoResponse, StatusCode> {
    let (id, ext) = filename.rsplit_once('.').ok_or(StatusCode::NOT_FOUND)?;
    Uuid::parse_str(id).map_err(|_| StatusCode::NOT_FOUND)?;
    let mime = match ext {
        "png" => "image/png", "jpg" => "image/jpeg", "webp" => "image/webp", "gif" => "image/gif",
        _ => return Err(StatusCode::NOT_FOUND),
    };
    let bytes = tokio::fs::read(format!("data/uploads/{filename}")).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, "private, max-age=3600"),
         (header::X_CONTENT_TYPE_OPTIONS, "nosniff")], bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_image_signatures_pass() {
        assert!(image_kind(b"\x89PNG\r\n\x1a\nxxxxx").is_some());
        assert!(image_kind(b"<svg onload=alert(1)>").is_none());
        assert!(image_kind(b"<html>bad</html>").is_none());
    }
}
