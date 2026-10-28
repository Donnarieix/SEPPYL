use argon2::{Argon2, PasswordHasher};
use axum::{Json, extract::State, http::StatusCode};
use sea_orm::{ActiveModelTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{entities, state::AppState};

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration successful"),
        (status = 400, description = "Username or password is too short"),
        (status = 409, description = "Username already exists"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(), StatusCode> {
    let username = payload.username.trim().to_string();
    if username.len() < 3 || username.len() > 80 || payload.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let existing = entities::user::Entity::find()
        .filter(entities::user::COLUMN.username.eq(username.clone()))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if existing.is_some() {
        return Err(StatusCode::CONFLICT);
    }

    let hash = Argon2::default()
        .hash_password(payload.password.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = entities::user::ActiveModel {
        id: sea_orm::ActiveValue::Set(Uuid::new_v4()),
        username: sea_orm::ActiveValue::Set(username),
        password: sea_orm::ActiveValue::Set(hash.to_string()),
        ..Default::default()
    };
    user.insert(&state.db).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::authentication::login::{login, LoginRequest};
    use sea_orm::Database;

    #[tokio::test]
    async fn a_registered_coach_can_log_in() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state = AppState { db };

        register(
            State(state.clone()),
            Json(RegisterRequest {
                username: "valentin-test".into(),
                password: "test-password-strong".into(),
            }),
        ).await.unwrap();

        let response = login(
            State(state.clone()),
            Json(LoginRequest {
                username: "valentin-test".into(),
                password: "test-password-strong".into(),
            }),
        ).await.unwrap();
        assert!(!response.0.token.is_empty());

        let duplicate = register(
            State(state),
            Json(RegisterRequest {
                username: "valentin-test".into(),
                password: "another-password".into(),
            }),
        ).await;
        assert!(matches!(duplicate, Err(StatusCode::CONFLICT)));
    }
}
