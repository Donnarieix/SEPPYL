//! Append-only audit for successful mutations. Request bodies, passwords and
//! bearer tokens are deliberately not recorded.
use axum::{extract::{Request,State},middleware::Next,response::Response,http::Method};
use sea_orm::{ActiveModelTrait,ActiveValue::Set};
use crate::{entities::audit_event,routes::authentication::login::Claims,state::AppState};

pub async fn record(
    State(state):State<AppState>,
    request:Request,
    next:Next,
)->Response{
    let action=match *request.method(){
        Method::POST=>"create",
        Method::PUT|Method::PATCH=>"update",
        Method::DELETE=>"delete",
        _=>"read",
    };
    let resource=request.uri().path().to_string();
    let actor=request.extensions().get::<Claims>().map(|c|c.sub);
    let response=next.run(request).await;
    if action!="read" && response.status().is_success(){
        let entry=audit_event::ActiveModel{
            id:Set(uuid::Uuid::new_v4()),
            actor_id:Set(actor),
            action:Set(action.to_string()),
            resource:Set(resource),
            status:Set(i32::from(response.status().as_u16())),
            occurred_at:Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        };
        if let Err(error)=entry.insert(&state.db).await{
            // Fail closed on startup migration errors, but do not silently
            // change a completed business action into an HTTP 500 afterward.
            eprintln!("AUDIT_WRITE_FAILED: {error}");
        }
    }
    response
}
