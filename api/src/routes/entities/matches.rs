//! Club-scoped matches, lineups and minute-accurate participation derived
//! from ordered substitutions. No individual statistics are public.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,ColumnTrait,EntityTrait,QueryFilter,QueryOrder,TransactionTrait};
use sea_orm::entity::prelude::DateTime;
use serde::{Deserialize,Serialize};
use std::collections::{HashMap,HashSet};
use uuid::Uuid;
use crate::{
    entities::{match_game,match_participant,match_event,match_event_type,team},
    middleware::CurrentRole,
    routes::{authentication::login::Claims,entities::organization},
    state::AppState,
};
type R<T>=Result<T,StatusCode>;
fn db_error(error:sea_orm::DbErr)->StatusCode{
    tracing::error!(%error,"Match database operation failed");
    StatusCode::INTERNAL_SERVER_ERROR
}
async fn access(state:&AppState,claims:&Claims,role:CurrentRole,id:Uuid,write:bool)->R<match_game::Model>{
    let game=match_game::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let allowed=if write{
        organization::can_write_team(state,claims,role.0,game.team_id).await?
    }else{
        organization::can_read_team(state,claims,role.0,game.team_id).await?
    };
    if !allowed {return Err(StatusCode::NOT_FOUND);}
    Ok(game)
}
#[derive(Deserialize)]
pub struct NewGame{pub team_id:Uuid,pub opponent:String,pub date:DateTime,pub half_minutes:i32}
#[derive(Deserialize)]
pub struct GameUpdate{pub opponent:String,pub date:DateTime,pub half_minutes:i32,pub notes:String}
fn valid_game(opponent:&str,half:i32)->bool{
    !opponent.trim().is_empty()&&opponent.len()<=160&&(1..=120).contains(&half)
}
#[derive(Serialize)]
pub struct PlayTime{pub player_id:Uuid,pub minutes:i32}
#[derive(Serialize)]
pub struct GameDetail{
    pub game:match_game::Model,
    pub participants:Vec<match_participant::Model>,
    pub events:Vec<match_event::Model>,
    pub event_types:Vec<match_event_type::Model>,
    pub playing_times:Vec<PlayTime>,
}
/// The clock never resets between halves: an event at 50 in a 45-minute-half
/// fixture means 5 minutes into the second half.
pub fn compute_playtime(
    half:i32,players:&[match_participant::Model],events:&[match_event::Model],
)->R<Vec<PlayTime>>{
    if !(1..=120).contains(&half){return Err(StatusCode::BAD_REQUEST);}
    let finish=half*2;
    let mut known=HashSet::new();
    let mut entered=HashMap::<Uuid,i32>::new();
    let mut totals=HashMap::<Uuid,i32>::new();
    for p in players {
        if !known.insert(p.player_id){return Err(StatusCode::BAD_REQUEST);}
        totals.insert(p.player_id,0);
        if p.starter{entered.insert(p.player_id,0);}
    }
    let mut ordered=events.to_vec();
    ordered.sort_by_key(|event|(event.minute,event.sort_order));
    for event in ordered {
        if event.minute<0||event.minute>finish{return Err(StatusCode::BAD_REQUEST);}
        if let Some(player)=event.player_id {
            if !known.contains(&player){return Err(StatusCode::BAD_REQUEST);}
        }
        if event.kind=="substitution"{
            let out=event.player_id.ok_or(StatusCode::BAD_REQUEST)?;
            let incoming=event.player_in_id.ok_or(StatusCode::BAD_REQUEST)?;
            if !known.contains(&incoming)||out==incoming||entered.contains_key(&incoming) {
                return Err(StatusCode::BAD_REQUEST);
            }
            let started=entered.remove(&out).ok_or(StatusCode::BAD_REQUEST)?;
            *totals.get_mut(&out).ok_or(StatusCode::BAD_REQUEST)?+=event.minute-started;
            entered.insert(incoming,event.minute);
        } else if event.player_in_id.is_some() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    for (id,start) in entered {
        *totals.get_mut(&id).ok_or(StatusCode::BAD_REQUEST)?+=finish-start;
    }
    let mut result=totals.into_iter().map(|(player_id,minutes)|PlayTime{player_id,minutes})
        .collect::<Vec<_>>();
    result.sort_by_key(|p|p.player_id);
    Ok(result)
}
async fn detail(state:&AppState,game:match_game::Model)->R<GameDetail>{
    let participants=match_participant::Entity::find()
        .filter(match_participant::COLUMN.match_id.eq(game.id))
        .all(&state.db).await.map_err(db_error)?;
    let events=match_event::Entity::find().filter(match_event::COLUMN.match_id.eq(game.id))
        .order_by_asc(match_event::COLUMN.minute).order_by_asc(match_event::COLUMN.sort_order)
        .all(&state.db).await.map_err(db_error)?;
    let club_id=team::Entity::find_by_id(game.team_id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?.club_id;
    let event_types=match_event_type::Entity::find()
        .filter(match_event_type::COLUMN.club_id.eq(club_id)).all(&state.db).await.map_err(db_error)?;
    let playing_times=compute_playtime(game.half_minutes,&participants,&events)?;
    Ok(GameDetail{game,participants,events,event_types,playing_times})
}
pub async fn list(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,
)->R<Json<Vec<match_game::Model>>>{
    let mut result=Vec::new();
    for game in match_game::Entity::find().order_by_desc(match_game::COLUMN.date)
        .all(&state.db).await.map_err(db_error)? {
        if organization::can_read_team(&state,&claims,role.0,game.team_id).await? {
            result.push(game);
        }
    }
    Ok(Json(result))
}
pub async fn create(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Json(body):Json<NewGame>,
)->R<(StatusCode,Json<match_game::Model>)>{
    if !valid_game(&body.opponent,body.half_minutes){return Err(StatusCode::BAD_REQUEST);}
    if !organization::can_write_team(&state,&claims,role.0,body.team_id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    let saved=match_game::ActiveModel{
        id:Set(Uuid::new_v4()),team_id:Set(body.team_id),created_by:Set(claims.sub),
        opponent:Set(body.opponent.trim().into()),date:Set(body.date),
        half_minutes:Set(body.half_minutes),notes:Set(String::new()),
        ..Default::default()
    }.insert(&state.db).await.map_err(db_error)?;
    Ok((StatusCode::CREATED,Json(saved)))
}
pub async fn get(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,
)->R<Json<GameDetail>>{
    let game=access(&state,&claims,role,id,false).await?;
    Ok(Json(detail(&state,game).await?))
}
pub async fn update(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(body):Json<GameUpdate>,
)->R<StatusCode>{
    if !valid_game(&body.opponent,body.half_minutes)||body.notes.len()>20_000 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let game=access(&state,&claims,role,id,true).await?;
    let old=detail(&state,game.clone()).await?;
    compute_playtime(body.half_minutes,&old.participants,&old.events)?;
    let mut active:match_game::ActiveModel=game.into();
    active.opponent=Set(body.opponent.trim().into());
    active.date=Set(body.date);active.half_minutes=Set(body.half_minutes);
    active.notes=Set(body.notes);
    active.update(&state.db).await.map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
pub struct LineupEntry{pub player_id:Uuid,pub position:String,pub starter:bool}
#[derive(Deserialize)]
pub struct Lineup{pub players:Vec<LineupEntry>}
pub async fn set_lineup(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(body):Json<Lineup>,
)->R<StatusCode>{
    let game=access(&state,&claims,role,id,true).await?;
    if body.players.len()>100 || body.players.iter().any(|p|p.position.len()>60) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let roster=organization::list_team_players_internal(&state,game.team_id).await?;
    let available:HashSet<_>=roster.iter().map(|p|p.id).collect();
    if body.players.iter().any(|p|!available.contains(&p.player_id)){
        return Err(StatusCode::FORBIDDEN);
    }
    let existing=detail(&state,game.clone()).await?;
    let new=body.players.into_iter().map(|p|match_participant::Model{
        match_id:id,player_id:p.player_id,position:p.position.trim().to_string(),starter:p.starter
    }).collect::<Vec<_>>();
    compute_playtime(game.half_minutes,&new,&existing.events)?;
    let txn=state.db.begin().await.map_err(db_error)?;
    match_participant::Entity::delete_many()
        .filter(match_participant::COLUMN.match_id.eq(id)).exec(&txn).await.map_err(db_error)?;
    for entry in new{
        match_participant::ActiveModel{
            match_id:Set(id),player_id:Set(entry.player_id),position:Set(entry.position),
            starter:Set(entry.starter),..Default::default()
        }.insert(&txn).await.map_err(db_error)?;
    }
    txn.commit().await.map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
pub struct NewEventType{pub label:String}
pub async fn create_type(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(body):Json<NewEventType>,
)->R<(StatusCode,Json<match_event_type::Model>)>{
    let game=access(&state,&claims,role,id,true).await?;
    if body.label.trim().is_empty()||body.label.len()>80{return Err(StatusCode::BAD_REQUEST);}
    let club_id=team::Entity::find_by_id(game.team_id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?.club_id;
    let entry=match_event_type::ActiveModel{
        id:Set(Uuid::new_v4()),club_id:Set(club_id),
        label:Set(body.label.trim().to_string()),kind:Set("other".into()),
        ..Default::default()
    }.insert(&state.db).await.map_err(db_error)?;
    Ok((StatusCode::CREATED,Json(entry)))
}
#[derive(Deserialize)]
pub struct NewEvent{
    pub minute:i32,pub kind:String,pub event_type_id:Option<Uuid>,
    pub player_id:Option<Uuid>,pub player_in_id:Option<Uuid>,
    #[serde(default)]pub notes:String,
}
fn valid_kind(value:&str)->bool{
    matches!(value,"goal"|"substitution"|"yellow_card"|"red_card"|"injury"|"other")
}
pub async fn add_event(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(body):Json<NewEvent>,
)->R<(StatusCode,Json<match_event::Model>)>{
    let game=access(&state,&claims,role,id,true).await?;
    if !valid_kind(&body.kind)||body.notes.len()>2000{return Err(StatusCode::BAD_REQUEST);}
    if let Some(type_id)=body.event_type_id{
        if body.kind!="other"{return Err(StatusCode::BAD_REQUEST);}
        let ty=match_event_type::Entity::find_by_id(type_id).one(&state.db).await.map_err(db_error)?
            .ok_or(StatusCode::BAD_REQUEST)?;
        let club_id=team::Entity::find_by_id(game.team_id).one(&state.db).await.map_err(db_error)?
            .ok_or(StatusCode::NOT_FOUND)?.club_id;
        if ty.club_id!=club_id{return Err(StatusCode::FORBIDDEN);}
    }
    let existing=detail(&state,game.clone()).await?;
    let order=existing.events.len() as i32;
    let entry=match_event::Model{
        id:Uuid::new_v4(),match_id:id,event_type_id:body.event_type_id,
        minute:body.minute,kind:body.kind,
        player_id:body.player_id,player_in_id:body.player_in_id,
        notes:body.notes,sort_order:order,
    };
    let mut proposed=existing.events;
    proposed.push(entry.clone());
    compute_playtime(game.half_minutes,&existing.participants,&proposed)?;
    let saved=match_event::ActiveModel{
        id:Set(entry.id),match_id:Set(id),event_type_id:Set(entry.event_type_id),
        minute:Set(entry.minute),kind:Set(entry.kind),player_id:Set(entry.player_id),
        player_in_id:Set(entry.player_in_id),notes:Set(entry.notes),
        sort_order:Set(entry.sort_order),..Default::default()
    }.insert(&state.db).await.map_err(db_error)?;
    Ok((StatusCode::CREATED,Json(saved)))
}
pub async fn remove_event(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path((id,event_id)):Path<(Uuid,Uuid)>,
)->R<StatusCode>{
    let game=access(&state,&claims,role,id,true).await?;
    let details=detail(&state,game.clone()).await?;
    if !details.events.iter().any(|e|e.id==event_id){return Err(StatusCode::NOT_FOUND);}
    let remaining=details.events.into_iter().filter(|e|e.id!=event_id).collect::<Vec<_>>();
    compute_playtime(game.half_minutes,&details.participants,&remaining)?;
    match_event::Entity::delete_by_id(event_id).exec(&state.db).await.map_err(db_error)?;
    Ok(StatusCode::NO_CONTENT)
}
#[cfg(test)]
mod tests{
    use super::*;
    fn p(id:Uuid,starter:bool)->match_participant::Model{
        match_participant::Model{match_id:Uuid::nil(),player_id:id,starter,position:"".into()}
    }
    fn substitution(out:Uuid,into:Uuid,minute:i32)->match_event::Model{
        match_event::Model{id:Uuid::new_v4(),match_id:Uuid::nil(),event_type_id:None,
            kind:"substitution".into(),minute,player_id:Some(out),player_in_id:Some(into),
            notes:String::new(),sort_order:0}
    }
    #[test]
    fn calculates_minutes_from_valid_substitutions(){
        let a=Uuid::new_v4();let b=Uuid::new_v4();let c=Uuid::new_v4();
        let ps=vec![p(a,true),p(b,false),p(c,false)];
        let events=vec![substitution(a,b,30),substitution(b,c,65)];
        let stats=compute_playtime(45,&ps,&events).unwrap();
        let time=|id|stats.iter().find(|p|p.player_id==id).unwrap().minutes;
        assert_eq!((time(a),time(b),time(c)),(30,35,25));
        assert!(compute_playtime(45,&ps,&[substitution(a,b,30),substitution(a,c,40)]).is_err());
        assert!(compute_playtime(45,&ps,&[substitution(b,a,20)]).is_err());
    }
}
