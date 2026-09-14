use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, extract::State, http::StatusCode};
use jsonwebtoken::{EncodingKey, Header};
use sea_orm::{EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{entities, state::AppState};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user = entities::user::Entity::find()
        .filter(entities::user::COLUMN.username.eq(payload.username))
        .one(&state.db)
        .await
        .unwrap();

    if user.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let user = user.unwrap();

    let password_hash = PasswordHash::new(&user.password).unwrap();
    if !Argon2::default()
        .verify_password(payload.password.as_bytes(), &password_hash)
        .is_ok()
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let now = jsonwebtoken::get_current_timestamp();

    let claims = Claims {
        sub: user.username.clone(),
        exp: now + 3600,
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(b"SECRET"),
    )
    .unwrap();

    Ok(Json(LoginResponse { token }))
}
