//! Club / team administration and scoped access checks.
//! Role changes require club managers or global administrators.
//! Team membership does not implicitly grant access to other teams.
use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use std::collections::HashSet;
use uuid::Uuid;
use crate::{
    entities::{club, team, club_membership, team_membership, user},
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
        id: Set(Uuid::new_v4()), name: Set(body.name.trim().into()), ..Default::default()
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
