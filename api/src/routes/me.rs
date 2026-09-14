use axum::Extension;

use crate::routes::login::Claims;

pub async fn me(Extension(claims): Extension<Claims>) -> String {
    format!("{{username:{}}}", claims.sub)
}
