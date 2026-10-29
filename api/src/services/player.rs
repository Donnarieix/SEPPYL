use axum::http::StatusCode;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

use crate::{access, entities, routes::{authentication::login::Claims, entities::player::{CreatePlayerRequest, UpdatePlayerRequest}}, state::AppState};

pub async fn get_players(
    claims:&Claims,state:&AppState
)->Result<Vec<entities::player::Model>,StatusCode>{
    let teams=access::teams_for(state,claims.sub,false).await?;
    let team_links=if teams.is_empty(){Vec::new()}else{
        entities::team_player::Entity::find()
            .filter(entities::team_player::COLUMN.team_id.is_in(teams))
            .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
    };
    let own_links=entities::player_user::Entity::find()
        .filter(entities::player_user::COLUMN.user_id.eq(claims.sub))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut ids:std::collections::HashSet<Uuid>=team_links.into_iter().map(|x|x.player_id).collect();
    // Unmapped legacy records are readable by their original user until
    // migration assigns them to a club/team. Mapped records use memberships.
    for owner in own_links {
        if entities::team_player::Entity::find()
            .filter(entities::team_player::COLUMN.player_id.eq(owner.player_id))
            .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.is_none(){
            ids.insert(owner.player_id);
        }
    }
    if ids.is_empty(){return Ok(Vec::new())}
    entities::player::Entity::find()
        .filter(entities::player::COLUMN.id.is_in(ids))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)
}
pub async fn get_players_for_team(
    claims:&Claims,state:&AppState,team:Option<Uuid>,
)->Result<Vec<entities::player::Model>,StatusCode>{
    let Some(team_id)=team else{return get_players(claims,state).await};
    access::assert_team(state,claims.sub,team_id,false).await?;
    let links=entities::team_player::Entity::find()
        .filter(entities::team_player::COLUMN.team_id.eq(team_id))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if links.is_empty(){return Ok(Vec::new())}
    entities::player::Entity::find()
        .filter(entities::player::COLUMN.id.is_in(links.into_iter().map(|r|r.player_id)))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)
}
async fn assert_can_write_player(
    claims:&Claims,state:&AppState,player_id:Uuid
)->Result<(),StatusCode>{
    let memberships=entities::team_player::Entity::find()
        .filter(entities::team_player::COLUMN.player_id.eq(player_id))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if memberships.is_empty(){
        let owner=entities::player_user::Entity::find_by_id((claims.sub,player_id))
            .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
        return if owner.is_some(){Ok(())}else{Err(StatusCode::FORBIDDEN)}
    }
    let writable=access::teams_for(state,claims.sub,true).await?;
    if memberships.iter().any(|m|writable.contains(&m.team_id)){Ok(())}
    else{Err(StatusCode::FORBIDDEN)}
}

pub async fn get_player(
    claims: &Claims,
    state: &AppState,
    player_id: Uuid,
) -> Result<Option<entities::player::Model>, StatusCode> {
    let players = get_players(claims, state).await?;
    Ok(players.into_iter().find(|p| p.id == player_id))
}

pub async fn create_player(
    claims: &Claims,
    state: &AppState,
    payload: CreatePlayerRequest,
) -> Result<Uuid, StatusCode> {
    let team_id=access::default_writable_team(state,claims.sub).await?;
    let txn = state.db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let player = entities::player::ActiveModel {
        id: Set(Uuid::new_v4()),
        firstname: Set(payload.firstname),
        lastname: Set(payload.lastname),
        number: Set(payload.number),
    };

    let player = player
        .insert(&txn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let player_user = entities::player_user::ActiveModel {
        player_id: Set(player.id),
        user_id: Set(claims.sub),
    };

    let _player_user = player_user
        .insert(&txn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    entities::team_player::ActiveModel{
        team_id:Set(team_id),player_id:Set(player.id),..Default::default()
    }.insert(&txn).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    txn.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(player.id)
}

pub async fn update_player(
    claims: &Claims,
    state: &AppState,
    payload: UpdatePlayerRequest,
) -> Result<Uuid, StatusCode> {
    let Some(player) = get_player(claims, state, payload.id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };
    assert_can_write_player(claims,state,payload.id).await?;
    let mut player: entities::player::ActiveModel = player.into();
    if let Some(firstname) = payload.firstname {
        if firstname.trim().is_empty() { return Err(StatusCode::BAD_REQUEST); }
        player.firstname = Set(firstname.trim().to_string());
    }
    if let Some(lastname) = payload.lastname {
        if lastname.trim().is_empty() { return Err(StatusCode::BAD_REQUEST); }
        player.lastname = Set(lastname.trim().to_string());
    }
    if let Some(number) = payload.number { player.number = Set(number); }
    
    let player = player.update(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(player.id)
}

pub async fn delete_player(
    claims: &Claims, state: &AppState, player_id: Uuid,
) -> Result<(), StatusCode> {
    use crate::entities::{training_attendance, training_score, activity_player, catalog_result};
    let Some(player) = get_player(claims, state, player_id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };
    assert_can_write_player(claims,state,player_id).await?;
    // Preserve history instead of deleting recorded evaluations.
    let attendance = training_attendance::Entity::find()
        .filter(training_attendance::COLUMN.player_id.eq(player_id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let scores = training_score::Entity::find()
        .filter(training_score::COLUMN.player_id.eq(player_id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let legacy = activity_player::Entity::find()
        .filter(activity_player::COLUMN.player_id.eq(player_id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let results = catalog_result::Entity::find()
        .filter(catalog_result::COLUMN.player_id.eq(player_id))
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if attendance.is_some() || scores.is_some() || legacy.is_some() || results.is_some() {
        return Err(StatusCode::CONFLICT);
    }
    let txn = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    entities::team_player::Entity::delete_many()
        .filter(entities::team_player::COLUMN.player_id.eq(player_id))
        .exec(&txn).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    entities::player_user::Entity::delete_many()
        .filter(entities::player_user::COLUMN.player_id.eq(player_id))
        .exec(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let active: entities::player::ActiveModel = player.into();
    active.delete(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    txn.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}
