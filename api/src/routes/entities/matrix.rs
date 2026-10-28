use axum::{
    Extension, Json,
    extract::State,
    http::StatusCode,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities,
    routes::authentication::login::Claims,
    services,
    state::AppState,
};

#[derive(Serialize)]
pub struct MatrixResult {
    pub activity_id: Uuid,
    pub player_id: Uuid,
    pub result: Option<String>,
}

#[derive(Serialize)]
pub struct MatrixResponse {
    pub players: Vec<entities::player::Model>,
    pub activities: Vec<entities::activity::Model>,
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

    let activities = entities::activity::Entity::find()
        .filter(entities::activity::COLUMN.id.is_in(
            links.iter().map(|link| link.activity_id),
        ))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let results = links.into_iter().map(|link| MatrixResult {
        activity_id: link.activity_id,
        player_id: link.player_id,
        result: link.result,
    }).collect();

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

    if !services::activity::has_rights(&claims, &state, payload.activity_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let current = entities::activity_player::Entity::find()
        .filter(entities::activity_player::COLUMN.player_id.eq(payload.player_id))
        .filter(entities::activity_player::COLUMN.activity_id.eq(payload.activity_id))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(mut link) = current {
        link.result = payload.result;
        let active: entities::activity_player::ActiveModel = link.into();
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
