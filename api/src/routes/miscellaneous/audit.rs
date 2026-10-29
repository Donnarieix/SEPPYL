use axum::{Extension,Json,extract::{State,Query},http::StatusCode};
use sea_orm::{EntityTrait,QueryOrder,PaginatorTrait,QuerySelect,QueryFilter,ColumnTrait,Condition};
use serde::{Deserialize,Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::{
    entities::{audit_event,user,club_membership,team_membership},
    middleware::{CurrentRole,Role},
    routes::authentication::login::Claims,
    state::AppState,
};

#[derive(Debug,Deserialize)]
pub struct AuditQuery{
    #[serde(default="default_page")] page:u64,
    #[serde(default="default_size")] page_size:u64,
}
fn default_page()->u64{1}
fn default_size()->u64{50}
fn accepted_size(value:u64)->bool{matches!(value,10|50|100|250|500|1000)}

#[derive(Serialize)]
pub struct AuditEvent {
    pub id:Uuid,
    pub author:String,
    pub action:String,
    pub description:String,
    pub status:i32,
    pub status_label:String,
    pub timestamp:i64,
}
#[derive(Serialize)]
pub struct AuditPage {
    pub items:Vec<AuditEvent>,
    pub total:u64,
    pub page:u64,
    pub page_size:u64,
    pub total_pages:u64,
}
pub async fn list(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,
    Query(query):Query<AuditQuery>,
)->Result<Json<AuditPage>,StatusCode>{
    if !role.0.can_write(){return Err(StatusCode::FORBIDDEN);}
    if query.page==0 || !accepted_size(query.page_size){return Err(StatusCode::BAD_REQUEST);}

    let mut selection=audit_event::Entity::find();
    if role.0!=Role::Administrator{
        // Never filter after pagination: totals, page boundaries and the
        // presence of other clubs' events must remain undiscoverable.
        // Scope-less historical entries are excluded for non-admins.
        let mut condition=Condition::any().add(
            Condition::all()
                .add(audit_event::COLUMN.scope.eq("personal"))
                .add(audit_event::COLUMN.actor_id.eq(claims.sub))
        );
        let manager_clubs=club_membership::Entity::find()
            .filter(club_membership::COLUMN.user_id.eq(claims.sub))
            .filter(club_membership::COLUMN.role.eq("manager"))
            .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter().map(|m|m.club_id).collect::<Vec<_>>();
        if !manager_clubs.is_empty(){
            condition=condition.add(
                Condition::all().add(audit_event::COLUMN.scope.eq("club"))
                    .add(audit_event::COLUMN.club_id.is_in(manager_clubs.clone()))
            ).add(
                Condition::all().add(audit_event::COLUMN.scope.eq("team"))
                    .add(audit_event::COLUMN.club_id.is_in(manager_clubs))
            );
        }
        let trainer_teams=team_membership::Entity::find()
            .filter(team_membership::COLUMN.user_id.eq(claims.sub))
            .filter(team_membership::COLUMN.role.eq("trainer"))
            .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter().map(|m|m.team_id).collect::<Vec<_>>();
        if !trainer_teams.is_empty(){
            condition=condition.add(
                Condition::all().add(audit_event::COLUMN.scope.eq("team"))
                    .add(audit_event::COLUMN.team_id.is_in(trainer_teams))
            );
        }
        selection=selection.filter(condition);
    }
    let total=selection.clone().count(&state.db).await
        .map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?;
    let total_pages=if total==0{0}else{1+(total-1)/query.page_size};
    let rows=if query.page>total_pages{Vec::new()}else{
        selection.order_by_desc(audit_event::COLUMN.timestamp)
            .order_by_desc(audit_event::COLUMN.id)
            .offset((query.page-1)*query.page_size).limit(query.page_size)
            .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
    };
    let authors=user::Entity::find()
        .filter(user::COLUMN.id.is_in(rows.iter().map(|r|r.actor_id).collect::<Vec<_>>()))
        .all(&state.db).await.map_err(|_|StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter().map(|u|(u.id,u.username)).collect::<HashMap<_,_>>();
    let items=rows.into_iter().map(|r|{
        let status_label=StatusCode::from_u16(r.status as u16).ok()
            .and_then(|s|s.canonical_reason()).unwrap_or("Statut inconnu").to_owned();
        AuditEvent{
            id:r.id,
            author:authors.get(&r.actor_id).cloned()
                .unwrap_or_else(||"Compte supprimé".into()),
            action:r.action,
            description:r.description
                .unwrap_or_else(||"Opération historique (détails indisponibles)".into()),
            status:r.status,status_label,timestamp:r.timestamp,
        }
    }).collect();
    Ok(Json(AuditPage{items,total,page:query.page,page_size:query.page_size,total_pages}))
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn only_supported_page_sizes(){
        for n in [10,50,100,250,500,1000]{assert!(accepted_size(n));}
        for n in [0,1,25,1001,u64::MAX]{assert!(!accepted_size(n));}
    }
}
