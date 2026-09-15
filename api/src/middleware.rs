use axum::{
    extract::Request,
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation};

use crate::routes::login::Claims;

pub async fn auth(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "SECRET".to_string());

    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}
