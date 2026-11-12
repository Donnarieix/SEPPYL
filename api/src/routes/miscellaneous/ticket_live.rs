//! Authenticated one-way live channel for ticket conversations.
//! SSE pushes events as they happen; messages, attachments and typing are sent
//! with ordinary authorized HTTP requests. Short-lived streams reauthenticate.
use axum::{
    Extension, Json,
    extract::{Path,State},
    http::{StatusCode,HeaderValue,header},
    response::{IntoResponse,Response,sse::{Event,KeepAlive,Sse}},
};
use sea_orm::EntityTrait;
use serde::Deserialize;
use std::{convert::Infallible,sync::OnceLock,time::Duration};
use tokio::sync::{broadcast,mpsc};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;
use crate::{
    entities::user,middleware::CurrentRole,
    routes::{authentication::login::Claims,miscellaneous::tickets},
    state::AppState,
};

#[derive(Clone,Debug)]
struct Signal{ticket_id:Uuid,kind:&'static str,actor:Uuid,username:String}
static CHANNEL:OnceLock<broadcast::Sender<Signal>>=OnceLock::new();
fn channel()->&'static broadcast::Sender<Signal>{
    CHANNEL.get_or_init(||broadcast::channel(256).0)
}
pub fn changed(ticket_id:Uuid,actor:Uuid){
    let _=channel().send(Signal{ticket_id,kind:"message",actor,username:String::new()});
}
pub async fn stream(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(ticket_id):Path<Uuid>,
)->Result<Response,StatusCode>{
    tickets::access(&state,&claims,role.0,ticket_id).await?;
    let mut events=channel().subscribe();
    let (tx,rx)=mpsc::channel::<Result<Event,Infallible>>(32);
    // Flush SSE headers immediately; waiting for keep-alives delays the UI.
    let _=tx.send(Ok(Event::default().event("ticket")
        .data(r#"{"kind":"connected"}"#))).await;

    tokio::spawn(async move{
        let expiration=tokio::time::sleep(Duration::from_secs(115));
        tokio::pin!(expiration);
        loop{
            tokio::select!{
                _=&mut expiration => break,
                event=events.recv()=>{
                    match event{
                        Ok(signal) if signal.ticket_id==ticket_id=>{
                            let data=serde_json::json!({"kind":signal.kind,"actor":signal.actor,
                                "username":signal.username});
                            if tx.send(Ok(Event::default().event("ticket").data(data.to_string())))
                                .await.is_err(){break;}
                        }
                        Ok(_)|Err(broadcast::error::RecvError::Lagged(_))=>{},
                        Err(broadcast::error::RecvError::Closed)=>break,
                    }
                }
            }
        }
    });
    let mut response=Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(12)).text("heartbeat"))
        .into_response();
    response.headers_mut().insert(header::CACHE_CONTROL,HeaderValue::from_static("no-cache, no-transform"));
    response.headers_mut().insert("x-accel-buffering",HeaderValue::from_static("no"));
    Ok(response)
}
#[derive(Deserialize)]
pub struct Typing{pub typing:bool}
pub async fn typing(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(ticket_id):Path<Uuid>,Json(body):Json<Typing>,
)->Result<StatusCode,StatusCode>{
    let ticket=tickets::access(&state,&claims,role.0,ticket_id).await?;
    if ticket.status=="closed" && !role.0.is_admin(){return Err(StatusCode::CONFLICT);}
    let account=user::Entity::find_by_id(claims.sub).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let kind=if body.typing{"typing"}else{"typing_end"};
    let _=channel().send(Signal{ticket_id,kind,actor:claims.sub,username:account.username});
    Ok(StatusCode::NO_CONTENT)
}
