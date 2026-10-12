use axum::http::StatusCode;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

use crate::{entities, routes::{authentication::login::Claims, entities::player::{CreatePlayerRequest, UpdatePlayerRequest}}, state::AppState};

pub async fn get_players(
    claims: &Claims,
    state: &AppState,
) -> Result<Vec<entities::player::Model>, StatusCode> {
    let players_users = entities::player_user::Entity::find()
        .filter(entities::player_user::COLUMN.user_id.eq(claims.sub))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let players = entities::player::Entity::find()
        .filter(entities::player::COLUMN.id.is_in(
            players_users
                .iter()
                .map(|pu| pu.player_id)
        ))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(players)
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
    let Some(mut player) = get_player(claims, state, payload.id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };

    if let Some(firstname) = payload.firstname {
        player.firstname = firstname;
    }
    if let Some(lastname) = payload.lastname {
        player.lastname = lastname;
    }
    if let Some(number) = payload.number {
        player.number = number;
    }

    let player: entities::player::ActiveModel = player.into();
    
    let player = player.update(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(player.id)
}

pub async fn delete_player(
    claims: &Claims,
    state: &AppState,
    player_id: Uuid,
) -> Result<(), StatusCode> {
    let Some(player) = get_player(claims, state, player_id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };

    let player: entities::player::ActiveModel = player.into();
    
    player.delete(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}