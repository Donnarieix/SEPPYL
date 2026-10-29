use axum::{extract::{Request,State},http::{StatusCode,Method,header::AUTHORIZATION},middleware::Next,response::Response};
use jsonwebtoken::{DecodingKey,Validation};
use crate::{routes::authentication::login::Claims,state::AppState,access};

pub async fn auth(
    State(state):State<AppState>,
    mut request:Request,
    next:Next,
)->Result<Response,StatusCode>{
    let secret=std::env::var("JWT_SECRET").map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if secret.len()<32{return Err(StatusCode::INTERNAL_SERVER_ERROR)}
    let token=request.headers().get(AUTHORIZATION).and_then(|v|v.to_str().ok())
        .and_then(|v|v.strip_prefix("Bearer ")).ok_or(StatusCode::UNAUTHORIZED)?;
    let claims=jsonwebtoken::decode::<Claims>(
        token,&DecodingKey::from_secret(secret.as_bytes()),&Validation::default()
    ).map_err(|_|StatusCode::UNAUTHORIZED)?.claims;
    let method=request.method().clone();
    let path=request.uri().path();
    // Account/profile/preferences writes are allowed for each authenticated
    // person. Other mutations require coach/manager/admin rights.
    let personal=path=="/preferences" || path=="/account/profile" || path=="/account/password";
    let role=access::effective_role(&state,claims.sub).await?;
    if !matches!(method,Method::GET|Method::HEAD|Method::OPTIONS)
        && !personal && !role.can_write(){return Err(StatusCode::FORBIDDEN)}
    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}
