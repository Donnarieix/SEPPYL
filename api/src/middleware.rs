use axum::{
    extract::{Request, State},
    http::{Method, StatusCode, header::{AUTHORIZATION,CONTENT_LENGTH,CONTENT_TYPE}},
    body::{Body,to_bytes},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation};
use sea_orm::EntityTrait;

use crate::{audit,entities::user, routes::authentication::login::Claims, state::AppState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role { Visitor, User, Trainer, Manager, Administrator, Maintainer }
impl Role {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "visitor" => Some(Self::Visitor),
            "user" => Some(Self::User),
            "trainer" => Some(Self::Trainer),
            "manager" => Some(Self::Manager),
            "administrator" => Some(Self::Administrator),
            "maintainer" => Some(Self::Maintainer),
            _ => None
        }
    }
    pub fn is_admin(self)->bool{
        matches!(self,Self::Administrator|Self::Maintainer)
    }
    pub fn can_write(self) -> bool {
        matches!(self, Self::Trainer | Self::Manager | Self::Administrator | Self::Maintainer)
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
    let self_service = path.starts_with("/account/") || path == "/preferences"
        || path.starts_with("/clubs") || path.starts_with("/teams")
        || path.starts_with("/admin/")
        // Read receipts belong to the authenticated account, even when that
        // account has no mutation permissions (Utilisateur / lecture seule).
        || (method == Method::POST && path.starts_with("/changelog/")
            && path.ends_with("/read"));
    if mutation && !self_service && !role.can_write() {
        tracing::warn!(actor=%data.claims.sub, %path, "role denied mutation");
        return Err(StatusCode::FORBIDDEN);
    }

    let actor = data.claims.sub;
    // Only inspect small JSON creations that do not expose their team ID in
    // the URL. Restore the exact body for Axum's real handler afterwards.
    let mut payload = None;
    if mutation && path != "/catalog/upload"
        && request.headers().get(CONTENT_TYPE)
            .and_then(|v|v.to_str().ok()).is_some_and(|v|v.starts_with("application/json")) {
        let length = request.headers().get(CONTENT_LENGTH)
            .and_then(|v|v.to_str().ok()).and_then(|v|v.parse::<usize>().ok());
        // Browser fetch typically omits Content-Length, especially over HTTP/2.
        // Read bounded JSON even when the header is absent.
        if length.is_none_or(|n|n <= 512 * 1024) {
            let original = std::mem::replace(request.body_mut(), Body::empty());
            let bytes = to_bytes(original, 512 * 1024).await
                .map_err(|_|StatusCode::PAYLOAD_TOO_LARGE)?;
            payload = serde_json::from_slice(&bytes).ok();
            *request.body_mut() = Body::from(bytes);
        }
    }
    let context = if mutation {
        Some(audit::capture(&state,actor,&method,&path,payload.as_ref()).await)
    }else{None};
    request.extensions_mut().insert(data.claims);
    request.extensions_mut().insert(CurrentRole(role));
    let response = next.run(request).await;
    if let Some(context)=context{
        tracing::info!(%actor,%path,status=%response.status(),"mutation");
        audit::record(&state,actor,&method,&path,response.status(),context).await;
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
        assert!(Role::Maintainer.can_write());
        assert!(Role::Maintainer.is_admin());
        assert!(Role::Administrator.is_admin());
        assert!(Role::parse("administrator").is_some());
        assert!(Role::parse("superuser").is_none());
    }
}
