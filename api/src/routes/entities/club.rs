//! Membership management. Global admins can manage every club. Club managers
//! manage their own teams and memberships, never global administrators.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,EntityTrait,QueryFilter};
use serde::{Serialize,Deserialize};
use uuid::Uuid;
use crate::{
  access::{self,Role},
  entities::{club,team,club_member,team_member,user},
  routes::authentication::login::Claims,state::AppState,
};
#[derive(Serialize)]
pub struct ClubSummary {
    pub club:club::Model,
    pub teams:Vec<team::Model>,
    pub role:String,
}
#[derive(Deserialize)]
pub struct Named {pub name:String}
#[derive(Deserialize)]
pub struct Membership {pub user_id:Uuid,pub role:String}

fn valid_name(name:&str)->bool{!name.trim().is_empty()&&name.len()<=120}
async fn club_visible(state:&AppState,actor:Uuid,club_id:Uuid)->Result<Role,StatusCode>{
    let global=access::effective_role(state,actor).await?;
    if global==Role::Administrator{return Ok(global)}
    let role=access::club_role(state,actor,club_id).await?;
    if role!=Role::Visitor{return Ok(role)}
    let teams=team::Entity::find().filter(team::COLUMN.club_id.eq(club_id))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    for entry in teams {
        let role=access::team_role(state,actor,entry.id,club_id).await?;
        if role.can_read(){return Ok(role)}
    }
    Err(StatusCode::NOT_FOUND)
}
pub async fn list(
 Extension(claims):Extension<Claims>,State(state):State<AppState>,
)->Result<Json<Vec<ClubSummary>>,StatusCode>{
    let clubs=club::Entity::find().all(&state.db).await
       .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut visible=Vec::new();
    for item in clubs {
        let Ok(role)=club_visible(&state,claims.sub,item.id).await else {continue};
        let teams=team::Entity::find().filter(team::COLUMN.club_id.eq(item.id))
            .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
        visible.push(ClubSummary {club:item,teams,role:format!("{role:?}").to_lowercase()});
    }
    Ok(Json(visible))
}
pub async fn create_club(
 Extension(claims):Extension<Claims>,State(state):State<AppState>,Json(body):Json<Named>,
)->Result<(StatusCode,Json<club::Model>),StatusCode>{
    if access::effective_role(&state,claims.sub).await?!=Role::Administrator{return Err(StatusCode::FORBIDDEN)}
    if !valid_name(&body.name){return Err(StatusCode::BAD_REQUEST)}
    let club=club::ActiveModel{id:Set(Uuid::new_v4()),name:Set(body.name.trim().into()),..Default::default()}
        .insert(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED,Json(club)))
}
pub async fn create_team(
 Extension(claims):Extension<Claims>,State(state):State<AppState>,
 Path(club_id):Path<Uuid>,Json(body):Json<Named>,
)->Result<(StatusCode,Json<team::Model>),StatusCode>{
    if !matches!(club_visible(&state,claims.sub,club_id).await?,Role::Administrator|Role::Manager)
        {return Err(StatusCode::FORBIDDEN)}
    if !valid_name(&body.name){return Err(StatusCode::BAD_REQUEST)}
    let item=team::ActiveModel{
        id:Set(Uuid::new_v4()),club_id:Set(club_id),
        name:Set(body.name.trim().into()),..Default::default()
    }.insert(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED,Json(item)))
}
pub async fn assign_club(
 Extension(claims):Extension<Claims>,State(state):State<AppState>,
 Path(club_id):Path<Uuid>,Json(body):Json<Membership>,
)->Result<StatusCode,StatusCode>{
    if !matches!(club_visible(&state,claims.sub,club_id).await?,Role::Administrator|Role::Manager)
        {return Err(StatusCode::FORBIDDEN)}
    if !["reader","manager"].contains(&body.role.as_str()){return Err(StatusCode::BAD_REQUEST)}
    if user::Entity::find_by_id(body.user_id).one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.is_none(){
        return Err(StatusCode::NOT_FOUND)
    }
    let previous=club_member::Entity::find_by_id((club_id,body.user_id))
        .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    match previous{
        Some(row)=>{
            let mut active:club_member::ActiveModel=row.into();
            active.role=Set(body.role);active.update(&state.db).await
                .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        None=>{
            club_member::ActiveModel{
                club_id:Set(club_id),user_id:Set(body.user_id),
                role:Set(body.role),..Default::default()
            }.insert(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    };
    Ok(StatusCode::NO_CONTENT)
}
pub async fn assign_team(
 Extension(claims):Extension<Claims>,State(state):State<AppState>,
 Path(team_id):Path<Uuid>,Json(body):Json<Membership>,
)->Result<StatusCode,StatusCode>{
    let team=team::Entity::find_by_id(team_id).one(&state.db)
        .await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !matches!(club_visible(&state,claims.sub,team.club_id).await?,Role::Administrator|Role::Manager)
        {return Err(StatusCode::FORBIDDEN)}
    if !["reader","coach"].contains(&body.role.as_str()){return Err(StatusCode::BAD_REQUEST)}
    if user::Entity::find_by_id(body.user_id).one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.is_none(){
        return Err(StatusCode::NOT_FOUND)
    }
    let previous=team_member::Entity::find_by_id((team_id,body.user_id)).one(&state.db)
        .await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    match previous{
        Some(row)=>{
            let mut active:team_member::ActiveModel=row.into();
            active.role=Set(body.role);active.update(&state.db).await
               .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        None=>{
            team_member::ActiveModel{
                team_id:Set(team_id),user_id:Set(body.user_id),
                role:Set(body.role),..Default::default()
            }.insert(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    };
    Ok(StatusCode::NO_CONTENT)
}
