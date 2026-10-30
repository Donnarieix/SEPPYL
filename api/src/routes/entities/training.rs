use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use sea_orm::entity::prelude::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    entities::{
        player, training, training_activity, training_attendance, training_material, training_score, catalog_activity, catalog_material,
    },
    routes::authentication::login::Claims,
    services,
    routes::entities::{catalog, organization},
    state::AppState,
};

type ApiError = StatusCode;

async fn access_training(
    claims:&Claims, state:&AppState, id:Uuid, write:bool,
) -> Result<training::Model,ApiError> {
    let found=training::Entity::find_by_id(id).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let role=organization::user_role(state,claims.sub).await?;
    let allowed=match found.team_id {
        Some(team_id) if write =>
            role.can_write() && organization::can_write_team(state,claims,role,team_id).await?,
        Some(team_id) => organization::can_read_team(state,claims,role,team_id).await?,
        None if write => role.can_write() && (found.user_id==claims.sub || role==crate::middleware::Role::Administrator),
        None => found.user_id==claims.sub || role==crate::middleware::Role::Administrator,
    };
    if !allowed{return Err(StatusCode::NOT_FOUND);}
    Ok(found)
}
async fn owned_training(claims:&Claims,state:&AppState,id:Uuid)->Result<training::Model,ApiError>{
    access_training(claims,state,id,true).await
}

