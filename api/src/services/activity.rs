use axum::http::StatusCode;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

use crate::{entities, routes::{authentication::login::Claims, entities::activity::{CreateActivityRequest, UpdateActivityRequest, UpdatePlayerActivityRequest}}, services::player::get_players, state::AppState};

pub async fn get_player_activities(
    claims: &Claims,
    state: &AppState,
    player_id: Uuid,
) -> Result<Vec<entities::activity::Model>, StatusCode> {
    let players = get_players(claims, state).await?;
    if players.iter().all(|p| p.id != player_id) {
        return Err(StatusCode::FORBIDDEN);
    }

    let activities_players = entities::activity_player::Entity::find()
        .filter(entities::activity_player::COLUMN.player_id.eq(player_id))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let activities = entities::activity::Entity::find()
        .filter(entities::activity::COLUMN.id.is_in(
            activities_players
                .iter()
                .map(|ap| ap.activity_id)
        ))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(activities)
}

pub async fn get_activities(
    claims: &Claims,
    state: &AppState,
) -> Result<Vec<entities::activity::Model>, StatusCode> {
    let players = get_players(claims, state).await?;
    if players.is_empty() {
        return Err(StatusCode::FORBIDDEN);
    }

    let activities_players = entities::activity_player::Entity::find()
        .filter(entities::activity_player::COLUMN.player_id.is_in(
            players.iter().map(|p| p.id)
        ))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let activities = entities::activity::Entity::find()
        .filter(entities::activity::COLUMN.id.is_in(
            activities_players.iter().map(|ap| ap.activity_id)
        ))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(activities)
}

pub async fn get_activity(
    claims: &Claims,
    state: &AppState,
    activity_id: Uuid,
) -> Result<Option<entities::activity::Model>, StatusCode> {
    if !has_rights(claims, state, activity_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let activities = entities::activity::Entity::find()
        .filter(entities::activity::COLUMN.id.eq(activity_id))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(activities.into_iter().next())
}

pub async fn has_rights(
    claims: &Claims,
    state: &AppState,
    activity_id: Uuid,
) -> Result<bool, StatusCode> {
    let user_players = get_players(claims, state).await?;
    let activity_players = get_activity_players(state, activity_id).await?;
    if activity_players.iter().all(|p| !user_players.iter().any(|up| up.id == p.id)) {
        return Ok(false);
    }
    Ok(true)
}

pub async fn get_player_activity(
    claims: &Claims,
    state: &AppState,
    player_id: Uuid,
    activity_id: Uuid,
) -> Result<Option<entities::activity::Model>, StatusCode> {
    let activities = get_player_activities(claims, state, player_id).await?;
    Ok(activities.into_iter().find(|a| a.id == activity_id))
}

pub async fn get_player_activity_player(
    claims: &Claims,
    state: &AppState,
    player_id: Uuid,
    activity_id: Uuid,
) -> Result<Option<entities::activity_player::Model>, StatusCode> {
    if !has_rights(claims, state, activity_id).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let activity_players = entities::activity_player::Entity::find()
        .filter(entities::activity_player::COLUMN.player_id.eq(player_id))
        .filter(entities::activity_player::COLUMN.activity_id.eq(activity_id))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(activity_players.into_iter().next())
}

pub async fn get_activity_players(
    state: &AppState,
    activity_id: Uuid,
) -> Result<Vec<entities::player::Model>, StatusCode> {
    let activity_players = entities::activity_player::Entity::find()
        .filter(entities::activity_player::COLUMN.activity_id.eq(activity_id))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let player_ids: Vec<Uuid> = activity_players.iter().map(|ap| ap.player_id).collect();

    let players = entities::player::Entity::find()
        .filter(entities::player::COLUMN.id.is_in(player_ids))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(players)
}

pub async fn create_activity(
    claims: &Claims,
    state: &AppState,
    payload: CreateActivityRequest,
) -> Result<Uuid, StatusCode> {
    let players = get_players(claims, state).await?;
    if players.iter().all(|p| p.id != payload.player_id) {
        return Err(StatusCode::FORBIDDEN);
    }

    let txn = state.db
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let activity = entities::activity::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(payload.name),
        date: Set(payload.date),
    };

    let activity = activity
        .insert(&txn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let activity_player = entities::activity_player::ActiveModel {
        activity_id: Set(activity.id),
        player_id: Set(payload.player_id),
        result: Set(payload.result)
    };

    let _activity_player = activity_player
        .insert(&txn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    txn.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(activity.id)
}

pub async fn update_activity(
    claims: &Claims,
    state: &AppState,
    payload: UpdateActivityRequest,
) -> Result<Uuid, StatusCode> {
    if !has_rights(claims, state, payload.id).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let Some(mut activity) = get_activity(claims, state, payload.id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };

    if let Some(name) = payload.name {
        activity.name = name;
    }
    if let Some(date) = payload.date {
        activity.date = date;
    }

    let activity: entities::activity::ActiveModel = activity.into();
    
    let activity = activity.update(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(activity.id)
}

pub async fn update_activity_player(
    claims: &Claims,
    state: &AppState,
    payload: UpdatePlayerActivityRequest,
) -> Result<(), StatusCode> {
    let players = get_players(claims, state).await?;
    if players.iter().all(|p| p.id != payload.player_id) {
        return Err(StatusCode::FORBIDDEN);
    }

    let Some(mut activity_player) = get_player_activity_player(claims, state, payload.player_id, payload.activity_id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };

    activity_player.result = payload.result;

    let activity_player: entities::activity_player::ActiveModel = activity_player.into();
    
    activity_player.update(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}

pub async fn delete_activity(
    claims: &Claims,
    state: &AppState,
    activity_id: Uuid,
) -> Result<(), StatusCode> {
    let Some(activity) = get_activity(claims, state, activity_id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };

    let activity: entities::activity::ActiveModel = activity.into();
    
    activity.delete(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(())
}