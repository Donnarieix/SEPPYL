//! Activity library. Personal definitions remain private; team definitions are shared only with their assigned team.
use axum::{
    Extension, Json, body::Bytes, extract::{Path, State},
    http::{StatusCode, header, HeaderMap}, response::IntoResponse,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{entities::{catalog_activity, catalog_material, catalog_selection, catalog_result, training_activity}, routes::{authentication::login::Claims,entities::organization}, state::AppState};

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
    pub description_html: Option<String>,
    pub image_url: Option<String>,
    pub team_id: Option<Uuid>,
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
pub fn valid_payload(body: &ActivityPayload) -> bool {
    !body.name.trim().is_empty() && body.name.len() <= 150 &&
        body.description.len() <= 20_000 &&
        body.description_html.as_ref().is_none_or(|html| html.len() <= 40_000) &&
        valid_image(body.image_url.as_deref())
}
pub async fn owned(
    claims: &Claims, state: &AppState, id: Uuid
) -> Result<catalog_activity::Model, StatusCode> {
    let item=catalog_activity::Entity::find_by_id(id)
        .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let role=organization::user_role(state,claims.sub).await?;
    let allowed=match item.team_id {
        Some(team_id)=>organization::can_read_team(state,claims,role,team_id).await?,
        None=>item.user_id==claims.sub || role.is_admin(),
    };
    if allowed{Ok(item)}else{Err(StatusCode::NOT_FOUND)}
}
pub async fn writable(claims:&Claims,state:&AppState,id:Uuid)
    ->Result<catalog_activity::Model,StatusCode>{
    let item=owned(claims,state,id).await?;
    let role=organization::user_role(state,claims.sub).await?;
    if !role.can_write(){return Err(StatusCode::FORBIDDEN);}
    if let Some(team_id)=item.team_id {
        if !organization::can_write_team(state,claims,role,team_id).await?{
            return Err(StatusCode::NOT_FOUND);
        }
    } else if item.user_id!=claims.sub && !role.is_admin() {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(item)
}
pub async fn insert(
    claims: &Claims, state: &AppState, body: ActivityPayload
) -> Result<catalog_activity::Model, StatusCode> {
    if !valid_payload(&body) { return Err(StatusCode::BAD_REQUEST); }
    if let Some(team_id)=body.team_id {
        let role=organization::user_role(state,claims.sub).await?;
        if !role.can_write() || !organization::can_write_team(state,claims,role,team_id).await? {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    catalog_activity::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(claims.sub),
        team_id: Set(body.team_id),
        name: Set(body.name.trim().into()),
        description: Set(body.description),
        description_html: Set(body.description_html.map(|html| ammonia::clean(&html))),
        image_url: Set(body.image_url.filter(|s| !s.is_empty())),
        ..Default::default()
    }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn list(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<CatalogEntry>>, StatusCode> {
    let all=catalog_activity::Entity::find().all(&state.db)
        .await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let role=organization::user_role(&state,claims.sub).await?;
    let mut activities=Vec::new();
    for activity in all {
        let visible=match activity.team_id {
            Some(team_id)=>organization::can_read_team(&state,&claims,role,team_id).await?,
            None=>activity.user_id==claims.sub || role.is_admin(),
        };
        if visible {activities.push(activity);}
    }
    let ids: Vec<Uuid> = activities.iter().map(|a| a.id).collect();
    let materials = if ids.is_empty() { Vec::new() } else {
        catalog_material::Entity::find()
            .filter(catalog_material::COLUMN.activity_id.is_in(ids))
            .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    Ok(Json(activities.into_iter().map(|a| CatalogEntry {
        materials: materials.iter().filter(|m| m.activity_id == a.id).cloned().collect(),
        activity: a,
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
    if let Some(team_id)=body.team_id {
        let role=organization::user_role(&state,claims.sub).await?;
        if !organization::can_write_team(&state,&claims,role,team_id).await? {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    let mut active: catalog_activity::ActiveModel = activity.into();
    if let Some(team_id)=body.team_id {active.team_id=Set(Some(team_id));}
    active.name = Set(body.name.trim().into());
    active.description = Set(body.description);
    // Older clients that omit rich HTML should not erase existing formatting.
    if let Some(html) = body.description_html {
        active.description_html = Set(Some(ammonia::clean(&html)));
    }
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


#[derive(Deserialize)]
pub struct UpdateMaterialPayload {
    pub name: String,
    pub quantity: i32,
}
pub async fn update_material(
    Extension(claims): Extension<Claims>, State(state): State<AppState>,
    Path((id, material_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateMaterialPayload>,
) -> Result<Json<catalog_material::Model>, StatusCode> {
    owned(&claims, &state, id).await?;
    if body.name.trim().is_empty() || body.name.len() > 150 ||
       !(1..=100_000).contains(&body.quantity) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let item = catalog_material::Entity::find_by_id(material_id)
        .filter(catalog_material::COLUMN.activity_id.eq(id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut active: catalog_material::ActiveModel = item.into();
    active.name = Set(body.name.trim().to_string());
    active.quantity = Set(body.quantity);
    let saved = active.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(saved))
}

pub async fn remove_material(
    Extension(claims): Extension<Claims>, State(state): State<AppState>,
    Path((id, material_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    owned(&claims, &state, id).await?;
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


#[cfg(test)]
mod richer_catalog_tests {
    use super::*;
    use crate::entities::user;
    use sea_orm::Database;

    #[test]
    fn unsafe_html_is_sanitized_before_storage() {
        let cleaned = ammonia::clean("<p onclick='alert(1)'><strong>Objectif</strong><script>alert(1)</script></p>");
        assert!(cleaned.contains("<strong>Objectif</strong>"));
        assert!(!cleaned.contains("onclick"));
        assert!(!cleaned.contains("<script"));
    }

    #[tokio::test]
    async fn equipment_name_and_quantity_can_be_changed() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state = AppState { db };
        let uid=Uuid::new_v4();
        user::ActiveModel {
            id:Set(uid),username:Set("coach-test".into()),password:Set("test".into()),
            ..Default::default()
        }.insert(&state.db).await.unwrap();
        let claims=Claims{sub:uid,exp:u64::MAX};
        let (_,activity)=create(Extension(claims.clone()),State(state.clone()),
            Json(ActivityPayload{
                name:"Passes".into(),description:"Consignes".into(),
                description_html:Some("<p><strong>Passes</strong></p>".into()),
                image_url:None,
                team_id:None
            })).await.unwrap();
        assert_eq!(activity.0.description_html.as_deref(),Some("<p><strong>Passes</strong></p>"));
        let id=activity.0.id;
        let (_,material)=add_material(
            Extension(claims.clone()),State(state.clone()),Path(id),
            Json(MaterialPayload{name:"Ballons".into(),quantity:3})
        ).await.unwrap();
        update_material(
            Extension(claims.clone()),State(state.clone()),Path((id,material.0.id)),
            Json(UpdateMaterialPayload{name:"Chasubles".into(),quantity:8})
        ).await.unwrap();
        let rows=list(Extension(claims),State(state)).await.unwrap().0;
        assert_eq!(rows[0].materials[0].name,"Chasubles");
        assert_eq!(rows[0].materials[0].quantity,8);
    }
}
