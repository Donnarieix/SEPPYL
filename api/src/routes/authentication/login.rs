use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, extract::State, http::StatusCode};
use jsonwebtoken::{EncodingKey, Header};
use sea_orm::{EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{entities, state::AppState};

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: u64,
}

#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "SECRET".to_string());

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
        sub: user.id,
        // Local provider session persists across refresh for up to 24 hours.
        // Replace with the Donnarieix SSO provider and refresh flow later.
        exp: now + 24 * 3600,
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    Ok(Json(LoginResponse { token }))
}
