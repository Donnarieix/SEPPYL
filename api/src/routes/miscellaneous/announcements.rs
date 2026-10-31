//! Urgent banners and ordinary announcements deliberately separate from release notes.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,ColumnTrait,EntityTrait,QueryFilter,QueryOrder};
use serde::Deserialize;
use uuid::Uuid;
use crate::{entities::announcement,middleware::CurrentRole,state::AppState};
type R<T>=Result<T,StatusCode>;
fn err(e:sea_orm::DbErr)->StatusCode{
    tracing::error!(%e,"Announcements database failure");
    StatusCode::INTERNAL_SERVER_ERROR
}
fn now()->i64{std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
    .map(|t|t.as_secs() as i64).unwrap_or(0)}
#[derive(Deserialize)]
pub struct Payload{
    pub title:String,pub body:String,pub urgent:bool,
    pub published:bool,pub starts_at:i64,pub ends_at:Option<i64>,
}
fn valid(p:&Payload)->bool{
    !p.title.trim().is_empty()&&p.title.len()<=150&&
    !p.body.trim().is_empty()&&p.body.len()<=10_000&&
    p.ends_at.is_none_or(|end|end>p.starts_at)
}
pub async fn public(State(state):State<AppState>)->R<Json<Vec<announcement::Model>>>{
    let stamp=now();
    let rows=announcement::Entity::find()
        .filter(announcement::COLUMN.published.eq(true))
        .filter(announcement::COLUMN.starts_at.lte(stamp))
        .order_by_desc(announcement::COLUMN.created_at)
        .all(&state.db).await.map_err(err)?;
    Ok(Json(rows.into_iter().filter(|a|a.ends_at.is_none_or(|end|end>stamp)).collect()))
}
pub async fn all(
    Extension(role):Extension<CurrentRole>,State(state):State<AppState>,
)->R<Json<Vec<announcement::Model>>>{
    if !role.0.is_admin(){return Err(StatusCode::FORBIDDEN);}
    Ok(Json(announcement::Entity::find()
        .order_by_desc(announcement::COLUMN.created_at)
        .all(&state.db).await.map_err(err)?))
}
pub async fn create(
    Extension(role):Extension<CurrentRole>,State(state):State<AppState>,Json(p):Json<Payload>,
)->R<(StatusCode,Json<announcement::Model>)>{
    if !role.0.is_admin(){return Err(StatusCode::FORBIDDEN);}
    if !valid(&p){return Err(StatusCode::BAD_REQUEST);}
    let row=announcement::ActiveModel{
        id:Set(Uuid::new_v4()),title:Set(p.title.trim().to_owned()),
        body:Set(p.body.trim().to_owned()),urgent:Set(p.urgent),published:Set(p.published),
        starts_at:Set(p.starts_at),ends_at:Set(p.ends_at),created_at:Set(now()),
        ..Default::default()
    }.insert(&state.db).await.map_err(err)?;
    Ok((StatusCode::CREATED,Json(row)))
}
pub async fn update(
    Extension(role):Extension<CurrentRole>,State(state):State<AppState>,
    Path(id):Path<Uuid>,Json(p):Json<Payload>,
)->R<StatusCode>{
    if !role.0.is_admin(){return Err(StatusCode::FORBIDDEN);}
    if !valid(&p){return Err(StatusCode::BAD_REQUEST);}
    let row=announcement::Entity::find_by_id(id).one(&state.db).await.map_err(err)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut active:announcement::ActiveModel=row.into();
    active.title=Set(p.title.trim().to_owned());active.body=Set(p.body.trim().to_owned());
    active.urgent=Set(p.urgent);active.published=Set(p.published);
    active.starts_at=Set(p.starts_at);active.ends_at=Set(p.ends_at);
    active.update(&state.db).await.map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
pub async fn remove(
    Extension(role):Extension<CurrentRole>,State(state):State<AppState>,Path(id):Path<Uuid>,
)->R<StatusCode>{
    if !role.0.is_admin(){return Err(StatusCode::FORBIDDEN);}
    let result=announcement::Entity::delete_by_id(id).exec(&state.db).await.map_err(err)?;
    if result.rows_affected==0{return Err(StatusCode::NOT_FOUND);}
    Ok(StatusCode::NO_CONTENT)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn end_must_be_later_than_start(){
        let p=Payload{title:"Maintenance".into(),body:"Service indisponible".into(),
            urgent:true,published:true,starts_at:100,ends_at:Some(100)};
        assert!(!valid(&p));
    }
}
