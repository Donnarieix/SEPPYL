use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use sea_orm::entity::prelude::DateTime;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{entities, routes::authentication::login::Claims, services, state::AppState};

#[utoipa::path(
    get,
    path = "/activity/list",
    responses(
        (status = 200, description = "Get all activities", body = [entities::activity::Model])
    )
)]
pub async fn get_activities(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<entities::activity::Model>>, StatusCode> {
    let activities = services::activity::get_activities(&claims, &state).await?;

    Ok(Json(activities))
}

#[utoipa::path(
    get,
    path = "/activity/get/{id}",
    responses(
        (status = 200, description = "Get activity by ID", body = entities::activity::Model),
        (status = 404, description = "Activity not found")
    )
)]
pub async fn get_activity(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<entities::activity::Model>, StatusCode> {
    if let Some(activity) = services::activity::get_activity(&claims, &state, id).await? {
        Ok(Json(activity))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateActivityRequest {
    pub name: String,
    pub date: DateTime,
    pub player_id: Uuid,
    pub result: Option<String>,
}

#[utoipa::path(
    post,
    path = "/activity/create",
    request_body = CreateActivityRequest,
    responses(
        (status = 201, description = "Activity created successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_activity(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<CreateActivityRequest>,
) -> Result<(StatusCode, String), StatusCode> {
    match services::activity::create_activity(&claims, &state, payload).await {
        Ok(activity_id) => Ok((StatusCode::CREATED, activity_id.to_string())),
        Err(status) => Err(status),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateActivityRequest {
    pub id: Uuid,
    pub name: Option<String>,
    pub date: Option<DateTime>,
}

#[utoipa::path(
    put,
    path = "/activity/update",
    request_body = UpdateActivityRequest,
    responses(
        (status = 200, description = "Activity updated successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Activity not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_activity(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateActivityRequest>,
) -> Result<(StatusCode, String), StatusCode> {
    match services::activity::update_activity(&claims, &state, payload).await {
        Ok(activity_id) => Ok((StatusCode::OK, activity_id.to_string())),
        Err(status) => Err(status),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePlayerActivityRequest {
    pub activity_id: Uuid,
    pub player_id: Uuid,
    pub result: Option<String>,
}

#[utoipa::path(
    put,
    path = "/activity_player/update",
    request_body = UpdatePlayerActivityRequest,
    responses(
        (status = 200, description = "Activity player updated successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Activity or player not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_activity_player(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<UpdatePlayerActivityRequest>,
) -> Result<(StatusCode, ()), StatusCode> {
    match services::activity::update_activity_player(&claims, &state, payload).await {
        Ok(()) => Ok((StatusCode::OK, ())),
        Err(status) => Err(status),
    }
}

#[utoipa::path(
    delete,
    path = "/activity/delete/{id}",
    responses(
        (status = 204, description = "Activity deleted successfully"),
        (status = 400, description = "Bad request"),
        (status = 404, description = "Activity not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_activity(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    match services::activity::delete_activity(&claims, &state, id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(status) => Err(status),
    }
}