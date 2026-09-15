use argon2::{Argon2, PasswordHasher};
use axum::{Json, extract::State, http::StatusCode};
use sea_orm::{ActiveModelTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::{entities, state::AppState};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(), StatusCode> {
    let existing_user = entities::user::Entity::find()
        .filter(entities::user::COLUMN.username.eq(payload.username.clone()))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if existing_user.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let hash = Argon2::default()
        .hash_password(payload.password.clone().as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = entities::user::ActiveModel {
        username: sea_orm::ActiveValue::Set(payload.username),
        password: sea_orm::ActiveValue::Set(hash.to_string()),
        ..Default::default()
    };

    user.insert(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}
