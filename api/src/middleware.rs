use axum::{
    extract::{Request, State},
    http::{Method, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation};
use sea_orm::EntityTrait;

use crate::{entities::user, routes::authentication::login::Claims, state::AppState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role { Visitor, User, Trainer, Manager, Administrator }
impl Role {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "visitor" => Some(Self::Visitor),
            "user" => Some(Self::User),
            "trainer" => Some(Self::Trainer),
            "manager" => Some(Self::Manager),
            "administrator" => Some(Self::Administrator),
            _ => None
        }
    }
    pub fn can_write(self) -> bool {
        matches!(self, Self::Trainer | Self::Manager | Self::Administrator)
    }
}

#[derive(Clone, Copy)]
pub struct CurrentRole(pub Role);

pub async fn auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "SECRET".to_string());
    let token = request.headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let data = jsonwebtoken::decode::<Claims>(
        token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default()
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Always check the live account role rather than trusting a stale JWT.
    let current = user::Entity::find_by_id(data.claims.sub)
        .one(&state.db).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    // Pre-existing, role-less users keep their previous write access.
    let role = current.role.as_deref().unwrap_or("trainer");
    let role = Role::parse(role).ok_or(StatusCode::FORBIDDEN)?;
    if matches!(role, Role::Visitor) { return Err(StatusCode::UNAUTHORIZED); }

    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let mutation = matches!(method, Method::POST | Method::PUT | Method::PATCH | Method::DELETE);
    let self_service = path.starts_with("/account/") || path == "/preferences";
    if mutation && !self_service && !role.can_write() {
        tracing::warn!(actor=%data.claims.sub, %path, "role denied mutation");
        return Err(StatusCode::FORBIDDEN);
    }

    let actor = data.claims.sub;
    request.extensions_mut().insert(data.claims);
    request.extensions_mut().insert(CurrentRole(role));
    let response = next.run(request).await;
    if mutation {
        let action = match method {
            Method::POST => "create", Method::PUT | Method::PATCH => "update",
            Method::DELETE => "delete", _ => "other",
        };
        tracing::info!(%actor, %path, action, status=%response.status(), "mutation");
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roles_and_writes() {
        assert!(!Role::User.can_write());
        assert!(!Role::Visitor.can_write());
        assert!(Role::Trainer.can_write());
        assert!(Role::Manager.can_write());
        assert!(Role::Administrator.can_write());
        assert!(Role::parse("administrator").is_some());
        assert!(Role::parse("superuser").is_none());
    }
}
