use axum::{Extension,Json,extract::State,http::StatusCode};
use sea_orm::{EntityTrait,QueryOrder,QuerySelect};
use crate::{entities::audit_event,middleware::{CurrentRole,Role},state::AppState};

pub async fn list(
    Extension(role): Extension<CurrentRole>,
    State(state):State<AppState>
)->Result<Json<Vec<audit_event::Model>>,StatusCode>{
    if role.0!=Role::Administrator {return Err(StatusCode::FORBIDDEN);}
    let rows=audit_event::Entity::find()
        .order_by_desc(audit_event::COLUMN.timestamp)
        .limit(100)
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
