use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter, TransactionTrait};
use sea_orm::entity::prelude::DateTime;
use utoipa::ToSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities,
    routes::authentication::login::Claims,
    services,
    routes::entities::catalog,
    state::AppState,
};

#[derive(Serialize)]
pub struct MatrixResult {
    pub activity_id: Uuid,
    pub player_id: Uuid,
    pub result: Option<String>,
}

#[derive(Serialize)]
pub struct MatrixActivity {
    pub id: Uuid,
    pub name: String,
    pub date: String,
}

#[derive(Serialize)]
pub struct MatrixResponse {
    pub players: Vec<entities::player::Model>,
    pub activities: Vec<MatrixActivity>,
    pub results: Vec<MatrixResult>,
}

/// A single authenticated snapshot for the spreadsheet.
/// Only returns players owned by the caller and activities linked to them.
pub async fn get_matrix(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<MatrixResponse>, StatusCode> {
    let players = services::player::get_players(&claims, &state).await?;

    if players.is_empty() {
        return Ok(Json(MatrixResponse {
            players,
            activities: Vec::new(),
            results: Vec::new(),
        }));
    }

    let links = entities::activity_player::Entity::find()
        .filter(entities::activity_player::COLUMN.player_id.is_in(
            players.iter().map(|player| player.id),
        ))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let legacy = entities::activity::Entity::find()
        .filter(entities::activity::COLUMN.id.is_in(
            links.iter().map(|link| link.activity_id),
        ))
        .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let selection = entities::catalog_selection::Entity::find()
        .filter(entities::catalog_selection::COLUMN.user_id.eq(claims.sub))
        .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let selected_ids: Vec<Uuid> = selection.iter().map(|s| s.activity_id).collect();
    let catalog_activities = if selected_ids.is_empty() { Vec::new() } else {
        entities::catalog_activity::Entity::find()
            .filter(entities::catalog_activity::COLUMN.user_id.eq(claims.sub))
            .filter(entities::catalog_activity::COLUMN.id.is_in(selected_ids))
            .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    let mut activities: Vec<MatrixActivity> = legacy.into_iter().map(|activity| MatrixActivity {
        id: activity.id, name: activity.name, date: activity.date.to_string(),
    }).collect();
    activities.extend(catalog_activities.iter().map(|activity| MatrixActivity {
        id: activity.id, name: activity.name.clone(), date: String::new(),
    }));

    let selected_results = if catalog_activities.is_empty() { Vec::new() } else {
        entities::catalog_result::Entity::find()
            .filter(entities::catalog_result::COLUMN.user_id.eq(claims.sub))
            .filter(entities::catalog_result::COLUMN.activity_id.is_in(catalog_activities.iter().map(|a| a.id)))
            .filter(entities::catalog_result::COLUMN.player_id.is_in(players.iter().map(|p| p.id)))
            .all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };
    let mut results: Vec<MatrixResult> = links.into_iter().map(|link| MatrixResult {
        activity_id: link.activity_id,
        player_id: link.player_id,
        result: link.result,
    }).collect();
    results.extend(selected_results.into_iter().map(|r| MatrixResult {
        activity_id: r.activity_id, player_id: r.player_id, result: r.result,
    }));

    Ok(Json(MatrixResponse {
        players,
        activities,
        results,
    }))
}

#[derive(Deserialize)]
pub struct UpdateMatrixResultRequest {
    pub activity_id: Uuid,
    pub player_id: Uuid,
    pub result: Option<String>,
}

/// Edit a cell, creating the player/activity association if necessary.
/// A caller must own the player and have access to the activity already.
pub async fn update_matrix_result(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateMatrixResultRequest>,
) -> Result<StatusCode, StatusCode> {
    let players = services::player::get_players(&claims, &state).await?;
    if !players.iter().any(|player| player.id == payload.player_id) {
        return Err(StatusCode::FORBIDDEN);
    }

    let catalog = entities::catalog_activity::Entity::find_by_id(payload.activity_id)
        .filter(entities::catalog_activity::COLUMN.user_id.eq(claims.sub))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if catalog.is_some() {
        let selected = entities::catalog_selection::Entity::find_by_id((claims.sub, payload.activity_id))
            .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if selected.is_none() { return Err(StatusCode::FORBIDDEN); }
        let existing = entities::catalog_result::Entity::find_by_id((claims.sub, payload.activity_id, payload.player_id))
            .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(value) = existing {
            let mut active: entities::catalog_result::ActiveModel = value.into();
            active.result = Set(payload.result);
            active.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        } else {
            entities::catalog_result::ActiveModel {
                user_id: Set(claims.sub),
                activity_id: Set(payload.activity_id),
                player_id: Set(payload.player_id),
                result: Set(payload.result),
                ..Default::default()
            }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        return Ok(StatusCode::NO_CONTENT);
    }

    if !services::activity::has_rights(&claims, &state, payload.activity_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let current = entities::activity_player::Entity::find()
        .filter(entities::activity_player::COLUMN.player_id.eq(payload.player_id))
        .filter(entities::activity_player::COLUMN.activity_id.eq(payload.activity_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(link) = current {
        let mut active: entities::activity_player::ActiveModel = link.into();
        active.result = Set(payload.result);
        active.update(&state.db).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        let active = entities::activity_player::ActiveModel {
            player_id: Set(payload.player_id),
            activity_id: Set(payload.activity_id),
            result: Set(payload.result),
            ..Default::default()
        };
        active.insert(&state.db).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::NO_CONTENT)
}


#[derive(Deserialize, ToSchema)]
pub struct CreateMatrixActivityRequest {
    pub name: String,
    pub date: DateTime,
}

/// A spreadsheet activity belongs to all of the current user's players.
/// The initial results are deliberately empty and can be filled in the grid.
#[utoipa::path(
    post,
    path = "/matrix/activity",
    request_body = CreateMatrixActivityRequest,
    responses(
        (status = 201, description = "Activity created for all players"),
        (status = 400, description = "A name, date and at least one player are required"),
        (status = 409, description = "An activity with this name already exists"),
        (status = 500, description = "Database error")
    )
)]
pub async fn create_matrix_activity(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<CreateMatrixActivityRequest>,
) -> Result<(StatusCode, String), StatusCode> {
    let _date = payload.date; // catalog definitions are reusable, not tied to a date.
    let activity = catalog::insert(&claims, &state, catalog::ActivityPayload {
        name: payload.name,
        description: String::new(),
        image_url: None,
    }).await?;
    entities::catalog_selection::ActiveModel {
        user_id: Set(claims.sub),
        activity_id: Set(activity.id),
        ..Default::default()
    }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, activity.id.to_string()))
}

#[derive(Deserialize)]
pub struct SelectCatalogActivity { pub activity_id: Uuid }

/// Add or remove an already-created activity from the tableur columns.
pub async fn select_activity(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<SelectCatalogActivity>,
) -> Result<StatusCode, StatusCode> {
    catalog::owned(&claims, &state, payload.activity_id).await?;
    let exists = entities::catalog_selection::Entity::find_by_id((claims.sub, payload.activity_id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if exists.is_none() {
        entities::catalog_selection::ActiveModel {
            user_id: Set(claims.sub),
            activity_id: Set(payload.activity_id),
            ..Default::default()
        }.insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(StatusCode::NO_CONTENT)
}
