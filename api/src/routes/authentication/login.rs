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
        .map_err(|error| {
            tracing::error!(%error, "Unable to query account for login");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let password_hash = PasswordHash::new(&user.password)
        .map_err(|error| {
            tracing::error!(%error, "Invalid stored password hash");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
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
        exp: now + 7 * 24 * 3600,
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|error| {
        tracing::error!(%error, "Unable to sign login token");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(LoginResponse { token }))
}
#[cfg(test)]
mod regression_tests {
    use super::*;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, Database};

    #[tokio::test]
    async fn malformed_password_hash_is_rejected_without_panicking() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        entities::user::ActiveModel {
            id: Set(Uuid::new_v4()),
            username: Set("corrupted-test-user".into()),
            password: Set("not-a-phc-hash".into()),
            role: Set(Some("user".into())),
            ..Default::default()
        }.insert(&db).await.unwrap();
        let response = login(
            State(AppState { db }),
            Json(LoginRequest {
                username: "corrupted-test-user".into(),
                password: "irrelevant".into(),
            })
        ).await;
        assert!(matches!(response, Err(StatusCode::INTERNAL_SERVER_ERROR)));
    }
}
