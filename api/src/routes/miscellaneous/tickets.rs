//! Private support tickets. Ownership and staff role are checked at every read/write.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,ColumnTrait,EntityTrait,QueryFilter,QueryOrder,TransactionTrait};
use serde::{Deserialize,Serialize};
use uuid::Uuid;
use crate::{
    entities::{ticket,ticket_message},
    middleware::{CurrentRole,Role},
    routes::authentication::login::Claims,state::AppState,
};
type R<T>=Result<T,StatusCode>;
fn db(error:sea_orm::DbErr)->StatusCode{
    tracing::error!(%error,"Ticket database failure");
    StatusCode::INTERNAL_SERVER_ERROR
}
fn now()->i64{
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|t|t.as_secs() as i64).unwrap_or(0)
}
fn staff(role:Role)->bool{role.is_admin()}
fn visible(row:&ticket::Model,claims:&Claims,role:Role)->bool{
    staff(role)||row.author_id==claims.sub
}
async fn access(state:&AppState,claims:&Claims,role:Role,id:Uuid)->R<ticket::Model>{
    let row=ticket::Entity::find_by_id(id).one(&state.db).await.map_err(db)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !visible(&row,claims,role){return Err(StatusCode::NOT_FOUND);}
    Ok(row)
}
#[derive(Deserialize)]
pub struct NewTicket{pub subject:String,pub message:String}
#[derive(Deserialize)]
pub struct NewMessage{pub message:String}
#[derive(Deserialize)]
pub struct TicketStatus{pub status:String,pub maintainer_required:bool}
#[derive(Serialize)]
pub struct TicketDetail{pub ticket:ticket::Model,pub messages:Vec<ticket_message::Model>}
fn valid_message(s:&str)->bool{!s.trim().is_empty()&&s.len()<=10_000}
pub async fn list(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,
)->R<Json<Vec<ticket::Model>>>{
    let query=ticket::Entity::find().order_by_desc(ticket::COLUMN.updated_at);
    let rows=if staff(role.0){query.all(&state.db).await.map_err(db)?}
    else{query.filter(ticket::COLUMN.author_id.eq(claims.sub)).all(&state.db).await.map_err(db)?};
    Ok(Json(rows))
}
pub async fn create(
    Extension(claims):Extension<Claims>,State(state):State<AppState>,
    Json(payload):Json<NewTicket>,
)->R<(StatusCode,Json<ticket::Model>)>{
    if payload.subject.trim().is_empty()||payload.subject.len()>160
        ||!valid_message(&payload.message){return Err(StatusCode::BAD_REQUEST);}
    let stamp=now();
    let txn=state.db.begin().await.map_err(db)?;
    let row=ticket::ActiveModel{
        id:Set(Uuid::new_v4()),author_id:Set(claims.sub),
        subject:Set(payload.subject.trim().into()),status:Set("open".into()),
        maintainer_required:Set(false),created_at:Set(stamp),updated_at:Set(stamp),
        ..Default::default()
    }.insert(&txn).await.map_err(db)?;
    ticket_message::ActiveModel{
        id:Set(Uuid::new_v4()),ticket_id:Set(row.id),author_id:Set(claims.sub),
        message:Set(payload.message.trim().into()),created_at:Set(stamp),
        ..Default::default()
    }.insert(&txn).await.map_err(db)?;
    txn.commit().await.map_err(db)?;
    notify_discord(row.id,&row.subject);
    Ok((StatusCode::CREATED,Json(row)))
}
pub async fn get(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,
)->R<Json<TicketDetail>>{
    let row=access(&state,&claims,role.0,id).await?;
    let messages=ticket_message::Entity::find()
        .filter(ticket_message::COLUMN.ticket_id.eq(id))
        .order_by_asc(ticket_message::COLUMN.created_at)
        .all(&state.db).await.map_err(db)?;
    Ok(Json(TicketDetail{ticket:row,messages}))
}
pub async fn message(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(payload):Json<NewMessage>,
)->R<(StatusCode,Json<ticket_message::Model>)>{
    let row=access(&state,&claims,role.0,id).await?;
    if !valid_message(&payload.message){return Err(StatusCode::BAD_REQUEST);}
    if row.status=="closed"&&!staff(role.0){return Err(StatusCode::CONFLICT);}
    let stamp=now();
    let entry=ticket_message::ActiveModel{
        id:Set(Uuid::new_v4()),ticket_id:Set(id),author_id:Set(claims.sub),
        message:Set(payload.message.trim().into()),created_at:Set(stamp),
        ..Default::default()
    }.insert(&state.db).await.map_err(db)?;
    let mut active:ticket::ActiveModel=row.into();
    active.updated_at=Set(stamp);
    active.update(&state.db).await.map_err(db)?;
    Ok((StatusCode::CREATED,Json(entry)))
}
pub async fn status(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(payload):Json<TicketStatus>,
)->R<StatusCode>{
    if !staff(role.0){return Err(StatusCode::FORBIDDEN);}
    if !matches!(payload.status.as_str(),"open"|"in_progress"|"resolved"|"closed"){
        return Err(StatusCode::BAD_REQUEST);
    }
    let row=access(&state,&claims,role.0,id).await?;
    let new_mention=payload.maintainer_required&&!row.maintainer_required;
    let mut active:ticket::ActiveModel=row.clone().into();
    active.status=Set(payload.status);active.maintainer_required=Set(payload.maintainer_required);
    active.updated_at=Set(now());active.update(&state.db).await.map_err(db)?;
    if new_mention{notify_discord(id,&format!("Escalade Maintainer : {}",row.subject));}
    Ok(StatusCode::NO_CONTENT)
}
/// Optional outbound-only notification. The webhook is a server-side secret,
/// never an API parameter or user-controlled URL. Text is truncated and
/// allowed mentions are explicitly disabled.
fn notify_discord(id:Uuid,subject:&str){
    let Ok(webhook)=std::env::var("SEPPYL_DISCORD_TICKET_WEBHOOK") else{return;};
    let Ok(url)=reqwest::Url::parse(&webhook) else{return;};
    if url.scheme()!="https" || !matches!(url.host_str(),Some("discord.com"|"discordapp.com"))
        || !url.path().starts_with("/api/webhooks/") {return;}
    // Optional Discord role/group ping. Administrators configure the snowflake
    // only on the server; arbitrary user-supplied @mentions stay disabled.
    let role_id=std::env::var("SEPPYL_DISCORD_TICKET_ROLE_ID").ok()
        .filter(|v|(15..=22).contains(&v.len())&&v.bytes().all(|b|b.is_ascii_digit()));
    let ping=role_id.as_ref().map(|id|format!("<@&{id}> ")).unwrap_or_default();
    let line=format!("{ping}Ticket SEPPYL {} — {}",id,subject.chars().take(160).collect::<String>());
    tokio::spawn(async move {
        let mentions=role_id.into_iter().collect::<Vec<_>>();
        let result=reqwest::Client::new().post(url)
            .timeout(std::time::Duration::from_secs(6))
            .json(&serde_json::json!({"content":line,"allowed_mentions":{"parse":[],"roles":mentions}}))
            .send().await;
        match result {
            Ok(response) if response.status().is_success()=>{},
            _=>tracing::warn!(ticket=%id,"Discord support notification unavailable"),
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ticket_visibility_and_maintainer_escalation_are_not_public(){
        let creator=Uuid::new_v4();
        let t=ticket::Model{id:Uuid::new_v4(),author_id:creator,
            subject:"Test".into(),status:"open".into(),maintainer_required:false,
            created_at:1,updated_at:1};
        assert!(visible(&t,&Claims{sub:creator,exp:u64::MAX},Role::User));
        assert!(!visible(&t,&Claims{sub:Uuid::new_v4(),exp:u64::MAX},Role::User));
        assert!(visible(&t,&Claims{sub:Uuid::new_v4(),exp:u64::MAX},Role::Administrator));
    }
}
