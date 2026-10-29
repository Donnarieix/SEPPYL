//! Club / team administration and scoped access checks.
//! Role changes require club managers or global administrators.
//! Team membership does not implicitly grant access to other teams.
use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize,Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use crate::{
    entities::{club, team, club_membership, team_membership, team_player, player, player_user, user},
    middleware::{CurrentRole, Role},
    routes::authentication::login::Claims,
    state::AppState,
};

type ApiResult<T> = Result<T, StatusCode>;
fn db_error(_: sea_orm::DbErr) -> StatusCode { StatusCode::INTERNAL_SERVER_ERROR }

pub async fn can_manage_club(state: &AppState, claims: &Claims, role: Role, id: Uuid) -> ApiResult<bool> {
    if role == Role::Administrator { return Ok(true); }
    let membership = club_membership::Entity::find_by_id((claims.sub, id))
        .one(&state.db).await.map_err(db_error)?;
    Ok(membership.is_some_and(|m| m.role == "manager"))
}
pub async fn can_access_club(state: &AppState, claims: &Claims, role: Role, id: Uuid) -> ApiResult<bool> {
    if role == Role::Administrator { return Ok(true); }
    if club_membership::Entity::find_by_id((claims.sub, id))
        .one(&state.db).await.map_err(db_error)?.is_some() { return Ok(true); }
    let teams = team::Entity::find().filter(team::COLUMN.club_id.eq(id))
        .all(&state.db).await.map_err(db_error)?;
    if teams.is_empty() { return Ok(false); }
    Ok(team_membership::Entity::find()
        .filter(team_membership::COLUMN.user_id.eq(claims.sub))
        .filter(team_membership::COLUMN.team_id.is_in(teams.iter().map(|t|t.id)))
        .one(&state.db).await.map_err(db_error)?.is_some())
}
pub async fn can_write_team(state: &AppState, claims: &Claims, role: Role, id: Uuid) -> ApiResult<bool> {
    if role == Role::Administrator { return Ok(true); }
    let Some(team) = team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)? else {
        return Ok(false);
    };
    if can_manage_club(state, claims, role, team.club_id).await? { return Ok(true); }
    let membership = team_membership::Entity::find_by_id((claims.sub,id))
        .one(&state.db).await.map_err(db_error)?;
    Ok(membership.is_some_and(|m| m.role == "trainer"))
}

#[derive(Deserialize)]
pub struct CreateNamed { name: String }
#[derive(Deserialize)]
pub struct GrantMembership { role: String }
#[derive(Deserialize)]
pub struct GrantGlobalRole { role: String }

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 120
}

