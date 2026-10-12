use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{entities, routes::authentication::login::Claims, services, state::AppState};

#[utoipa::path(
    get,
    path = "/player/list",
    responses(
        (status = 200, description = "List of players retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_players(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<entities::player::Model>>, StatusCode> {
    let players = services::player::get_players(&claims, &state).await?;

    Ok(Json(players))
}

#[utoipa::path(
    get,
    path = "/player/get/{id}",
    responses(
        (status = 200, description = "Player retrieved successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Player not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_player(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<entities::player::Model>, StatusCode> {
    if let Some(player) = services::player::get_player(&claims, &state, id).await? {
        Ok(Json(player))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePlayerRequest {
    pub firstname: String,
    pub lastname: String,
    pub number: u32,
}

#[utoipa::path(
    post,
    path = "/player/create",
    request_body = CreatePlayerRequest,
    responses(
        (status = 201, description = "Player created successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_player(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<CreatePlayerRequest>,
) -> Result<(StatusCode, String), StatusCode> {
    match services::player::create_player(&claims, &state, payload).await {
        Ok(player_id) => Ok((StatusCode::CREATED, player_id.to_string())),
        Err(status) => Err(status),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePlayerRequest {
    pub id: Uuid,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub number: Option<u32>,
}

#[utoipa::path(
    put,
    path = "/player/update",
    request_body = UpdatePlayerRequest,
    responses(
        (status = 200, description = "Player updated successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Player not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_player(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<UpdatePlayerRequest>,
) -> Result<(StatusCode, String), StatusCode> {
    match services::player::update_player(&claims, &state, payload).await {
        Ok(player_id) => Ok((StatusCode::OK, player_id.to_string())),
        Err(status) => Err(status),
    }
}

#[utoipa::path(
    delete,
    path = "/player/delete/{id}",
    responses(
        (status = 204, description = "Player deleted successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Player not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_player(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    match services::player::delete_player(&claims, &state, id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(status) => Err(status),
    }
}