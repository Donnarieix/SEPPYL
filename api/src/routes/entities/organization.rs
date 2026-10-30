//! Club / team administration and scoped access checks.
//! Role changes require club managers or global administrators.
//! Team membership does not implicitly grant access to other teams.
use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use serde::{Deserialize,Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use crate::{
    entities::{club, team, club_membership, team_membership, team_player, club_player, player, player_user, user, training, catalog_activity},
    middleware::{CurrentRole, Role},
    routes::authentication::login::Claims,
    state::AppState,
};

type ApiResult<T> = Result<T, StatusCode>;
fn db_error(_: sea_orm::DbErr) -> StatusCode { StatusCode::INTERNAL_SERVER_ERROR }

pub async fn can_manage_club(state: &AppState, claims: &Claims, role: Role, id: Uuid) -> ApiResult<bool> {
    if role == Role::Administrator { return Ok(true); }
    if !role.can_write(){return Ok(false);}
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
    if !role.can_write(){return Ok(false);}
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
/// Global administration is separate from the clubs a user actively belongs to.
pub async fn list_admin_clubs(
    Extension(role):Extension<CurrentRole>,State(state):State<AppState>,
)->ApiResult<Json<Vec<club::Model>>>{
    if role.0!=Role::Administrator{return Err(StatusCode::FORBIDDEN);}
    Ok(Json(club::Entity::find().all(&state.db).await.map_err(db_error)?))
}
#[derive(Deserialize)]
pub struct ClubInformation {
    pub name:String,
    pub description:String,
    pub logo_url:Option<String>,
    #[serde(default)]pub logo_x:Option<i32>,
    #[serde(default)]pub logo_y:Option<i32>,
    #[serde(default)]pub logo_zoom:Option<i32>,
    pub is_public:bool,
}
fn valid_crop(x:Option<i32>,y:Option<i32>,zoom:Option<i32>)->bool {
    x.unwrap_or(0).abs()<=100 && y.unwrap_or(0).abs()<=100
        && (100..=300).contains(&zoom.unwrap_or(100))
}
fn valid_logo(url:Option<&str>)->bool {
    let Some(url)=url else{return true;};
    if url.is_empty(){return true;}
    if url.len()>2000{return false;}
    if url.starts_with("https://"){return true;}
    let Some(filename)=url.strip_prefix("/catalog/images/") else{return false;};
    let Some((identifier,extension))=filename.rsplit_once('.') else{return false;};
    Uuid::parse_str(identifier).is_ok() && matches!(extension,"png"|"jpg"|"webp"|"gif")
}
pub async fn update_club_information(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(body):Json<ClubInformation>,
)->ApiResult<Json<club::Model>>{
    if !can_manage_club(&state,&claims,role.0,id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    if !valid_name(&body.name)||body.description.len()>3000||
        !valid_logo(body.logo_url.as_deref())||
        !valid_crop(body.logo_x,body.logo_y,body.logo_zoom){
        return Err(StatusCode::BAD_REQUEST);
    }
    let row=club::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut active:club::ActiveModel=row.into();
    active.name=Set(body.name.trim().into());
    active.description=Set(if body.description.trim().is_empty(){None}
        else{Some(body.description.trim().to_owned())});
    active.logo_url=Set(body.logo_url.filter(|u|!u.is_empty()));
    active.logo_x=Set(Some(body.logo_x.unwrap_or(0)));
    active.logo_y=Set(Some(body.logo_y.unwrap_or(0)));
    active.logo_zoom=Set(Some(body.logo_zoom.unwrap_or(100)));
    active.is_public=Set(Some(body.is_public));
    let saved=active.update(&state.db).await.map_err(db_error)?;
    tracing::info!(actor=%claims.sub,club=%id,"Club information updated");
    Ok(Json(saved))
}
#[derive(Deserialize)]
pub struct TeamInformation {
    pub name:String,
    pub description:String,
    pub logo_url:Option<String>,
    #[serde(default)]pub logo_x:Option<i32>,
    #[serde(default)]pub logo_y:Option<i32>,
    #[serde(default)]pub logo_zoom:Option<i32>,
}
pub async fn update_team_information(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(body):Json<TeamInformation>
)->ApiResult<Json<team::Model>>{
    let old=team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !can_manage_club(&state,&claims,role.0,old.club_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }
    if !valid_name(&body.name)||body.description.len()>3000||
        !valid_logo(body.logo_url.as_deref())||
        !valid_crop(body.logo_x,body.logo_y,body.logo_zoom){
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut active:team::ActiveModel=old.into();
    active.name=Set(body.name.trim().to_string());
    active.description=Set(if body.description.trim().is_empty(){None}
        else{Some(body.description.trim().to_string())});
    active.logo_url=Set(body.logo_url.filter(|s|!s.is_empty()));
    active.logo_x=Set(Some(body.logo_x.unwrap_or(0)));
    active.logo_y=Set(Some(body.logo_y.unwrap_or(0)));
    active.logo_zoom=Set(Some(body.logo_zoom.unwrap_or(100)));
    Ok(Json(active.update(&state.db).await.map_err(db_error)?))
}

pub async fn create_club(
    Extension(claims): Extension<Claims>, Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>, Json(body): Json<CreateNamed>,
) -> ApiResult<(StatusCode,Json<club::Model>)> {
    if role.0 != Role::Administrator { return Err(StatusCode::FORBIDDEN); }
    if !valid_name(&body.name) { return Err(StatusCode::BAD_REQUEST); }
    let saved = club::ActiveModel {
        id: Set(Uuid::new_v4()), name: Set(body.name.trim().into()),
        is_public: Set(Some(false)),description:Set(None),logo_url:Set(None), ..Default::default()
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
    let direct_club_membership=club_membership::Entity::find_by_id((claims.sub,id))
        .one(&state.db).await.map_err(db_error)?.is_some();
    if role.0 == Role::Administrator || direct_club_membership {
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
        assert!(!can_write_team(&state,&coach,Role::User,team_id).await.unwrap());
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
    if club_membership::Entity::find_by_id((claims.sub,team.club_id))
        .one(&state.db).await.map_err(db_error)?.is_some(){return Ok(true);}
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
    let target=team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut known_in_club=false;
    let existing=team_player::Entity::find().filter(team_player::COLUMN.player_id.eq(player_id))
        .all(&state.db).await.map_err(db_error)?;
    for entry in existing{
        if let Some(source)=team::Entity::find_by_id(entry.team_id).one(&state.db).await.map_err(db_error)?{
            if source.club_id==target.club_id &&
                can_read_team(&state,&claims,role.0,source.id).await?{
                    known_in_club=true;break;
                }
        }
    }
    if !owner && !known_in_club && role.0!=Role::Administrator{
        return Err(StatusCode::NOT_FOUND);
    }
    if player::Entity::find_by_id(player_id).one(&state.db).await.map_err(db_error)?.is_none(){
        return Err(StatusCode::NOT_FOUND);
    }
    // Club membership persists even when the player leaves this team.
    if club_player::Entity::find_by_id((target.club_id,player_id))
        .one(&state.db).await.map_err(db_error)?.is_none(){
        club_player::ActiveModel{club_id:Set(target.club_id),player_id:Set(player_id),
            ..Default::default()}.insert(&state.db).await.map_err(db_error)?;
    }
    // An existing relationship is not a successful insertion. Report a conflict
    // and avoid producing an incorrect user notification or audit "create".
    if team_player::Entity::find_by_id((id,player_id)).one(&state.db)
        .await.map_err(db_error)?.is_some(){
        return Err(StatusCode::CONFLICT);
    }
    team_player::ActiveModel{team_id:Set(id),player_id:Set(player_id),..Default::default()}
        .insert(&state.db).await.map_err(|error|{
            tracing::warn!(%error,team=%id,player=%player_id,"Unable to assign player");
            if matches!(error.sql_err(),Some(sea_orm::SqlErr::UniqueConstraintViolation(_))){
                StatusCode::CONFLICT
            }else{StatusCode::INTERNAL_SERVER_ERROR}
        })?;
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

/* Only club managers and administrators can enumerate candidate accounts
   for membership management. UUIDs stay in API payloads, not the UI. */
pub async fn eligible_users(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,
)->ApiResult<Json<Vec<UserSummary>>>{
    if !can_manage_club(&state,&claims,role.0,id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    let users=user::Entity::find().all(&state.db).await.map_err(db_error)?;
    Ok(Json(users.into_iter().map(|u|UserSummary{
        id:u.id,username:u.username,
        role:u.role.unwrap_or_else(||"user".into()),
    }).collect()))
}

#[derive(Serialize)]
pub struct ClubCapabilities{
    pub can_manage:bool,
    pub can_create_team:bool,
    pub can_write_teams:Vec<Uuid>,
}
pub async fn capabilities(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,
)->ApiResult<Json<ClubCapabilities>>{
    if !can_access_club(&state,&claims,role.0,id).await?{
        return Err(StatusCode::NOT_FOUND);
    }
    let can_manage=can_manage_club(&state,&claims,role.0,id).await?;
    let teams=team::Entity::find().filter(team::COLUMN.club_id.eq(id))
        .all(&state.db).await.map_err(db_error)?;
    let mut can_write_teams=Vec::new();
    for team in teams{
        if role.0.can_write() && can_write_team(&state,&claims,role.0,team.id).await?{
            can_write_teams.push(team.id);
        }
    }
    Ok(Json(ClubCapabilities{can_manage,can_create_team:can_manage,can_write_teams}))
}

pub async fn rename_team(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(body):Json<CreateNamed>,
)->ApiResult<Json<team::Model>>{
    let original=team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !can_manage_club(&state,&claims,role.0,original.club_id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    if !valid_name(&body.name){return Err(StatusCode::BAD_REQUEST);}
    let mut active:team::ActiveModel=original.into();
    active.name=Set(body.name.trim().to_owned());
    let updated=active.update(&state.db).await.map_err(db_error)?;
    tracing::info!(actor=%claims.sub,team=%id,action="team.rename");
    Ok(Json(updated))
}
pub async fn delete_team(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,
)->ApiResult<StatusCode>{
    let original=team::Entity::find_by_id(id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !can_manage_club(&state,&claims,role.0,original.club_id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    // Preserve all historic sessions and shared catalogue definitions: refuse
    // destructive deletion when either points to the team.
    let training_exists=training::Entity::find()
        .filter(training::COLUMN.team_id.eq(id)).one(&state.db).await.map_err(db_error)?.is_some();
    let catalog_exists=catalog_activity::Entity::find()
        .filter(catalog_activity::COLUMN.team_id.eq(id)).one(&state.db).await.map_err(db_error)?.is_some();
    if training_exists||catalog_exists{return Err(StatusCode::CONFLICT);}
    let tx=state.db.begin().await.map_err(db_error)?;
    team_player::Entity::delete_many().filter(team_player::COLUMN.team_id.eq(id))
        .exec(&tx).await.map_err(db_error)?;
    team_membership::Entity::delete_many().filter(team_membership::COLUMN.team_id.eq(id))
        .exec(&tx).await.map_err(db_error)?;
    let active:team::ActiveModel=original.into();
    active.delete(&tx).await.map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    tracing::warn!(actor=%claims.sub,team=%id,action="team.delete");
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
pub struct NewTeamPlayer{pub firstname:String,pub lastname:String,pub number:u32}
pub async fn create_team_player(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(team_id):Path<Uuid>,Json(body):Json<NewTeamPlayer>,
)->ApiResult<(StatusCode,Json<player::Model>)>{
    if !role.0.can_write()||!can_write_team(&state,&claims,role.0,team_id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    if body.firstname.trim().is_empty()||body.lastname.trim().is_empty()
       ||body.firstname.len()>100||body.lastname.len()>100||body.number>999{
        return Err(StatusCode::BAD_REQUEST);
    }
    let target=team::Entity::find_by_id(team_id).one(&state.db).await.map_err(db_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let tx=state.db.begin().await.map_err(db_error)?;
    let saved=player::ActiveModel{
        id:Set(Uuid::new_v4()),firstname:Set(body.firstname.trim().to_owned()),
        lastname:Set(body.lastname.trim().to_owned()),number:Set(body.number),
        ..Default::default()
    }.insert(&tx).await.map_err(|error|{
        tracing::warn!(%error,"Unable to create team player");
        match error.sql_err(){
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))=>StatusCode::CONFLICT,
            _=>StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;
    club_player::ActiveModel{
        club_id:Set(target.club_id),player_id:Set(saved.id),..Default::default()
    }.insert(&tx).await.map_err(db_error)?;
    team_player::ActiveModel{
        team_id:Set(team_id),player_id:Set(saved.id),..Default::default()
    }.insert(&tx).await.map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    tracing::info!(actor=%claims.sub,team=%team_id,player=%saved.id,action="team.player.create");
    Ok((StatusCode::CREATED,Json(saved)))
}

pub async fn list_club_players(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>
)->ApiResult<Json<Vec<player::Model>>>{
    if !can_access_club(&state,&claims,role.0,id).await?{
        return Err(StatusCode::NOT_FOUND);
    }
    let direct=role.0==Role::Administrator ||
        club_membership::Entity::find_by_id((claims.sub,id))
            .one(&state.db).await.map_err(db_error)?.is_some();
    let all_teams=team::Entity::find()
        .filter(team::COLUMN.club_id.eq(id)).all(&state.db).await.map_err(db_error)?;
    let mut ids=HashSet::new();
    if direct{
        for link in club_player::Entity::find()
            .filter(club_player::COLUMN.club_id.eq(id))
            .all(&state.db).await.map_err(db_error)?{
                ids.insert(link.player_id);
        }
    }
    for entry in all_teams{
        if direct || can_read_team(&state,&claims,role.0,entry.id).await?{
            for link in team_player::Entity::find()
                .filter(team_player::COLUMN.team_id.eq(entry.id))
                .all(&state.db).await.map_err(db_error)?{
                ids.insert(link.player_id);
            }
        }
    }
    let rows=player::Entity::find().filter(player::COLUMN.id.is_in(ids))
        .all(&state.db).await.map_err(db_error)?;
    Ok(Json(rows))
}

#[cfg(test)]
mod roster_lifecycle_tests{
    use super::*;
    use sea_orm::Database;

    #[tokio::test]
    async fn team_player_is_shared_and_survives_empty_team_deletion(){
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        let coach=Uuid::new_v4();
        let teammate=Uuid::new_v4();
        let outsider=Uuid::new_v4();
        for (id,name,global) in [(coach,"coach","trainer"),(teammate,"reader","user"),(outsider,"outsider","user")]{
            user::ActiveModel{
                id:Set(id),username:Set(name.into()),password:Set("test".into()),
                role:Set(Some(global.into())),..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        let club_id=Uuid::new_v4();
        let team_id=Uuid::new_v4();
        club::ActiveModel{id:Set(club_id),name:Set("Club".into()),
            is_public:Set(Some(false)),..Default::default()}
            .insert(&state.db).await.unwrap();
        team::ActiveModel{id:Set(team_id),club_id:Set(club_id),
            name:Set("U17".into()),..Default::default()}
            .insert(&state.db).await.unwrap();
        team_membership::ActiveModel{user_id:Set(coach),team_id:Set(team_id),
            role:Set("trainer".into()),..Default::default()}
            .insert(&state.db).await.unwrap();
        club_membership::ActiveModel{user_id:Set(teammate),club_id:Set(club_id),
            role:Set("user".into()),..Default::default()}
            .insert(&state.db).await.unwrap();
        let coach_claims=Claims{sub:coach,exp:u64::MAX};
        let (_,created)=create_team_player(
            Extension(coach_claims),Extension(CurrentRole(Role::Trainer)),
            State(state.clone()),Path(team_id),Json(NewTeamPlayer{
                firstname:"Alice".into(),lastname:"Test".into(),number:7
            })
        ).await.unwrap();
        let id=created.0.id;
        let reader_claims=Claims{sub:teammate,exp:u64::MAX};
        let players=list_team_players(
            Extension(reader_claims.clone()),Extension(CurrentRole(Role::User)),
            State(state.clone()),Path(team_id)
        ).await.unwrap().0;
        assert_eq!(players.len(),1);
        assert_eq!(players[0].id,id);
        let denied=list_team_players(
            Extension(Claims{sub:outsider,exp:u64::MAX}),Extension(CurrentRole(Role::User)),
            State(state.clone()),Path(team_id)
        ).await;
        assert!(matches!(denied,Err(StatusCode::NOT_FOUND)));
        assert!(!can_write_team(&state,&reader_claims,Role::User,team_id).await.unwrap());
        // Removing a team must not delete the club's player record.
        // Grant club manager rights for the removal operation.
        club_membership::Entity::find_by_id((teammate,club_id))
            .one(&state.db).await.unwrap().unwrap();
        let row=club_membership::Entity::find_by_id((teammate,club_id))
            .one(&state.db).await.unwrap().unwrap();
        let mut active:club_membership::ActiveModel=row.into();
        active.role=Set("manager".into());
        active.update(&state.db).await.unwrap();
        let manager=Claims{sub:teammate,exp:u64::MAX};
        let result=rename_team(
            Extension(manager.clone()),Extension(CurrentRole(Role::Manager)),
            State(state.clone()),Path(team_id),Json(CreateNamed{name:"U18".into()})
        ).await.unwrap();
        assert_eq!(result.0.name,"U18");
        delete_team(Extension(manager.clone()),Extension(CurrentRole(Role::Manager)),
            State(state.clone()),Path(team_id)).await.unwrap();
        let remaining=list_club_players(
            Extension(manager),Extension(CurrentRole(Role::Manager)),
            State(state),Path(club_id)).await.unwrap().0;
        assert_eq!(remaining.len(),1);
        assert_eq!(remaining[0].id,id);
    }
}

#[cfg(test)]
mod club_information_regression {
    use super::*;
    use sea_orm::Database;
    #[tokio::test]
    async fn admin_inventory_is_separate_from_memberships_and_club_metadata_is_editable(){
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        let admin_id=Uuid::new_v4();let guest_id=Uuid::new_v4();
        for (id,username,role) in [(admin_id,"admin","administrator"),(guest_id,"guest","user")]{
            user::ActiveModel{id:Set(id),username:Set(username.into()),
                password:Set("test".into()),role:Set(Some(role.into())),..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        let admin=Claims{sub:admin_id,exp:u64::MAX};
        let guest=Claims{sub:guest_id,exp:u64::MAX};
        let role=CurrentRole(Role::Administrator);
        let (_,first)=create_club(Extension(admin.clone()),Extension(role),State(state.clone()),
            Json(CreateNamed{name:"Club A".into()})).await.unwrap();
        let (_,second)=create_club(Extension(admin.clone()),Extension(role),State(state.clone()),
            Json(CreateNamed{name:"Club B".into()})).await.unwrap();
        let all=list_admin_clubs(Extension(role),State(state.clone())).await.unwrap().0;
        assert_eq!(all.len(),2);
        let personal=list_clubs(Extension(admin.clone()),Extension(role),
            State(state.clone())).await.unwrap().0;
        assert!(personal.is_empty(),"Global admin rights are not club membership");
        club_membership::ActiveModel{
            user_id:Set(admin_id),club_id:Set(first.0.id),
            role:Set("manager".into()),..Default::default()
        }.insert(&state.db).await.unwrap();
        let personal=list_clubs(Extension(admin.clone()),Extension(role),
            State(state.clone())).await.unwrap().0;
        assert_eq!(personal.len(),1);
        assert_eq!(personal[0].id,first.0.id);
        let denied=list_admin_clubs(Extension(CurrentRole(Role::User)),
            State(state.clone())).await.err();
        assert_eq!(denied,Some(StatusCode::FORBIDDEN));
        let invalid=update_club_information(Extension(admin.clone()),Extension(role),
            State(state.clone()),Path(second.0.id),Json(ClubInformation{
                name:"B".into(),description:"A".into(),
                logo_url:Some("javascript:alert(1)".into()),is_public:true,
            })).await.err();
        assert_eq!(invalid,Some(StatusCode::BAD_REQUEST));
        let forbidden=update_club_information(Extension(guest),
            Extension(CurrentRole(Role::User)),State(state.clone()),
            Path(second.0.id),Json(ClubInformation{
                name:"Club B".into(),description:String::new(),logo_url:None,is_public:false,
            })).await.err();
        assert_eq!(forbidden,Some(StatusCode::FORBIDDEN));
        let Json(updated)=update_club_information(
            Extension(admin),Extension(role),State(state.clone()),
            Path(second.0.id),Json(ClubInformation{
                name:"Club B renommé".into(),
                description:"Club amateur et école de sport".into(),
                logo_url:Some("https://example.org/logo.png".into()),
                is_public:true,
            })
        ).await.unwrap();
        assert_eq!(updated.name,"Club B renommé");
        assert_eq!(updated.description.as_deref(),Some("Club amateur et école de sport"));
        assert_eq!(updated.logo_url.as_deref(),Some("https://example.org/logo.png"));
        assert_eq!(updated.is_public,Some(true));
        assert_eq!(public_clubs(State(state)).await.unwrap().0.len(),1);
    }
    #[test]
    fn only_approved_logo_urls_are_accepted(){
        assert!(valid_logo(None));
        assert!(valid_logo(Some("https://example.com/logo.webp")));
        assert!(valid_logo(Some(&format!("/catalog/images/{}.png",Uuid::new_v4()))));
        for url in ["javascript:alert(1)","data:image/svg+xml;base64,AA","/catalog/images/../secret","http://example.com/test.png"]{
            assert!(!valid_logo(Some(url)));
        }
    }
}
