//! Maintainer-only lifecycle management. All destructive changes are atomic.
use axum::{Extension,Json,extract::{Path,State},http::StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,ColumnTrait,EntityTrait,QueryFilter,TransactionTrait};
use serde::{Deserialize,Serialize};
use uuid::Uuid;
use crate::{
    entities::{user,training,catalog_activity,tactical_diagram,match_game,ticket,ticket_message,
        player_user,club_membership,team_membership,user_preference,changelog_read,
        catalog_result,catalog_selection,catalog_material,training_activity,training_attendance,
        training_material,training_score,activity_player,audit_event},
    middleware::{CurrentRole,Role},routes::authentication::login::Claims,state::AppState,
};
type R<T>=Result<T,StatusCode>;
fn failure(e:sea_orm::DbErr)->StatusCode{
    tracing::error!(%e,"Account lifecycle operation failed");
    StatusCode::INTERNAL_SERVER_ERROR
}
fn permitted(role:Role,actor:Uuid,target:&user::Model)->R<()> {
    if role!=Role::Maintainer || actor==target.id {return Err(StatusCode::FORBIDDEN);}
    Ok(())
}
async fn target(state:&AppState,id:Uuid)->R<user::Model>{
    user::Entity::find_by_id(id).one(&state.db).await.map_err(failure)?
        .ok_or(StatusCode::NOT_FOUND)
}
#[derive(Serialize)]
pub struct AccountDependencies{
    pub trainings:u64,pub catalog_activities:u64,pub tactical_diagrams:u64,
    pub matches_created:u64,pub tickets:u64,pub ticket_messages:u64,
    pub linked_players:u64,pub club_memberships:u64,pub team_memberships:u64,
}
pub async fn dependencies(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,
)->R<Json<AccountDependencies>>{
    let subject=target(&state,id).await?;
    permitted(role.0,claims.sub,&subject)?;
    let db=&state.db;
    Ok(Json(AccountDependencies{
        trainings:training::Entity::find().filter(training::COLUMN.user_id.eq(id)).count(db).await.map_err(failure)?,
        catalog_activities:catalog_activity::Entity::find().filter(catalog_activity::COLUMN.user_id.eq(id)).count(db).await.map_err(failure)?,
        tactical_diagrams:tactical_diagram::Entity::find().filter(tactical_diagram::COLUMN.user_id.eq(id)).count(db).await.map_err(failure)?,
        matches_created:match_game::Entity::find().filter(match_game::COLUMN.created_by.eq(id)).count(db).await.map_err(failure)?,
        tickets:ticket::Entity::find().filter(ticket::COLUMN.author_id.eq(id)).count(db).await.map_err(failure)?,
        ticket_messages:ticket_message::Entity::find().filter(ticket_message::COLUMN.author_id.eq(id)).count(db).await.map_err(failure)?,
        linked_players:player_user::Entity::find().filter(player_user::COLUMN.user_id.eq(id)).count(db).await.map_err(failure)?,
        club_memberships:club_membership::Entity::find().filter(club_membership::COLUMN.user_id.eq(id)).count(db).await.map_err(failure)?,
        team_memberships:team_membership::Entity::find().filter(team_membership::COLUMN.user_id.eq(id)).count(db).await.map_err(failure)?,
    }))
}
#[derive(Deserialize)]
pub struct DisableRequest{pub disabled:bool}
pub async fn set_disabled(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(payload):Json<DisableRequest>,
)->R<StatusCode>{
    let row=target(&state,id).await?;
    permitted(role.0,claims.sub,&row)?;
    let mut active:user::ActiveModel=row.into();
    active.disabled=Set(Some(payload.disabled));
    active.update(&state.db).await.map_err(failure)?;
    tracing::warn!(actor=%claims.sub,target=%id,disabled=payload.disabled,"Account status changed");
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
#[serde(rename_all="snake_case")]
pub enum DeleteMode{Transfer,Delete}
#[derive(Deserialize)]
pub struct DeleteRequest{pub mode:DeleteMode,pub confirmation:String}
pub async fn remove(
    Extension(claims):Extension<Claims>,Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,Path(id):Path<Uuid>,Json(payload):Json<DeleteRequest>,
)->R<StatusCode>{
    let row=target(&state,id).await?;
    permitted(role.0,claims.sub,&row)?;
    if payload.confirmation!=row.username {return Err(StatusCode::BAD_REQUEST);}
    let destination=claims.sub;
    let tx=state.db.begin().await.map_err(failure)?;
    match payload.mode {
        DeleteMode::Transfer=>{
            // Preserve existing sporting history, keeping ownership with the acting maintainer.
            for item in training::Entity::find().filter(training::COLUMN.user_id.eq(id)).all(&tx).await.map_err(failure)?{
                let mut m:training::ActiveModel=item.into();m.user_id=Set(destination);m.update(&tx).await.map_err(failure)?;
            }
            for item in catalog_activity::Entity::find().filter(catalog_activity::COLUMN.user_id.eq(id)).all(&tx).await.map_err(failure)?{
                let mut m:catalog_activity::ActiveModel=item.into();m.user_id=Set(destination);m.update(&tx).await.map_err(failure)?;
            }
            for item in tactical_diagram::Entity::find().filter(tactical_diagram::COLUMN.user_id.eq(id)).all(&tx).await.map_err(failure)?{
                let mut m:tactical_diagram::ActiveModel=item.into();m.user_id=Set(destination);m.update(&tx).await.map_err(failure)?;
            }
            for item in match_game::Entity::find().filter(match_game::COLUMN.created_by.eq(id)).all(&tx).await.map_err(failure)?{
                let mut m:match_game::ActiveModel=item.into();m.created_by=Set(destination);m.update(&tx).await.map_err(failure)?;
            }
            for item in ticket::Entity::find().filter(ticket::COLUMN.author_id.eq(id)).all(&tx).await.map_err(failure)?{
                let mut m:ticket::ActiveModel=item.into();m.author_id=Set(destination);m.update(&tx).await.map_err(failure)?;
            }
            for item in ticket_message::Entity::find().filter(ticket_message::COLUMN.author_id.eq(id)).all(&tx).await.map_err(failure)?{
                let mut m:ticket_message::ActiveModel=item.into();m.author_id=Set(destination);m.update(&tx).await.map_err(failure)?;
            }
            // User-specific scoring and selection records cannot collide on transfer.
            // They are removed, while team/training results remain intact.
        },
        DeleteMode::Delete=>{
            // Remove dependent children first; never delete shared players, clubs or teams.
            for training in training::Entity::find().filter(training::COLUMN.user_id.eq(id)).all(&tx).await.map_err(failure)?{
                for activity in training_activity::Entity::find().filter(training_activity::COLUMN.training_id.eq(training.id)).all(&tx).await.map_err(failure)?{
                    training_material::Entity::delete_many().filter(training_material::COLUMN.activity_id.eq(activity.id)).exec(&tx).await.map_err(failure)?;
                    training_score::Entity::delete_many().filter(training_score::COLUMN.activity_id.eq(activity.id)).exec(&tx).await.map_err(failure)?;
                    training_activity::Entity::delete_by_id(activity.id).exec(&tx).await.map_err(failure)?;
                }
                training_attendance::Entity::delete_many().filter(training_attendance::COLUMN.training_id.eq(training.id)).exec(&tx).await.map_err(failure)?;
                training::Entity::delete_by_id(training.id).exec(&tx).await.map_err(failure)?;
            }
            for activity in catalog_activity::Entity::find().filter(catalog_activity::COLUMN.user_id.eq(id)).all(&tx).await.map_err(failure)?{
                // Other users' training sessions keep a historical snapshot, but no dangling catalog link.
                for linked in training_activity::Entity::find().filter(training_activity::COLUMN.catalog_id.eq(Some(activity.id))).all(&tx).await.map_err(failure)?{
                    let mut m:training_activity::ActiveModel=linked.into();m.catalog_id=Set(None);m.update(&tx).await.map_err(failure)?;
                }
                catalog_material::Entity::delete_many().filter(catalog_material::COLUMN.activity_id.eq(activity.id)).exec(&tx).await.map_err(failure)?;
                catalog_result::Entity::delete_many().filter(catalog_result::COLUMN.activity_id.eq(activity.id)).exec(&tx).await.map_err(failure)?;
                catalog_selection::Entity::delete_many().filter(catalog_selection::COLUMN.activity_id.eq(activity.id)).exec(&tx).await.map_err(failure)?;
                catalog_activity::Entity::delete_by_id(activity.id).exec(&tx).await.map_err(failure)?;
            }
            tactical_diagram::Entity::delete_many().filter(tactical_diagram::COLUMN.user_id.eq(id)).exec(&tx).await.map_err(failure)?;
            // A created match belongs to a team; preserve its history while transferring
            // creator attribution to the acting maintainer instead of destroying team stats.
            for item in match_game::Entity::find().filter(match_game::COLUMN.created_by.eq(id)).all(&tx).await.map_err(failure)?{
                let mut m:match_game::ActiveModel=item.into();m.created_by=Set(destination);m.update(&tx).await.map_err(failure)?;
            }
            for ticket in ticket::Entity::find().filter(ticket::COLUMN.author_id.eq(id)).all(&tx).await.map_err(failure)?{
                ticket_message::Entity::delete_many().filter(ticket_message::COLUMN.ticket_id.eq(ticket.id)).exec(&tx).await.map_err(failure)?;
                ticket::Entity::delete_by_id(ticket.id).exec(&tx).await.map_err(failure)?;
            }
            ticket_message::Entity::delete_many().filter(ticket_message::COLUMN.author_id.eq(id)).exec(&tx).await.map_err(failure)?;
        }
    }
    // Per-user joins and preferences are always removed. Shared players are not owned
    // exclusively by a user: deleting a user must not silently remove club rosters.
    player_user::Entity::delete_many().filter(player_user::COLUMN.user_id.eq(id)).exec(&tx).await.map_err(failure)?;
    club_membership::Entity::delete_many().filter(club_membership::COLUMN.user_id.eq(id)).exec(&tx).await.map_err(failure)?;
    team_membership::Entity::delete_many().filter(team_membership::COLUMN.user_id.eq(id)).exec(&tx).await.map_err(failure)?;
    user_preference::Entity::delete_many().filter(user_preference::COLUMN.user_id.eq(id)).exec(&tx).await.map_err(failure)?;
    changelog_read::Entity::delete_many().filter(changelog_read::COLUMN.user_id.eq(id)).exec(&tx).await.map_err(failure)?;
    catalog_result::Entity::delete_many().filter(catalog_result::COLUMN.user_id.eq(id)).exec(&tx).await.map_err(failure)?;
    catalog_selection::Entity::delete_many().filter(catalog_selection::COLUMN.user_id.eq(id)).exec(&tx).await.map_err(failure)?;
    // Preserve audit events as immutable history, but no longer associate
    // their actor with an active account.
    user::Entity::delete_by_id(id).exec(&tx).await.map_err(failure)?;
    tx.commit().await.map_err(failure)?;
    tracing::warn!(actor=%destination,target=%id,"Account deleted");
    Ok(StatusCode::NO_CONTENT)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_maintainers_can_mutate_other_accounts(){
        let subject=user::Model{id:Uuid::new_v4(),username:"alice".into(),
            password:"hash".into(),role:Some("user".into()),disabled:None};
        assert_eq!(permitted(Role::Administrator,Uuid::new_v4(),&subject),Err(StatusCode::FORBIDDEN));
        assert_eq!(permitted(Role::Maintainer,subject.id,&subject),Err(StatusCode::FORBIDDEN));
        assert!(permitted(Role::Maintainer,Uuid::new_v4(),&subject).is_ok());
    }
}