pub async fn list_clubs(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<club::Model>>> {
    let all = club::Entity::find().all(&state.db).await.map_err(db_error)?;
    if role.0 == Role::Administrator { return Ok(Json(all)); }
    let club_ids:HashSet<Uuid> = club_membership::Entity::find()
        .filter(club_membership::COLUMN.user_id.eq(claims.sub))
        .all(&state.db).await.map_err(db_error)?
        .iter().map(|m|m.club_id).collect();
    let team_ids:HashSet<Uuid> = team_membership::Entity::find()
        .filter(team_membership::COLUMN.user_id.eq(claims.sub))
        .all(&state.db).await.map_err(db_error)?
        .iter().map(|m|m.team_id).collect();
    let indirect:HashSet<Uuid> = team::Entity::find().all(&state.db).await.map_err(db_error)?
        .iter().filter(|t| team_ids.contains(&t.id)).map(|t|t.club_id).collect();
    Ok(Json(all.into_iter().filter(|c|club_ids.contains(&c.id)||indirect.contains(&c.id)).collect()))
}
pub async fn create_club(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Json(body): Json<CreateNamed>,
) -> ApiResult<(StatusCode,Json<club::Model>)> {
    if role.0 != Role::Administrator { return Err(StatusCode::FORBIDDEN); }
    if !valid_name(&body.name) { return Err(StatusCode::BAD_REQUEST); }
    let saved = club::ActiveModel {
        id: Set(Uuid::new_v4()), name: Set(body.name.trim().into()),
        is_public: Set(Some(false)), ..Default::default()
    }.insert(&state.db).await.map_err(db_error)?;
    tracing::info!(actor=%claims.sub, club=%saved.id, action="club.create");
    Ok((StatusCode::CREATED, Json(saved)))
}
pub async fn list_teams(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<team::Model>>> {
    if !can_access_club(&state,&claims,role.0,id).await? { return Err(StatusCode::NOT_FOUND); }
    let all = team::Entity::find().filter(team::COLUMN.club_id.eq(id))
        .all(&state.db).await.map_err(db_error)?;
    if role.0 == Role::Administrator || can_manage_club(&state,&claims,role.0,id).await? {
        return Ok(Json(all));
    }
    let assigned:HashSet<Uuid> = team_membership::Entity::find()
        .filter(team_membership::COLUMN.user_id.eq(claims.sub))
        .all(&state.db).await.map_err(db_error)?
        .iter().map(|m|m.team_id).collect();
    Ok(Json(all.into_iter().filter(|t|assigned.contains(&t.id)).collect()))
}
pub async fn create_team(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path(id): Path<Uuid>, Json(body): Json<CreateNamed>,
) -> ApiResult<(StatusCode,Json<team::Model>)> {
    if !can_manage_club(&state,&claims,role.0,id).await? { return Err(StatusCode::FORBIDDEN); }
    if !valid_name(&body.name) { return Err(StatusCode::BAD_REQUEST); }
    if club::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let saved=team::ActiveModel {
        id:Set(Uuid::new_v4()),club_id:Set(id),name:Set(body.name.trim().into()),..Default::default()
    }.insert(&state.db).await.map_err(db_error)?;
    tracing::info!(actor=%claims.sub, team=%saved.id, club=%id, action="team.create");
    Ok((StatusCode::CREATED,Json(saved)))
}
pub async fn grant_club_membership(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path((id,user_id)): Path<(Uuid,Uuid)>,
    Json(body): Json<GrantMembership>,
) -> ApiResult<StatusCode> {
    if !can_manage_club(&state,&claims,role.0,id).await? { return Err(StatusCode::FORBIDDEN); }
    if !matches!(body.role.as_str(),"user"|"manager") { return Err(StatusCode::BAD_REQUEST); }
    if user::Entity::find_by_id(user_id).one(&state.db).await.map_err(db_error)?.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing=club_membership::Entity::find_by_id((user_id,id))
        .one(&state.db).await.map_err(db_error)?;
    match existing {
        Some(m)=>{
            let mut active:club_membership::ActiveModel=m.into();
            active.role=Set(body.role);
            active.update(&state.db).await.map_err(db_error)?;
        }
        None=>{
            club_membership::ActiveModel {
                user_id:Set(user_id),club_id:Set(id),role:Set(body.role),..Default::default()
            }.insert(&state.db).await.map_err(db_error)?;
        }
    }
    tracing::info!(actor=%claims.sub, member=%user_id, club=%id, action="club.member.grant");
    Ok(StatusCode::NO_CONTENT)
}
pub async fn grant_team_membership(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path((id,user_id)): Path<(Uuid,Uuid)>,
    Json(body): Json<GrantMembership>,
) -> ApiResult<StatusCode> {
    let t=team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !can_manage_club(&state,&claims,role.0,t.club_id).await? { return Err(StatusCode::FORBIDDEN); }
    if !matches!(body.role.as_str(),"user"|"trainer") { return Err(StatusCode::BAD_REQUEST); }
    if user::Entity::find_by_id(user_id).one(&state.db).await.map_err(db_error)?.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let existing=team_membership::Entity::find_by_id((user_id,id))
        .one(&state.db).await.map_err(db_error)?;
    match existing {
        Some(m)=>{
            let mut active:team_membership::ActiveModel=m.into();
            active.role=Set(body.role);
            active.update(&state.db).await.map_err(db_error)?;
        }
        None=>{
            team_membership::ActiveModel {
                user_id:Set(user_id),team_id:Set(id),role:Set(body.role),..Default::default()
            }.insert(&state.db).await.map_err(db_error)?;
        }
    }
    tracing::info!(actor=%claims.sub, member=%user_id, team=%id, action="team.member.grant");
    Ok(StatusCode::NO_CONTENT)
}
pub async fn grant_global_role(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path(user_id): Path<Uuid>,
    Json(body): Json<GrantGlobalRole>,
) -> ApiResult<StatusCode> {
    if role.0 != Role::Administrator { return Err(StatusCode::FORBIDDEN); }
    if !matches!(body.role.as_str(),"user"|"trainer"|"manager"|"administrator") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let user=user::Entity::find_by_id(user_id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut active:user::ActiveModel=user.into();
    active.role=Set(Some(body.role));
    active.update(&state.db).await.map_err(db_error)?;
    tracing::warn!(actor=%claims.sub, target=%user_id, action="role.global.change");
    Ok(StatusCode::NO_CONTENT)
}


#[cfg(test)]
mod integration_tests {
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn club_and_team_memberships_are_scoped() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        let admin_id=Uuid::new_v4();
        let trainer_id=Uuid::new_v4();
        let stranger_id=Uuid::new_v4();
        for (id,name,role) in [(admin_id,"admin","administrator"),(trainer_id,"coach","user"),(stranger_id,"other","user")] {
            user::ActiveModel{
                id:Set(id),username:Set(name.into()),password:Set("test".into()),
                role:Set(Some(role.into())),..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        let admin=Claims{sub:admin_id,exp:u64::MAX};
        let coach=Claims{sub:trainer_id,exp:u64::MAX};
        let stranger=Claims{sub:stranger_id,exp:u64::MAX};
        let denied=create_club(Extension(coach.clone()),Extension(CurrentRole(Role::User)),
            State(state.clone()),Json(CreateNamed{name:"Club".into()})).await;
        assert!(matches!(denied,Err(StatusCode::FORBIDDEN)));
        let (_,club)=create_club(Extension(admin.clone()),Extension(CurrentRole(Role::Administrator)),
            State(state.clone()),Json(CreateNamed{name:"Club".into()})).await.unwrap();
        let club_id=club.0.id;
        let (_,team)=create_team(Extension(admin.clone()),Extension(CurrentRole(Role::Administrator)),
            State(state.clone()),Path(club_id),Json(CreateNamed{name:"Équipe U16".into()})).await.unwrap();
        let team_id=team.0.id;
        assert!(!can_access_club(&state,&stranger,Role::User,club_id).await.unwrap());
        grant_team_membership(
            Extension(admin.clone()),Extension(CurrentRole(Role::Administrator)),State(state.clone()),
            Path((team_id,trainer_id)),Json(GrantMembership{role:"trainer".into()})
        ).await.unwrap();
        assert!(can_access_club(&state,&coach,Role::User,club_id).await.unwrap());
        assert!(can_write_team(&state,&coach,Role::User,team_id).await.unwrap());
        assert!(!can_write_team(&state,&stranger,Role::User,team_id).await.unwrap());
        let visible=list_clubs(Extension(coach),Extension(CurrentRole(Role::User)),State(state)).await.unwrap().0;
        assert_eq!(visible.len(),1);
    }
}

#[derive(Serialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}
pub async fn list_users(
    Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<UserSummary>>> {
    if role.0 != Role::Administrator { return Err(StatusCode::FORBIDDEN); }
    let users = user::Entity::find().all(&state.db).await.map_err(db_error)?;
    Ok(Json(users.into_iter().map(|u| UserSummary {
        id: u.id, username: u.username,
        role: u.role.unwrap_or_else(|| "trainer".into()),
    }).collect()))
}
pub async fn list_club_members(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<club_membership::Model>>> {
    if !can_manage_club(&state, &claims, role.0, id).await? {
        return Err(StatusCode::FORBIDDEN);
    }
    let rows = club_membership::Entity::find()
        .filter(club_membership::COLUMN.club_id.eq(id))
        .all(&state.db).await.map_err(db_error)?;
    Ok(Json(rows))
}
pub async fn list_team_members(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<team_membership::Model>>> {
    let t=team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !can_manage_club(&state,&claims,role.0,t.club_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }
    let rows=team_membership::Entity::find()
        .filter(team_membership::COLUMN.team_id.eq(id))
        .all(&state.db).await.map_err(db_error)?;
    Ok(Json(rows))
}
pub async fn remove_club_member(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path((id,user_id)): Path<(Uuid,Uuid)>,
) -> ApiResult<StatusCode> {
    if !can_manage_club(&state,&claims,role.0,id).await? {
        return Err(StatusCode::FORBIDDEN);
    }
    if let Some(row)=club_membership::Entity::find_by_id((user_id,id))
        .one(&state.db).await.map_err(db_error)? {
        let active:club_membership::ActiveModel=row.into();
        active.delete(&state.db).await.map_err(db_error)?;
    }
    tracing::warn!(actor=%claims.sub, target=%user_id, club=%id, action="club.member.remove");
    Ok(StatusCode::NO_CONTENT)
}
pub async fn remove_team_member(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Path((id,user_id)): Path<(Uuid,Uuid)>,
) -> ApiResult<StatusCode> {
    let t=team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !can_manage_club(&state,&claims,role.0,t.club_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }
    if let Some(row)=team_membership::Entity::find_by_id((user_id,id))
        .one(&state.db).await.map_err(db_error)? {
        let active:team_membership::ActiveModel=row.into();
        active.delete(&state.db).await.map_err(db_error)?;
    }
    tracing::warn!(actor=%claims.sub,target=%user_id,team=%id,action="team.member.remove");
    Ok(StatusCode::NO_CONTENT)
}


pub async fn user_role(state:&AppState, id:Uuid) -> ApiResult<Role> {
    let user=user::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    Role::parse(user.role.as_deref().unwrap_or("trainer"))
        .ok_or(StatusCode::FORBIDDEN)
}
pub async fn can_read_team(
    state: &AppState, claims: &Claims, role: Role, id: Uuid
) -> ApiResult<bool> {
    if role==Role::Administrator {return Ok(true);}
    let Some(team)=team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
    else {return Ok(false)};
    if can_manage_club(state,claims,role,team.club_id).await? {return Ok(true);}
    Ok(team_membership::Entity::find_by_id((claims.sub,id))
        .one(&state.db).await.map_err(db_error)?.is_some())
}

pub async fn list_team_players(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,
)->ApiResult<Json<Vec<player::Model>>>{
    if !can_read_team(&state,&claims,role.0,id).await?{
        return Err(StatusCode::NOT_FOUND);
    }
    let ids=team_player::Entity::find()
        .filter(team_player::COLUMN.team_id.eq(id))
        .all(&state.db).await.map_err(db_error)?
        .into_iter().map(|entry|entry.player_id).collect::<Vec<_>>();
    let result=player::Entity::find()
        .filter(player::COLUMN.id.is_in(ids))
        .all(&state.db).await.map_err(db_error)?;
    Ok(Json(result))
}
pub async fn assign_team_player(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path((id,player_id)):Path<(Uuid,Uuid)>,
)->ApiResult<StatusCode>{
    if !role.0.can_write() || !can_write_team(&state,&claims,role.0,id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    let owner=player_user::Entity::find_by_id((claims.sub,player_id))
        .one(&state.db).await.map_err(db_error)?.is_some();
    // An administrator may migrate an existing player; ordinary coaches
    // can only attach their own players or already-linked team players.
    if !owner && role.0!=Role::Administrator{return Err(StatusCode::NOT_FOUND);}
    if player::Entity::find_by_id(player_id).one(&state.db).await.map_err(db_error)?.is_none(){
        return Err(StatusCode::NOT_FOUND);
    }
    if team_player::Entity::find_by_id((id,player_id)).one(&state.db).await.map_err(db_error)?.is_none(){
        team_player::ActiveModel{team_id:Set(id),player_id:Set(player_id),..Default::default()}
            .insert(&state.db).await.map_err(db_error)?;
    }
    tracing::info!(actor=%claims.sub,team=%id,player=%player_id,action="team.player.add");
    Ok(StatusCode::NO_CONTENT)
}
pub async fn remove_team_player(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path((id,player_id)):Path<(Uuid,Uuid)>,
)->ApiResult<StatusCode>{
    if !role.0.can_write() || !can_write_team(&state,&claims,role.0,id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    if let Some(row)=team_player::Entity::find_by_id((id,player_id))
        .one(&state.db).await.map_err(db_error)?{
        let active:team_player::ActiveModel=row.into();
        active.delete(&state.db).await.map_err(db_error)?;
    }
    tracing::info!(actor=%claims.sub,team=%id,player=%player_id,action="team.player.remove");
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_team_players_internal(state:&AppState,id:Uuid)->ApiResult<Vec<player::Model>>{
    let ids=team_player::Entity::find().filter(team_player::COLUMN.team_id.eq(id))
        .all(&state.db).await.map_err(db_error)?
        .into_iter().map(|m|m.player_id).collect::<Vec<_>>();
    player::Entity::find().filter(player::COLUMN.id.is_in(ids))
        .all(&state.db).await.map_err(db_error)
}

#[derive(Deserialize)]
pub struct SetPublic {pub is_public:bool}
pub async fn update_public_visibility(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(body):Json<SetPublic>,
)->ApiResult<StatusCode>{
    if !can_manage_club(&state,&claims,role.0,id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    let club=club::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut active:club::ActiveModel=club.into();
    active.is_public=Set(Some(body.is_public));
    active.update(&state.db).await.map_err(db_error)?;
    tracing::warn!(actor=%claims.sub,club=%id,is_public=body.is_public,action="club.visibility.update");
    Ok(StatusCode::NO_CONTENT)
}
pub async fn public_clubs(State(state):State<AppState>)
    ->ApiResult<Json<Vec<club::Model>>>{
    let all=club::Entity::find().all(&state.db).await.map_err(db_error)?;
    Ok(Json(all.into_iter().filter(|c|c.is_public==Some(true)).collect()))
}
pub async fn public_teams(State(state):State<AppState>,Path(id):Path<Uuid>)
    ->ApiResult<Json<Vec<team::Model>>>{
    let club=club::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if club.is_public!=Some(true){return Err(StatusCode::NOT_FOUND);}
    let teams=team::Entity::find()
        .filter(team::COLUMN.club_id.eq(id)).all(&state.db).await.map_err(db_error)?;
    Ok(Json(teams))
}


#[cfg(test)]
mod public_visibility_tests {
    use super::*;
    use sea_orm::Database;
    #[tokio::test]
    async fn clubs_are_private_by_default_and_publication_is_explicit() {
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        let admin=Uuid::new_v4();
        user::ActiveModel {
            id:Set(admin),username:Set("admin".into()),password:Set("test".into()),
            role:Set(Some("administrator".into())),..Default::default()
        }.insert(&state.db).await.unwrap();
        let claims=Claims{sub:admin,exp:u64::MAX};
        let (_,club)=create_club(
            Extension(claims.clone()),Extension(CurrentRole(Role::Administrator)),
            State(state.clone()),Json(CreateNamed{name:"Example".into()})
        ).await.unwrap();
        let id=club.0.id;
        assert!(public_clubs(State(state.clone())).await.unwrap().0.is_empty());
        update_public_visibility(
            Extension(claims),Extension(CurrentRole(Role::Administrator)),
            State(state.clone()),Path(id),Json(SetPublic{is_public:true})
        ).await.unwrap();
        let visible=public_clubs(State(state)).await.unwrap().0;
        assert_eq!(visible.len(),1);
        assert_eq!(visible[0].id,id);
    }
}
