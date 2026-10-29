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
    #[tokio::test]
    async fn audit_visibility_tracks_live_write_permissions_and_paginates_after_filter(){
        use sea_orm::{Database,ActiveModelTrait,ActiveValue::Set};
        use crate::entities::{club,team};
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state=AppState{db};
        let admin_id=Uuid::new_v4();
        let coach_id=Uuid::new_v4();
        let manager_id=Uuid::new_v4();
        let foreign_id=Uuid::new_v4();
        for (id,name,role) in [
            (admin_id,"admin","administrator"),(coach_id,"coach","trainer"),
            (manager_id,"manager","manager"),(foreign_id,"other","manager")
        ]{
            user::ActiveModel{id:Set(id),username:Set(name.into()),password:Set("test".into()),
                role:Set(Some(role.into())),..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        let club_a=Uuid::new_v4();let club_b=Uuid::new_v4();
        let team_a=Uuid::new_v4();let team_b=Uuid::new_v4();
        for (id,name) in [(club_a,"Club A"),(club_b,"Club B")]{
            club::ActiveModel{id:Set(id),name:Set(name.into()),is_public:Set(Some(false)),
                ..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        for (id,cid) in [(team_a,club_a),(team_b,club_b)]{
            team::ActiveModel{id:Set(id),club_id:Set(cid),name:Set("Équipe".into()),
                ..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        club_membership::ActiveModel{user_id:Set(manager_id),club_id:Set(club_a),
            role:Set("manager".into()),..Default::default()
        }.insert(&state.db).await.unwrap();
        team_membership::ActiveModel{user_id:Set(coach_id),team_id:Set(team_a),
            role:Set("trainer".into()),..Default::default()
        }.insert(&state.db).await.unwrap();
        for (actor,scope,club_id,team_id,description) in [
            (admin_id,Some("club"),Some(club_a),None,"Club A"),
            (admin_id,Some("team"),Some(club_a),Some(team_a),"Équipe A"),
            (admin_id,Some("team"),Some(club_b),Some(team_b),"Équipe B"),
            (coach_id,Some("personal"),None,None,"Mon schéma"),
            (admin_id,None,None,None,"Ancien événement")
        ]{
            audit_event::ActiveModel{
                id:Set(Uuid::new_v4()),actor_id:Set(actor),action:Set("update".into()),
                path:Set("/example".into()),status:Set(200),timestamp:Set(1),
                scope:Set(scope.map(str::to_owned)),club_id:Set(club_id),team_id:Set(team_id),
                description:Set(Some(description.into())),..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        let query=||Query(AuditQuery{page:1,page_size:10});
        let coach=Claims{sub:coach_id,exp:u64::MAX};
        let manager=Claims{sub:manager_id,exp:u64::MAX};
        let admin=Claims{sub:admin_id,exp:u64::MAX};
        let coach_events=list(Extension(coach.clone()),Extension(CurrentRole(Role::Trainer)),
            State(state.clone()),query()).await.unwrap().0;
        assert_eq!(coach_events.total,2);
        assert!(coach_events.items.iter().all(|e|e.description=="Équipe A"||e.description=="Mon schéma"));
        let manager_events=list(Extension(manager),Extension(CurrentRole(Role::Manager)),
            State(state.clone()),query()).await.unwrap().0;
        assert_eq!(manager_events.total,2);
        assert!(manager_events.items.iter().all(|e|e.description=="Club A"||e.description=="Équipe A"));
        assert_eq!(list(Extension(coach),Extension(CurrentRole(Role::User)),
            State(state.clone()),query()).await.err(),Some(StatusCode::FORBIDDEN));
        let admin_events=list(Extension(admin),Extension(CurrentRole(Role::Administrator)),
            State(state.clone()),query()).await.unwrap().0;
        assert_eq!(admin_events.total,5);
        // Membership is checked per request, not frozen at event creation.
        team_membership::Entity::delete_by_id((coach_id,team_a))
            .exec(&state.db).await.unwrap();
        let after=list(Extension(Claims{sub:coach_id,exp:u64::MAX}),
            Extension(CurrentRole(Role::Trainer)),State(state),query()).await.unwrap().0;
        assert_eq!(after.total,1);
        assert_eq!(after.items[0].description,"Mon schéma");
    }
}
