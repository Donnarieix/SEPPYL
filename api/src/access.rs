//! Server-side authorization, independent of future Donnarieix SSO.
//! Role membership is always read from the database; JWTs contain identity only.
use axum::http::StatusCode;
use sea_orm::{EntityTrait, QueryFilter};
use uuid::Uuid;
use crate::{entities::{user, club_member, team_member},state::AppState};

#[derive(Clone,Copy,Debug,Eq,PartialEq)]
pub enum Role { Visitor, User, Coach, Manager, Administrator }
impl Role {
    pub fn can_read(self) -> bool { self != Self::Visitor }
    pub fn can_write(self) -> bool { matches!(self,Self::Coach|Self::Manager|Self::Administrator) }
}

pub async fn effective_role(state:&AppState, subject:Uuid)->Result<Role,StatusCode> {
    let Some(account)=user::Entity::find_by_id(subject).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)? else {return Err(StatusCode::UNAUTHORIZED)};
    if account.global_role.as_deref()==Some("administrator") { return Ok(Role::Administrator); }
    let clubs=club_member::Entity::find().filter(club_member::COLUMN.user_id.eq(subject))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if clubs.iter().any(|m|m.role=="manager"){return Ok(Role::Manager)}
    let teams=team_member::Entity::find().filter(team_member::COLUMN.user_id.eq(subject))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if teams.iter().any(|m|m.role=="coach"){return Ok(Role::Coach)}
    Ok(Role::User)
}

pub async fn club_role(state:&AppState, subject:Uuid,club_id:Uuid)->Result<Role,StatusCode>{
    if effective_role(state,subject).await?==Role::Administrator{return Ok(Role::Administrator)}
    let m=club_member::Entity::find_by_id((club_id,subject)).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(match m.map(|x|x.role).as_deref(){Some("manager")=>Role::Manager,Some("reader")=>Role::User,_=>Role::Visitor})
}

pub async fn team_role(state:&AppState, subject:Uuid,team_id:Uuid,club_id:Uuid)->Result<Role,StatusCode>{
    let club=club_role(state,subject,club_id).await?;
    if matches!(club,Role::Administrator|Role::Manager){return Ok(club)}
    let team=team_member::Entity::find_by_id((team_id,subject)).one(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(match team.map(|x|x.role).as_deref(){Some("coach")=>Role::Coach,Some("reader")=>Role::User,_=>club})
}

#[cfg(test)]
mod tests{
 use super::*;
 #[test]fn only_coaches_and_above_may_write(){
  assert!(!Role::Visitor.can_read());assert!(Role::User.can_read());
  assert!(!Role::User.can_write());
  assert!(Role::Coach.can_write());assert!(Role::Manager.can_write());assert!(Role::Administrator.can_write());
 }
}

pub async fn teams_for(state:&AppState,subject:Uuid,write:bool)->Result<Vec<Uuid>,StatusCode>{
    use crate::entities::team;
    let role=effective_role(state,subject).await?;
    if role==Role::Administrator{
        return Ok(team::Entity::find().all(&state.db).await
            .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter().map(|x|x.id).collect());
    }
    let memberships=team_member::Entity::find().filter(team_member::COLUMN.user_id.eq(subject))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let clubs=club_member::Entity::find().filter(club_member::COLUMN.user_id.eq(subject))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let allowed_clubs:std::collections::HashSet<Uuid>=clubs.into_iter()
        .filter(|c|!write || c.role=="manager").map(|c|c.club_id).collect();
    let club_teams=if allowed_clubs.is_empty(){Vec::new()}else{
        team::Entity::find().filter(team::COLUMN.club_id.is_in(allowed_clubs))
            .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter().map(|t|t.id).collect::<Vec<_>>()
    };
    let mut ids:std::collections::HashSet<Uuid>=club_teams.into_iter().collect();
    ids.extend(memberships.into_iter().filter(|m|!write || m.role=="coach").map(|m|m.team_id));
    Ok(ids.into_iter().collect())
}
pub async fn assert_team(state:&AppState,subject:Uuid,team_id:Uuid,write:bool)->Result<(),StatusCode>{
    if teams_for(state,subject,write).await?.contains(&team_id){Ok(())}
    else{Err(StatusCode::FORBIDDEN)}
}
pub async fn default_writable_team(state:&AppState,subject:Uuid)->Result<Uuid,StatusCode>{
    let mut available=teams_for(state,subject,true).await?;
    available.sort_unstable();
    available.into_iter().next().ok_or(StatusCode::FORBIDDEN)
}
