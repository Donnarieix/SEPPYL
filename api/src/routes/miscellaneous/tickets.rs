//! Private support tickets. Ownership and staff role are checked at every read/write.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,ColumnTrait,EntityTrait,QueryFilter,QueryOrder,TransactionTrait,ConnectionTrait,Statement,DbBackend};
use serde::{Deserialize,Serialize};
use uuid::Uuid;
use std::collections::{HashMap,HashSet};
use crate::{
    entities::{ticket,ticket_message,ticket_message_revision,ticket_read,user},
    routes::miscellaneous::ticket_live,
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
fn precise_now()->i64{
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|t|t.as_micros() as i64).unwrap_or(0)
}
fn staff(role:Role)->bool{role.is_admin()}
fn visible(row:&ticket::Model,claims:&Claims,role:Role)->bool{
    staff(role)||row.author_id==claims.sub
}
pub(super) async fn access(state:&AppState,claims:&Claims,role:Role,id:Uuid)->R<ticket::Model>{
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
pub struct TicketDetail{pub ticket:ticket::Model,pub requester_name:String,pub messages:Vec<ticket_message::Model>}
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
#[derive(Serialize)]
pub struct UnreadTicket{pub ticket_id:Uuid,pub count:u64}
#[derive(Serialize)]
pub struct UnreadSummary{pub total:u64,pub tickets:Vec<UnreadTicket>}
/// A new ticket (its first message) is unread for staff; a requester only
/// receives notifications for replies by someone else. Reads are per account.
pub async fn unread(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,
)->R<Json<UnreadSummary>>{
    let query=ticket::Entity::find();
    let visible=if staff(role.0){query.all(&state.db).await.map_err(db)?}
        else{query.filter(ticket::COLUMN.author_id.eq(claims.sub))
            .all(&state.db).await.map_err(db)?};
    if visible.is_empty(){
        return Ok(Json(UnreadSummary{total:0,tickets:Vec::new()}));
    }
    let ids=visible.iter().map(|t|t.id).collect::<Vec<_>>();
    let reads=ticket_read::Entity::find()
        .filter(ticket_read::COLUMN.user_id.eq(claims.sub))
        .filter(ticket_read::COLUMN.ticket_id.is_in(ids.clone()))
        .all(&state.db).await.map_err(db)?;
    let seen=reads.into_iter().map(|r|r.message_id).collect::<HashSet<_>>();
    let messages=ticket_message::Entity::find()
        .filter(ticket_message::COLUMN.ticket_id.is_in(ids))
        .order_by_asc(ticket_message::COLUMN.ticket_id)
        .order_by_asc(ticket_message::COLUMN.created_at)
        .order_by_asc(ticket_message::COLUMN.id)
        .all(&state.db).await.map_err(db)?;
    let mut pending=HashMap::<Uuid,u64>::new();
    for message in messages{
        if !seen.contains(&message.id) && message.author_id!=claims.sub {
            *pending.entry(message.ticket_id).or_default()+=1;
        }
    }
    let mut tickets=pending.into_iter().map(|(ticket_id,count)|UnreadTicket{ticket_id,count})
        .collect::<Vec<_>>();
    tickets.sort_by_key(|t|t.ticket_id);
    let total=tickets.iter().map(|t|t.count).sum();
    Ok(Json(UnreadSummary{total,tickets}))
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
        message:Set(payload.message.trim().into()),attachment_name:Set(None),created_at:Set(precise_now()),
        ..Default::default()
    }.insert(&txn).await.map_err(db)?;
    txn.commit().await.map_err(db)?;
    notify_discord(row.id,&row.subject);
    ticket_live::changed(row.id,claims.sub);
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
        .order_by_asc(ticket_message::COLUMN.id)
        .all(&state.db).await.map_err(db)?;
    // Persist a receipt for each fetched message. UUIDs and timestamps do
    // not determine read order, so replies in the same second are safe.
    let seen=ticket_read::Entity::find()
        .filter(ticket_read::COLUMN.user_id.eq(claims.sub))
        .filter(ticket_read::COLUMN.ticket_id.eq(id))
        .all(&state.db).await.map_err(db)?;
    let known=seen.into_iter().map(|r|r.message_id).collect::<HashSet<_>>();
    for message in &messages{
        if !known.contains(&message.id){
            state.db.execute_raw(Statement::from_sql_and_values(DbBackend::Sqlite,
                "INSERT OR IGNORE INTO support_ticket_reads (user_id, message_id, ticket_id) VALUES (?, ?, ?)",
                vec![claims.sub.into(),message.id.into(),id.into()]
            )).await.map_err(db)?;
        }
    }
    let requester_name=user::Entity::find_by_id(row.author_id)
        .one(&state.db).await.map_err(db)?
        .map(|account|account.username).unwrap_or_else(||"Compte supprimé".into());
    Ok(Json(TicketDetail{ticket:row,requester_name,messages}))
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
        message:Set(payload.message.trim().into()),created_at:Set(precise_now()),
        ..Default::default()
    }.insert(&state.db).await.map_err(db)?;
    let mut active:ticket::ActiveModel=row.into();
    active.updated_at=Set(stamp);
    active.update(&state.db).await.map_err(db)?;
    notify_discord(id,&format!("Nouveau message dans le ticket : {}",entry.ticket_id));
    ticket_live::changed(id,claims.sub);
    Ok((StatusCode::CREATED,Json(entry)))
}
#[derive(Deserialize)]
pub struct EditMessage{pub message:String}
/// Editing is limited to the original author. A revision is inserted in the
/// same transaction as the update so pre-edit contents can never be lost.
pub async fn edit_message(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path((ticket_id,message_id)):Path<(Uuid,Uuid)>,
    Json(payload):Json<EditMessage>,
)->R<Json<ticket_message::Model>>{
    let ticket=access(&state,&claims,role.0,ticket_id).await?;
    if ticket.status=="closed"&&!staff(role.0){return Err(StatusCode::CONFLICT);}
    if !valid_message(&payload.message){return Err(StatusCode::BAD_REQUEST);}
    let current=ticket_message::Entity::find_by_id(message_id)
        .one(&state.db).await.map_err(db)?.ok_or(StatusCode::NOT_FOUND)?;
    if current.ticket_id!=ticket_id{return Err(StatusCode::NOT_FOUND);}
    if current.author_id!=claims.sub{return Err(StatusCode::FORBIDDEN);}
    if current.deleted_at.is_some(){return Err(StatusCode::CONFLICT);}
    let changed=payload.message.trim();
    if current.message==changed{return Err(StatusCode::BAD_REQUEST);}
    let timestamp=now();
    let txn=state.db.begin().await.map_err(db)?;
    ticket_message_revision::ActiveModel{
        id:Set(Uuid::new_v4()),ticket_id:Set(ticket_id),message_id:Set(message_id),
        edited_by:Set(claims.sub),previous_text:Set(current.message.clone()),
        previous_attachment_name:Set(current.attachment_name.clone()),
        action:Set("edit".into()),changed_at:Set(precise_now()),
        ..Default::default()
    }.insert(&txn).await.map_err(db)?;
    let mut active:ticket_message::ActiveModel=current.into();
    active.message=Set(changed.into());
    active.edited_at=Set(Some(timestamp));
    let updated=active.update(&txn).await.map_err(db)?;
    txn.commit().await.map_err(db)?;
    ticket_live::changed(ticket_id,claims.sub);
    Ok(Json(updated))
}
/// Staff may remove a message from the conversation, but its full contents
/// remain in an immutable revision accessible only to staff.
pub async fn delete_message(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path((ticket_id,message_id)):Path<(Uuid,Uuid)>,
)->R<StatusCode>{
    if !staff(role.0){return Err(StatusCode::FORBIDDEN);}
    access(&state,&claims,role.0,ticket_id).await?;
    let current=ticket_message::Entity::find_by_id(message_id)
        .one(&state.db).await.map_err(db)?.ok_or(StatusCode::NOT_FOUND)?;
    if current.ticket_id!=ticket_id{return Err(StatusCode::NOT_FOUND);}
    if current.deleted_at.is_some(){return Err(StatusCode::CONFLICT);}
    let timestamp=now();
    let txn=state.db.begin().await.map_err(db)?;
    ticket_message_revision::ActiveModel{
        id:Set(Uuid::new_v4()),ticket_id:Set(ticket_id),message_id:Set(message_id),
        edited_by:Set(claims.sub),previous_text:Set(current.message.clone()),
        previous_attachment_name:Set(current.attachment_name.clone()),
        action:Set("delete".into()),changed_at:Set(precise_now()),
        ..Default::default()
    }.insert(&txn).await.map_err(db)?;
    let mut active:ticket_message::ActiveModel=current.into();
    active.message=Set(String::new());
    active.attachment_name=Set(None);
    active.deleted_at=Set(Some(timestamp));
    active.update(&txn).await.map_err(db)?;
    txn.commit().await.map_err(db)?;
    ticket_live::changed(ticket_id,claims.sub);
    Ok(StatusCode::NO_CONTENT)
}
/// The full revision trail, including pre-deletion contents, never leaves
/// the authenticated staff API.
/// Restoring a historical version creates another audit entry. A staff member
/// can therefore recover pre-edit or pre-deletion content without erasing history.
pub async fn restore_message(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path((ticket_id,message_id,revision_id)):Path<(Uuid,Uuid,Uuid)>,
)->R<Json<ticket_message::Model>>{
    if !staff(role.0){return Err(StatusCode::FORBIDDEN);}
    access(&state,&claims,role.0,ticket_id).await?;
    let previous=ticket_message_revision::Entity::find_by_id(revision_id)
        .one(&state.db).await.map_err(db)?.ok_or(StatusCode::NOT_FOUND)?;
    if previous.ticket_id!=ticket_id||previous.message_id!=message_id{
        return Err(StatusCode::NOT_FOUND);
    }
    let current=ticket_message::Entity::find_by_id(message_id)
        .one(&state.db).await.map_err(db)?.ok_or(StatusCode::NOT_FOUND)?;
    if current.ticket_id!=ticket_id{return Err(StatusCode::NOT_FOUND);}
    let stamp=now();
    let txn=state.db.begin().await.map_err(db)?;
    ticket_message_revision::ActiveModel{
        id:Set(Uuid::new_v4()),ticket_id:Set(ticket_id),message_id:Set(message_id),
        edited_by:Set(claims.sub),previous_text:Set(current.message.clone()),
        previous_attachment_name:Set(current.attachment_name.clone()),
        action:Set("restore".into()),changed_at:Set(precise_now()),
        ..Default::default()
    }.insert(&txn).await.map_err(db)?;
    let mut active:ticket_message::ActiveModel=current.into();
    active.message=Set(previous.previous_text);
    active.attachment_name=Set(previous.previous_attachment_name);
    active.deleted_at=Set(None);
    active.edited_at=Set(Some(stamp));
    let restored=active.update(&txn).await.map_err(db)?;
    txn.commit().await.map_err(db)?;
    ticket_live::changed(ticket_id,claims.sub);
    Ok(Json(restored))
}
pub async fn message_history(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path((ticket_id,message_id)):Path<(Uuid,Uuid)>,
)->R<Json<Vec<ticket_message_revision::Model>>>{
    if !staff(role.0){return Err(StatusCode::FORBIDDEN);}
    access(&state,&claims,role.0,ticket_id).await?;
    let current=ticket_message::Entity::find_by_id(message_id)
        .one(&state.db).await.map_err(db)?.ok_or(StatusCode::NOT_FOUND)?;
    if current.ticket_id!=ticket_id{return Err(StatusCode::NOT_FOUND);}
    let trail=ticket_message_revision::Entity::find()
        .filter(ticket_message_revision::COLUMN.message_id.eq(message_id))
        .filter(ticket_message_revision::COLUMN.ticket_id.eq(ticket_id))
        .order_by_asc(ticket_message_revision::COLUMN.changed_at)
        .all(&state.db).await.map_err(db)?;
    Ok(Json(trail))
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
    ticket_live::changed(id,claims.sub);
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
