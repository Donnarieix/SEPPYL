use axum::Extension;

use crate::routes::authentication::login::Claims;

pub async fn me(Extension(claims): Extension<Claims>) -> String {
    format!("{{username:{}}}", claims.sub)
}
