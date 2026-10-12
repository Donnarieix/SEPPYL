use axum::Extension;

use crate::routes::authentication::login::Claims;

#[utoipa::path(
    get,
    path = "/me",
    responses(
        (status = 200, description = "Get current user information")
    )
)]
pub async fn me(Extension(claims): Extension<Claims>) -> String {
    format!("{{username:{}}}", claims.sub)
}
