use axum::http::StatusCode;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, QueryFilter, TransactionTrait, SqlErr};
use uuid::Uuid;

use crate::{entities, routes::{authentication::login::Claims, entities::{player::{CreatePlayerRequest, UpdatePlayerRequest},organization}}, state::AppState};

pub async fn get_players(
    claims: &Claims,
    state: &AppState,
) -> Result<Vec<entities::player::Model>, StatusCode> {
    let players_users = entities::player_user::Entity::find()
        .filter(entities::player_user::COLUMN.user_id.eq(claims.sub))
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Own legacy players plus team players visible to this account.
    // Team read permissions are checked individually, including club managers.
    let mut ids:std::collections::HashSet<Uuid>=
        players_users.into_iter().map(|pu|pu.player_id).collect();
    let role=organization::user_role(state,claims.sub).await?;
    if role==crate::middleware::Role::Administrator {
        return entities::player::Entity::find().all(&state.db)
            .await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR);
    }
    let all_clubs=entities::club_player::Entity::find().all(&state.db)
        .await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    for relation in all_clubs{
        if entities::club_membership::Entity::find_by_id((claims.sub,relation.club_id))
            .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.is_some(){
            ids.insert(relation.player_id);
        }
    }
    let memberships=entities::team_player::Entity::find().all(&state.db)
        .await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    for membership in memberships {
        if organization::can_read_team(state,claims,role,membership.team_id).await? {
            ids.insert(membership.player_id);
        }
    }
    let players=entities::player::Entity::find()
        .filter(entities::player::COLUMN.id.is_in(ids))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
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


async fn can_modify_player(claims:&Claims,state:&AppState,id:Uuid)->Result<bool,StatusCode>{
    let role=organization::user_role(state,claims.sub).await?;
    if !role.can_write(){return Ok(false);}
    if role==crate::middleware::Role::Administrator{return Ok(true);}
    // A shared player's identity belongs to its clubs/teams, not to the
    // person who initially entered the record. Every attached team must be
    // writable to avoid cross-team unauthorized edits.
    let teams=entities::team_player::Entity::find()
        .filter(entities::team_player::COLUMN.player_id.eq(id))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if !teams.is_empty(){
        for membership in teams{
            if !organization::can_write_team(state,claims,role,membership.team_id).await?{
                return Ok(false);
            }
        }
        return Ok(true);
    }
    let clubs=entities::club_player::Entity::find()
        .filter(entities::club_player::COLUMN.player_id.eq(id))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if !clubs.is_empty(){
        for association in clubs{
            if !organization::can_manage_club(state,claims,role,association.club_id).await?{
                return Ok(false);
            }
        }
        return Ok(true);
    }
    Ok(entities::player_user::Entity::find_by_id((claims.sub,id))
        .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?.is_some())
}

pub async fn update_player(
    claims: &Claims,
    state: &AppState,
    payload: UpdatePlayerRequest,
) -> Result<Uuid, StatusCode> {
    if !can_modify_player(claims,state,payload.id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    let Some(player) = get_player(claims, state, payload.id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };
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
        .map_err(|error| {
            // Keep the SQL details on the server, never in the HTTP response.
            // A constraint violation is a data conflict, not a server crash.
            tracing::error!(
                player_id=%payload.id,
                actor_id=%claims.sub,
                error=%error,
                "Unable to update player"
            );
            match error.sql_err() {
                Some(SqlErr::UniqueConstraintViolation(_)) |
                Some(SqlErr::ForeignKeyConstraintViolation(_)) => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(player.id)
}

pub async fn delete_player(
    claims: &Claims, state: &AppState, player_id: Uuid,
) -> Result<(), StatusCode> {
    use crate::entities::{training_attendance, training_score, activity_player, catalog_result};
    if !can_modify_player(claims,state,player_id).await?{
        return Err(StatusCode::FORBIDDEN);
    }
    let Some(player) = get_player(claims, state, player_id).await? else {
        return Err(StatusCode::NOT_FOUND);
    };
    // Shared player records must be explicitly unlinked from their teams
    // before deletion; deleting a team never deletes a player.
    let team_link=entities::team_player::Entity::find()
        .filter(entities::team_player::COLUMN.player_id.eq(player_id))
        .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let club_link=entities::club_player::Entity::find()
        .filter(entities::club_player::COLUMN.player_id.eq(player_id))
        .one(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    if team_link.is_some()||club_link.is_some(){return Err(StatusCode::CONFLICT);}
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
    entities::player_user::Entity::delete_many()
        .filter(entities::player_user::COLUMN.player_id.eq(player_id))
        .exec(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    entities::team_player::Entity::delete_many()
        .filter(entities::team_player::COLUMN.player_id.eq(player_id))
        .exec(&txn).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    entities::club_player::Entity::delete_many()
        .filter(entities::club_player::COLUMN.player_id.eq(player_id))
        .exec(&txn).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let active: entities::player::ActiveModel = player.into();
    active.delete(&txn).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    txn.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

#[cfg(test)]
mod update_tests {
    use super::*;
    use crate::{entities::{player,user},routes::authentication::login::Claims};
    use sea_orm::{Database,ConnectionTrait,Statement,DbBackend};

    async fn fresh_state()->AppState{
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        AppState{db}
    }
    async fn admin(state:&AppState)->Claims{
        let id=Uuid::new_v4();
        user::ActiveModel{
            id:Set(id),username:Set(format!("test-{id}")),
            password:Set("placeholder".into()),
            role:Set(Some("administrator".into())),..Default::default()
        }.insert(&state.db).await.unwrap();
        Claims{sub:id,exp:u64::MAX}
    }
    #[tokio::test]
    async fn two_players_can_share_a_first_name_and_edit_surnames(){
        let state=fresh_state().await;
        let claims=admin(&state).await;
        let first=create_player(&claims,&state,CreatePlayerRequest{
            firstname:"Emma".into(),lastname:"Durand".into(),number:7,
        }).await.unwrap();
        let second=create_player(&claims,&state,CreatePlayerRequest{
            firstname:"Emma".into(),lastname:"Martin".into(),number:10,
        }).await.unwrap();
        assert_ne!(first,second);
        let updated=update_player(&claims,&state,UpdatePlayerRequest{
            id:first,firstname:Some("Emma".into()),
            lastname:Some("Petit".into()),number:Some(7),
        }).await.unwrap();
        assert_eq!(updated,first);
        let after=get_player(&claims,&state,first).await.unwrap().unwrap();
        assert_eq!(after.firstname,"Emma");
        assert_eq!(after.lastname,"Petit");
        let unchanged=get_player(&claims,&state,second).await.unwrap().unwrap();
        assert_eq!(unchanged.lastname,"Martin");
    }
    #[tokio::test]
    async fn schema_sync_removes_obsolete_firstname_unique_index(){
        let state=fresh_state().await;
        // Reproduce the old SeaORM-generated unique index on an existing DB.
        state.db.execute(Statement::from_string(DbBackend::Sqlite,
            "CREATE UNIQUE INDEX \"idx-players-firstname\" ON \"players\" (\"firstname\")"
        )).await.unwrap();
        state.db.get_schema_registry("api::entities::*")
            .sync(&state.db).await.unwrap();
        let claims=admin(&state).await;
        create_player(&claims,&state,CreatePlayerRequest{
            firstname:"Lina".into(),lastname:"A".into(),number:3,
        }).await.unwrap();
        create_player(&claims,&state,CreatePlayerRequest{
            firstname:"Lina".into(),lastname:"B".into(),number:4,
        }).await.unwrap();
    }
}