async fn owned_activity(
    claims: &Claims,
    state: &AppState,
    training_id: Uuid,
    activity_id: Uuid,
) -> Result<training_activity::Model, ApiError> {
    owned_training(claims, state, training_id).await?;
    training_activity::Entity::find_by_id(activity_id)
        .filter(training_activity::COLUMN.training_id.eq(training_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Serialize)]
pub struct TrainingDetail {
    pub training: training::Model,
    pub players: Vec<player::Model>,
    pub attendance: Vec<training_attendance::Model>,
    pub activities: Vec<training_activity::Model>,
    pub materials: Vec<training_material::Model>,
    pub scores: Vec<training_score::Model>,
}

#[derive(Deserialize)]
pub struct CreateTraining {
    pub name: String,
    pub date: DateTime,
    pub team_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateTraining {
    name: String,
    date: DateTime,
    notes: String,
}

#[derive(Deserialize)]
pub struct CreateActivity {
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateMaterial {
    name: String,
    quantity: i32,
}

#[derive(Deserialize)]
pub struct AttendanceUpdate {
    player_ids: Vec<Uuid>,
}

#[derive(Deserialize)]
pub struct ScoreUpdate {
    player_id: Uuid,
    activity_id: Option<Uuid>,
    score: Option<i32>,
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 150
}

fn valid_score(score: Option<i32>) -> bool {
    score.is_none_or(|value| (0..=5).contains(&value))
}

fn valid_image_url(url: Option<&str>) -> bool {
    url.is_none_or(|url| {
        url.len() <= 2000 && (url.starts_with("https://") || url.starts_with("http://"))
    })
}

pub async fn list(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<training::Model>>, ApiError> {
    let role=organization::user_role(&state,claims.sub).await?;
    let all=training::Entity::find()
        .order_by_desc(training::COLUMN.date).all(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut allowed=Vec::new();
    for session in all {
        let visible=match session.team_id {
            Some(id) => organization::can_read_team(&state,&claims,role,id).await?,
            None => session.user_id==claims.sub || role==crate::middleware::Role::Administrator,
        };
        if visible {allowed.push(session);}
    }
    Ok(Json(allowed))
}

pub async fn create(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(body): Json<CreateTraining>,
) -> Result<(StatusCode, String), ApiError> {
    if !valid_name(&body.name) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let role=organization::user_role(&state,claims.sub).await?;
    if !role.can_write(){return Err(StatusCode::FORBIDDEN);}
    if let Some(team_id)=body.team_id {
        if !organization::can_write_team(&state,&claims,role,team_id).await? {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    let new = training::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(claims.sub),
        team_id: Set(body.team_id),
        name: Set(body.name.trim().to_string()),
        date: Set(body.date),
        notes: Set(String::new()),
        ..Default::default()
    };
    let item = new.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, item.id.to_string()))
}

pub async fn get(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TrainingDetail>, ApiError> {
    let training = access_training(&claims, &state, id, false).await?;
    let players = if let Some(team_id)=training.team_id {
        organization::list_team_players_internal(&state,team_id).await?
    } else {
        services::player::get_players(&claims,&state).await?
    };
    let attendance = training_attendance::Entity::find()
        .filter(training_attendance::COLUMN.training_id.eq(id))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut activities = training_activity::Entity::find()
        .filter(training_activity::COLUMN.training_id.eq(id))
        .order_by_asc(training_activity::COLUMN.sort_order)
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let activity_ids: Vec<Uuid> = activities.iter().map(|a| a.id).collect();
    let (materials, scores) = if activity_ids.is_empty() {
        (Vec::new(), Vec::new())
    } else {
        let materials = training_material::Entity::find()
            .filter(training_material::COLUMN.activity_id.is_in(activity_ids.clone()))
            .all(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let scores = training_score::Entity::find()
            .filter(training_score::COLUMN.activity_id.is_in(activity_ids))
            .all(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        (materials, scores)
    };

    // Activity definitions are shared: edits to the catalog update the training
    // and its printable handout, while score IDs remain training-specific.
    let mut materials = materials;
    for activity in &mut activities {
        let Some(catalog_id) = activity.catalog_id else { continue; };
        let Some(definition) = catalog_activity::Entity::find_by_id(catalog_id)
            .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? else { continue };
        activity.name = definition.name;
        activity.description = definition.description;
        activity.description_html = definition.description_html;
        activity.image_url = definition.image_url;
        let equipment = catalog_material::Entity::find()
            .filter(catalog_material::COLUMN.activity_id.eq(catalog_id))
            .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        materials.extend(equipment.into_iter().map(|item| training_material::Model {
            id: item.id, activity_id: activity.id, name: item.name, quantity: item.quantity,
        }));
    }

    Ok(Json(TrainingDetail {
        training, players, attendance, activities, materials, scores,
    }))
}

pub async fn update(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTraining>,
) -> Result<StatusCode, ApiError> {
    if !valid_name(&body.name) || body.notes.len() > 20_000 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let training = owned_training(&claims, &state, id).await?;
    let mut active: training::ActiveModel = training.into();
    active.name = Set(body.name.trim().to_string());
    active.date = Set(body.date);
    active.notes = Set(body.notes);
    active.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_activity(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateActivity>,
) -> Result<(StatusCode, String), ApiError> {
    owned_training(&claims, &state, id).await?;
    if !valid_name(&body.name) || body.description.len() > 20_000
        || !valid_image_url(body.image_url.as_deref()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let count = training_activity::Entity::find()
        .filter(training_activity::COLUMN.training_id.eq(id))
        .all(&state.db).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.len();

    let definition = catalog::insert(&claims, &state, catalog::ActivityPayload {
        name: body.name.clone(),
        description: body.description.clone(),
        description_html: None,
        image_url: body.image_url.clone(),
        team_id: access_training(&claims,&state,id,true).await?.team_id,
    }).await?;
    let new = training_activity::ActiveModel {
        id: Set(Uuid::new_v4()),
        training_id: Set(id),
        catalog_id: Set(Some(definition.id)),
        name: Set(definition.name),
        description: Set(definition.description),
        description_html: Set(definition.description_html),
        image_url: Set(definition.image_url),
        sort_order: Set(count as i32),
        ..Default::default()
    };
    let saved = new.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, saved.id.to_string()))
}



#[derive(Deserialize)]
pub struct EditSessionActivity {
    pub name:String,
    pub description:String,
    pub description_html:Option<String>,
    pub image_url:Option<String>,
}
/// Only edits the copy in this training. The reusable library definition
/// remains unchanged, so other sessions are never modified implicitly.
pub async fn update_activity(
    Extension(claims):Extension<Claims>,
    State(state):State<AppState>,
    Path((training_id,activity_id)):Path<(Uuid,Uuid)>,
    Json(body):Json<EditSessionActivity>,
)->Result<StatusCode,ApiError>{
    let activity=owned_activity(&claims,&state,training_id,activity_id).await?;
    if !valid_name(&body.name) || body.description.len()>20_000
        || body.description_html.as_ref().is_some_and(|v|v.len()>40_000)
        || !valid_image_url(body.image_url.as_deref()){
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut active:training_activity::ActiveModel=activity.into();
    active.name=Set(body.name.trim().into());
    active.description=Set(body.description);
    active.description_html=Set(body.description_html.map(|html|ammonia::clean(&html)));
    active.image_url=Set(body.image_url);
    active.update(&state.db).await.map_err(|error|{
        tracing::error!(%error,training=%training_id,activity=%activity_id,
            "Could not update session activity");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn attach_catalog(
    Extension(claims): Extension<Claims>, State(state): State<AppState>,
    Path((id, catalog_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, String), ApiError> {
    owned_training(&claims, &state, id).await?;
    let def = catalog::owned(&claims, &state, catalog_id).await?;
    let training=owned_training(&claims,&state,id).await?;
    // Do not attach a catalog entry shared with some other team.
    if let Some(activity_team)=def.team_id {
        if training.team_id!=Some(activity_team) {return Err(StatusCode::FORBIDDEN);}
    }
    let existing = training_activity::Entity::find()
        .filter(training_activity::COLUMN.training_id.eq(id))
        .filter(training_activity::COLUMN.catalog_id.eq(catalog_id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if existing.is_some() { return Err(StatusCode::CONFLICT); }
    let count = training_activity::Entity::find()
        .filter(training_activity::COLUMN.training_id.eq(id))
        .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.len();
    let linked = training_activity::ActiveModel {
        id: Set(Uuid::new_v4()),
        training_id: Set(id),
        catalog_id: Set(Some(catalog_id)),
        name: Set(def.name),
        description: Set(def.description),
        description_html: Set(def.description_html),
        image_url: Set(def.image_url),
        sort_order: Set(count as i32),
        ..Default::default()
    }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, linked.id.to_string()))
}


#[derive(Deserialize)]
pub struct ReorderActivities {
    pub activity_ids: Vec<Uuid>,
}
pub async fn reorder_activities(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReorderActivities>,
) -> Result<StatusCode, ApiError> {
    owned_training(&claims, &state, id).await?;
    let activities = training_activity::Entity::find()
        .filter(training_activity::COLUMN.training_id.eq(id))
        .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let actual: HashSet<Uuid> = activities.iter().map(|a| a.id).collect();
    let proposed: HashSet<Uuid> = body.activity_ids.iter().copied().collect();
    if actual != proposed || body.activity_ids.len() != activities.len() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let txn = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for (index, activity_id) in body.activity_ids.iter().enumerate() {
        let item = activities.iter().find(|a| a.id == *activity_id)
            .ok_or(StatusCode::BAD_REQUEST)?;
        let mut active: training_activity::ActiveModel = item.clone().into();
        active.sort_order = Set(index as i32);
        active.update(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    txn.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tracing::info!(actor=%claims.sub, training=%id, count=body.activity_ids.len(), action="training.reorder");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_activity(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((training_id, activity_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let activity = owned_activity(&claims, &state, training_id, activity_id).await?;
    let txn = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    training_score::Entity::delete_many()
        .filter(training_score::COLUMN.activity_id.eq(activity_id))
        .exec(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    training_material::Entity::delete_many()
        .filter(training_material::COLUMN.activity_id.eq(activity_id))
        .exec(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let active: training_activity::ActiveModel = activity.into();
    active.delete(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    txn.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_material(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((training_id, activity_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateMaterial>,
) -> Result<(StatusCode, String), ApiError> {
    let activity = owned_activity(&claims, &state, training_id, activity_id).await?;
    if !valid_name(&body.name) || !(1..=100_000).contains(&body.quantity) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(catalog_id) = activity.catalog_id {
        catalog::writable(&claims,&state,catalog_id).await?;
        let material = catalog_material::ActiveModel {
            id: Set(Uuid::new_v4()),
            activity_id: Set(catalog_id),
            name: Set(body.name.trim().into()),
            quantity: Set(body.quantity),
            ..Default::default()
        }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok((StatusCode::CREATED, material.id.to_string()));
    }
    let new = training_material::ActiveModel {
        id: Set(Uuid::new_v4()),
        activity_id: Set(activity_id),
        name: Set(body.name.trim().to_string()),
        quantity: Set(body.quantity),
        ..Default::default()
    };
    let material = new.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, material.id.to_string()))
}

pub async fn remove_material(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path((training_id, activity_id, material_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let activity = owned_activity(&claims, &state, training_id, activity_id).await?;
    if let Some(catalog_id) = activity.catalog_id {
        catalog::writable(&claims,&state,catalog_id).await?;
        let item = catalog_material::Entity::find_by_id(material_id)
            .filter(catalog_material::COLUMN.activity_id.eq(catalog_id))
            .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
        let active: catalog_material::ActiveModel = item.into();
        active.delete(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(StatusCode::NO_CONTENT);
    }
    let item = training_material::Entity::find_by_id(material_id)
        .filter(training_material::COLUMN.activity_id.eq(activity_id))
        .one(&state.db).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let active: training_material::ActiveModel = item.into();
    active.delete(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_attendance(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<AttendanceUpdate>,
) -> Result<StatusCode, ApiError> {
    let training=owned_training(&claims,&state,id).await?;
    let players=if let Some(team_id)=training.team_id {
        organization::list_team_players_internal(&state,team_id).await?
    } else {
        services::player::get_players(&claims,&state).await?
    };
    let allowed: HashSet<Uuid> = players.iter().map(|p| p.id).collect();
    let present: HashSet<Uuid> = body.player_ids.into_iter().collect();

    if !present.is_subset(&allowed) {
        return Err(StatusCode::FORBIDDEN);
    }
    let txn = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for player in players {
        let current = training_attendance::Entity::find_by_id((id, player.id))
            .one(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let is_present = present.contains(&player.id);
        match current {
            Some(attendance) => {
                let mut active: training_attendance::ActiveModel = attendance.into();
                active.present = Set(is_present);
                active.update(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            None if is_present => {
                training_attendance::ActiveModel {
                    training_id: Set(id),
                    player_id: Set(player.id),
                    present: Set(true),
                    score: Set(None),
                    ..Default::default()
                }.insert(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            None => {}
        }
    }
    txn.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_score(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ScoreUpdate>,
) -> Result<StatusCode, ApiError> {
    let training=owned_training(&claims,&state,id).await?;
    if let Some(team_id)=training.team_id {
        let roster=organization::list_team_players_internal(&state,team_id).await?;
        if !roster.iter().any(|p|p.id==body.player_id) {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    if !valid_score(body.score) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let Some(attendance) = training_attendance::Entity::find_by_id((id, body.player_id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? else {
        return Err(StatusCode::BAD_REQUEST);
    };
    if !attendance.present {
        return Err(StatusCode::BAD_REQUEST);
    }

    if let Some(activity_id) = body.activity_id {
        owned_activity(&claims, &state, id, activity_id).await?;
        let current = training_score::Entity::find_by_id((activity_id, body.player_id))
            .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        match (current, body.score) {
            (Some(score), None) => {
                let active: training_score::ActiveModel = score.into();
                active.delete(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            (Some(score), Some(value)) => {
                let mut active: training_score::ActiveModel = score.into();
                active.score = Set(value);
                active.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            (None, Some(value)) => {
                training_score::ActiveModel {
                    activity_id: Set(activity_id),
                    player_id: Set(body.player_id),
                    score: Set(value),
                    ..Default::default()
                }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            (None, None) => {}
        }
    } else {
        let mut active: training_attendance::ActiveModel = attendance.into();
        active.score = Set(body.score);
        active.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::{valid_image_url, valid_name, valid_score};

    #[test]
    fn score_is_bounded_inclusively() {
        assert!(valid_score(None));
        assert!(valid_score(Some(0)));
        assert!(valid_score(Some(5)));
        assert!(!valid_score(Some(-1)));
        assert!(!valid_score(Some(6)));
    }

    #[test]
    fn rejects_empty_names_and_untrusted_image_schemes() {
        assert!(!valid_name("  "));
        assert!(valid_name("Échauffement"));
        assert!(valid_image_url(Some("https://example.org/diagramme.png")));
        assert!(!valid_image_url(Some("javascript:alert(1)")));
    }
}


#[cfg(test)]
mod integration_tests {
    use super::*;
    use axum::{Extension, Json, extract::{Path, State}};
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, Database, DatabaseConnection};
    use crate::entities::{player, player_user, user};

    async fn test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*")
            .sync(&db).await.unwrap();
        db
    }

    #[tokio::test]
    async fn a_training_can_be_planned_scored_and_is_private_to_its_owner() {
        let state = AppState { db: test_db().await };
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();
        let claims = Claims { sub: owner, exp: u64::MAX };
        let stranger = Claims { sub: other, exp: u64::MAX };
        // A real user row is required for the player_user foreign key.
        user::ActiveModel {
            id: Set(owner),
            username: Set("test-coach".into()),
            password: Set("not-used-in-this-test".into()),
            ..Default::default()
        }.insert(&state.db).await.unwrap();
        // The authenticated stranger also exists in the user table:
        // the real middleware never issues Claims for deleted users.
        user::ActiveModel {
            id: Set(other), username: Set("other-coach".into()),
            password: Set("not-used-in-this-test".into()), ..Default::default()
        }.insert(&state.db).await.unwrap();
        let date = DateTime::parse_from_str("2026-10-28 18:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let (created, id_string) = create(
            Extension(claims.clone()),
            State(state.clone()),
            Json(CreateTraining { name: "Entraînement test".into(), date, team_id: None }),
        ).await.unwrap();
        assert_eq!(created, StatusCode::CREATED);
        let id = Uuid::parse_str(&id_string).unwrap();

        // No data from another coach may be exposed by a guessed UUID.
        let forbidden = get(
            Extension(stranger.clone()), State(state.clone()), Path(id),
        ).await;
        assert!(matches!(forbidden, Err(StatusCode::NOT_FOUND)));
        let other_list = list(Extension(stranger), State(state.clone())).await.unwrap().0;
        assert!(other_list.is_empty());

        let player_id = Uuid::new_v4();
        player::ActiveModel {
            id: Set(player_id),
            firstname: Set("Camille".into()),
            lastname: Set("Exemple".into()),
            number: Set(8),
            ..Default::default()
        }.insert(&state.db).await.unwrap();
        player_user::ActiveModel {
            user_id: Set(owner),
            player_id: Set(player_id),
            ..Default::default()
        }.insert(&state.db).await.unwrap();

        let (_, activity_id) = add_activity(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(CreateActivity {
                name: "Passes".into(),
                description: "En binôme, pendant 10 minutes.".into(),
                image_url: None,
            })
        ).await.unwrap();
        let activity_id = Uuid::parse_str(&activity_id).unwrap();
        let original_catalog_id=training_activity::Entity::find_by_id(activity_id)
            .one(&state.db).await.unwrap().unwrap().catalog_id.unwrap();
        let refused=update_activity(
            Extension(Claims{sub:other,exp:u64::MAX}),State(state.clone()),
            Path((id,activity_id)),
            Json(EditSessionActivity{
                name:"Tentative".into(),description:"".into(),description_html:None,image_url:None
            })
        ).await;
        assert!(matches!(refused,Err(StatusCode::NOT_FOUND)));
        update_activity(
            Extension(claims.clone()),State(state.clone()),Path((id,activity_id)),
            Json(EditSessionActivity{
                name:"Passes corrigées".into(),
                description:"Description corrigée".into(),
                description_html:Some("<p><strong>Consigne</strong></p>".into()),
                image_url:None,
            })
        ).await.unwrap();
        let modified=training_activity::Entity::find_by_id(activity_id)
            .one(&state.db).await.unwrap().unwrap();
        assert_eq!(modified.name,"Passes corrigées");
        assert_eq!(modified.description,"Description corrigée");
        let original=catalog_activity::Entity::find_by_id(original_catalog_id)
            .one(&state.db).await.unwrap().unwrap();
        assert_eq!(original.name,"Passes");
        // Restore the label used by the remaining existing assertions.
        update_activity(
            Extension(claims.clone()),State(state.clone()),Path((id,activity_id)),
            Json(EditSessionActivity{
                name:"Passes".into(),
                description:"En binôme, pendant 10 minutes.".into(),
                description_html:None,image_url:None,
            })
        ).await.unwrap();

        add_material(
            Extension(claims.clone()), State(state.clone()), Path((id, activity_id)),
            Json(CreateMaterial { name: "Ballons".into(), quantity: 5 }),
        ).await.unwrap();

        set_attendance(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(AttendanceUpdate { player_ids: vec![player_id] }),
        ).await.unwrap();

        set_score(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(ScoreUpdate { player_id, activity_id: None, score: Some(5) }),
        ).await.unwrap();

        set_score(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(ScoreUpdate { player_id, activity_id: Some(activity_id), score: Some(3) }),
        ).await.unwrap();

        let detail = get(Extension(claims.clone()), State(state.clone()), Path(id))
            .await.unwrap().0;
        assert_eq!(detail.activities.len(), 1);
        assert_eq!(detail.activities[0].name, "Passes");
        assert_eq!(detail.materials[0].quantity, 5);
        assert_eq!(detail.attendance[0].score, Some(5));
        assert!(detail.attendance[0].present);
        assert_eq!(detail.scores[0].score, 3);

        // Subsequent updates must persist as well, not just initial inserts.
        set_score(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(ScoreUpdate { player_id, activity_id: None, score: Some(2) }),
        ).await.unwrap();
        set_score(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(ScoreUpdate { player_id, activity_id: Some(activity_id), score: Some(4) }),
        ).await.unwrap();
        set_attendance(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(AttendanceUpdate { player_ids: vec![] }),
        ).await.unwrap();
        let updated = get(Extension(claims.clone()), State(state.clone()), Path(id)).await.unwrap().0;
        assert_eq!(updated.attendance[0].score, Some(2));
        assert!(!updated.attendance[0].present);
        assert_eq!(updated.scores[0].score, 4);

        set_attendance(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(AttendanceUpdate { player_ids: vec![player_id] }),
        ).await.unwrap();
        update(
            Extension(claims.clone()), State(state.clone()), Path(id),
            Json(UpdateTraining {
                name: "Entraînement corrigé".into(),
                date: DateTime::parse_from_str("2026-10-28 19:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
                notes: "Consignes générales".into(),
            }),
        ).await.unwrap();
        let saved = get(Extension(claims.clone()), State(state.clone()), Path(id)).await.unwrap().0;
        assert!(saved.attendance[0].present);
        assert_eq!(saved.training.name, "Entraînement corrigé");
        assert_eq!(saved.training.notes, "Consignes générales");

        let invalid = set_score(
            Extension(claims), State(state), Path(id),
            Json(ScoreUpdate { player_id, activity_id: None, score: Some(6) }),
        ).await;
        assert!(matches!(invalid, Err(StatusCode::BAD_REQUEST)));
    }
}


#[cfg(test)]
mod reorder_tests {
    use super::*;
    use crate::entities::user;
    use sea_orm::Database;
    #[tokio::test]
    async fn activity_order_is_persisted_and_must_be_a_permutation() {
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        let uid=Uuid::new_v4();
        user::ActiveModel{id:Set(uid),username:Set("trainer".into()),password:Set("test".into()),
            ..Default::default()}.insert(&state.db).await.unwrap();
        let claims=Claims{sub:uid,exp:u64::MAX};
        let date=DateTime::parse_from_str("2026-10-29 18:00:00","%Y-%m-%d %H:%M:%S").unwrap();
        let (_,id)=create(Extension(claims.clone()),State(state.clone()),
            Json(CreateTraining{name:"Training".into(),date,team_id:None})).await.unwrap();
        let tid=Uuid::parse_str(&id).unwrap();
        for name in ["Échauffement","Passes","Match"] {
            add_activity(Extension(claims.clone()),State(state.clone()),Path(tid),
                Json(CreateActivity{name:name.into(),description:String::new(),image_url:None}))
            .await.unwrap();
        }
        let initial=get(Extension(claims.clone()),State(state.clone()),Path(tid)).await.unwrap().0;
        let mut order:Vec<Uuid>=initial.activities.iter().map(|a|a.id).collect();
        order.reverse();
        reorder_activities(Extension(claims.clone()),State(state.clone()),Path(tid),
            Json(ReorderActivities{activity_ids:order.clone()})).await.unwrap();
        let updated=get(Extension(claims.clone()),State(state.clone()),Path(tid)).await.unwrap().0;
        assert_eq!(updated.activities.iter().map(|a|a.id).collect::<Vec<_>>(),order);
        let invalid=reorder_activities(Extension(claims),State(state),Path(tid),
            Json(ReorderActivities{activity_ids:vec![order[0],order[0],order[1]]})).await;
        assert!(matches!(invalid,Err(StatusCode::BAD_REQUEST)));
    }
}


#[cfg(test)]
mod team_scope_regression {
    use super::*;
    use crate::entities::{user,club,team,team_membership};
    use sea_orm::Database;
    #[tokio::test]
    async fn assigned_team_is_shared_but_unauthorized_teams_remain_private() {
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        let creator=Uuid::new_v4();
        let teammate=Uuid::new_v4();
        let outsider=Uuid::new_v4();
        for (id,name,role) in [(creator,"coach1","trainer"),(teammate,"coach2","user"),(outsider,"other","trainer")] {
            user::ActiveModel{
                id:Set(id),username:Set(name.into()),password:Set("test".into()),
                role:Set(Some(role.into())),..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        let club_id=Uuid::new_v4();
        club::ActiveModel{id:Set(club_id),name:Set("Club".into()),
            is_public:Set(Some(false)),..Default::default()}
            .insert(&state.db).await.unwrap();
        let team_id=Uuid::new_v4();
        team::ActiveModel{id:Set(team_id),club_id:Set(club_id),
            name:Set("Équipe".into()),..Default::default()}
            .insert(&state.db).await.unwrap();
        for (id,role) in [(creator,"trainer"),(teammate,"user")] {
            team_membership::ActiveModel{user_id:Set(id),team_id:Set(team_id),
                role:Set(role.into()),..Default::default()}
                .insert(&state.db).await.unwrap();
        }
        let now=DateTime::parse_from_str("2026-10-29 18:00:00","%Y-%m-%d %H:%M:%S").unwrap();
        let claims=Claims{sub:creator,exp:u64::MAX};
        let (_,created_id)=create(Extension(claims.clone()),State(state.clone()),
            Json(CreateTraining{name:"Séance partagée".into(),date:now,team_id:Some(team_id)}))
            .await.unwrap();
        let id=Uuid::parse_str(&created_id).unwrap();
        let reader=Claims{sub:teammate,exp:u64::MAX};
        assert!(get(Extension(reader.clone()),State(state.clone()),Path(id)).await.is_ok());
        let their_sessions=list(Extension(reader.clone()),State(state.clone())).await.unwrap().0;
        assert_eq!(their_sessions.len(),1);
        let forbidden=update(Extension(reader),State(state.clone()),Path(id),
            Json(UpdateTraining{name:"Interdit".into(),date:now,notes:String::new()})).await;
        assert!(matches!(forbidden,Err(StatusCode::NOT_FOUND)));
        let stranger=Claims{sub:outsider,exp:u64::MAX};
        assert!(matches!(
            get(Extension(stranger.clone()),State(state.clone()),Path(id)).await,
            Err(StatusCode::NOT_FOUND)
        ));
        assert!(list(Extension(stranger),State(state)).await.unwrap().0.is_empty());
    }
}
