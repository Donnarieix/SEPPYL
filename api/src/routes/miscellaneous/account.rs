//! Local account settings. This boundary will be replaced/disabled when
//! Donnarieix SSO becomes the identity provider.
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{Extension, Json, extract::State, http::StatusCode};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{entities::user, routes::authentication::login::Claims, state::AppState};

#[derive(Serialize)]
pub struct AccountInfo {
    pub id: String,
    pub username: String,
    pub provider: &'static str,
    pub role: String,
}
#[derive(Deserialize)]
pub struct UpdateProfile { pub username: String }
#[derive(Deserialize)]
pub struct ChangePassword { pub current_password: String, pub new_password: String }

async fn owned(claims: &Claims, state: &AppState) -> Result<user::Model, StatusCode> {
    user::Entity::find_by_id(claims.sub)
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)
}

pub async fn get(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<AccountInfo>, StatusCode> {
    let user = owned(&claims, &state).await?;
    Ok(Json(AccountInfo {
        id: user.id.to_string(),
        username: user.username,
        provider: "local",
        role: user.role.unwrap_or_else(|| "trainer".into()),
    }))
}

pub async fn update_profile(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateProfile>,
) -> Result<Json<AccountInfo>, StatusCode> {
    let username = payload.username.trim();
    if username.len() < 3 || username.len() > 80 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let user = owned(&claims, &state).await?;
    if user.username != username {
        let duplicate = user::Entity::find()
            .filter(user::COLUMN.username.eq(username))
            .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if duplicate.is_some() { return Err(StatusCode::CONFLICT); }
        let mut active: user::ActiveModel = user.clone().into();
        active.username = Set(username.to_owned());
        active.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(Json(AccountInfo { id: user.id.to_string(), username: username.into(), provider: "local", role: user.role.unwrap_or_else(|| "trainer".into()) }))
}

pub async fn change_password(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<ChangePassword>,
) -> Result<StatusCode, StatusCode> {
    if payload.new_password.len() < 12 || payload.new_password.len() > 1024
        || payload.current_password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let user = owned(&claims, &state).await?;
    let old_hash = PasswordHash::new(&user.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Argon2::default().verify_password(payload.current_password.as_bytes(), &old_hash)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    if payload.current_password == payload.new_password {
        return Err(StatusCode::BAD_REQUEST);
    }
    let new_hash = Argon2::default()
        .hash_password(payload.new_password.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut active: user::ActiveModel = user.into();
    active.password = Set(new_hash.to_string());
    active.update(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::authentication::{login::{login, LoginRequest}, register::{register, RegisterRequest}};
    use sea_orm::Database;

    #[tokio::test]
    async fn edit_profile_and_change_password_requires_old_password() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state = AppState { db };
        register(State(state.clone()), Json(RegisterRequest {
            username: "trainer".into(), password: "initial-password-123".into()
        })).await.unwrap();
        let saved = user::Entity::find().one(&state.db).await.unwrap().unwrap();
        let claims = Claims { sub: saved.id, exp: u64::MAX };
        update_profile(Extension(claims.clone()), State(state.clone()),
            Json(UpdateProfile { username: "coach-new".into() })).await.unwrap();
        let bad = change_password(Extension(claims.clone()), State(state.clone()),
            Json(ChangePassword { current_password: "wrong".into(), new_password: "new-password-456".into() })).await;
        assert!(matches!(bad, Err(StatusCode::FORBIDDEN)));
        change_password(Extension(claims), State(state.clone()),
            Json(ChangePassword { current_password: "initial-password-123".into(),
                                  new_password: "new-password-456".into() })).await.unwrap();
        assert!(login(State(state.clone()), Json(LoginRequest {
            username: "coach-new".into(), password: "initial-password-123".into()
        })).await.is_err());
        assert!(login(State(state), Json(LoginRequest {
            username: "coach-new".into(), password: "new-password-456".into()
        })).await.is_ok());
    }
}
