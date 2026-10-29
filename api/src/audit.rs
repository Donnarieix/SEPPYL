//! Safe, human-readable audit snapshots. The scope is captured before mutations,
//! so deleting an entity cannot silently erase its club association.
use axum::http::{Method,StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,EntityTrait};
use serde_json::Value;
use std::time::{SystemTime,UNIX_EPOCH};
use uuid::Uuid;
use crate::{entities::{audit_event,club,team,training,catalog_activity,player,user},state::AppState};

#[derive(Clone,Debug)]
pub struct Context {
    pub scope: &'static str,
    pub club_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub description: String,
    rename_from: Option<String>,
}
impl Context {
    fn admin(description:String)->Self {
        Self{scope:"admin",club_id:None,team_id:None,description,rename_from:None}
    }
    fn personal(description:String)->Self {
        Self{scope:"personal",club_id:None,team_id:None,description,rename_from:None}
    }
    fn club(id:Uuid,description:String)->Self {
        Self{scope:"club",club_id:Some(id),team_id:None,description,rename_from:None}
    }
    fn team(id:Uuid,club_id:Uuid,description:String)->Self {
        Self{scope:"team",club_id:Some(club_id),team_id:Some(id),description,rename_from:None}
    }
}

fn operation(method:&Method)->&'static str {
    match *method {
        Method::POST=>"Création",Method::PUT|Method::PATCH=>"Modification",
        Method::DELETE=>"Suppression",_=>"Opération"
    }
}
fn quoted(value:&str)->String { format!("« {value} »") }
fn payload_team(payload:Option<&Value>)->Option<Uuid> {
    payload?.get("team_id")?.as_str().and_then(|v|Uuid::parse_str(v).ok())
}
pub async fn capture(
    state:&AppState, method:&Method, path:&str, payload:Option<&Value>,
)->Context{
    let parts:Vec<&str>=path.trim_matches('/').split('/').collect();
    let generic=format!("{} de données",operation(method));
    match parts.as_slice(){
        ["clubs",id,tail @ ..]=>{
            let Ok(id)=Uuid::parse_str(id) else {return Context::admin(generic)};
            let Ok(Some(c))=club::Entity::find_by_id(id).one(&state.db).await
                else{return Context::admin(generic)};
            let name=quoted(&c.name);
            let desc=match *tail {
                ["teams"]=>format!("{} d'une équipe dans le club {name}",operation(method)),
                ["members",user_id]=>{
                    let person=Uuid::parse_str(user_id).ok();
                    let username=if let Some(uid)=person {
                        user::Entity::find_by_id(uid).one(&state.db).await.ok()
                            .flatten().map(|u|u.username)
                    }else{None};
                    let account=username.map(|u|quoted(&u)).unwrap_or_else(||"un compte".into());
                    format!("{} des accès de {account} au club {name}",operation(method))
                },
                ["visibility"]=>format!("Modification de la visibilité du club {name}"),
                _=>format!("{} du club {name}",operation(method)),
            };
            Context::club(id,desc)
        }
        ["teams",id,tail @ ..]=>{
            let Ok(id)=Uuid::parse_str(id) else{return Context::admin(generic)};
            let Ok(Some(t))=team::Entity::find_by_id(id).one(&state.db).await
                else{return Context::admin(generic)};
            let name=quoted(&t.name);
            let desc=match *tail {
                [] if *method==Method::PUT=>format!("Renommage de l'équipe {name}"),
                [] if *method==Method::DELETE=>format!("Suppression de l'équipe {name}"),
                ["players",player_id]=>{
                    let person=Uuid::parse_str(player_id).ok();
                    let player=if let Some(uid)=person{
                        player::Entity::find_by_id(uid).one(&state.db).await.ok().flatten()
                    }else{None};
                    let who=player.map(|p|quoted(&format!("{} {}",p.firstname,p.lastname)))
                        .unwrap_or_else(||"une joueuse".into());
                    format!("{} de {who} dans l'équipe {name}",if *method==Method::DELETE{"Retrait"}else{"Ajout"})
                },
                ["players"]=>format!("Création d'une joueuse dans l'équipe {name}"),
                ["members",user_id]=>{
                    let username=if let Ok(uid)=Uuid::parse_str(user_id){
                        user::Entity::find_by_id(uid).one(&state.db).await.ok()
                            .flatten().map(|u|u.username)
                    }else{None};
                    let account=username.map(|v|quoted(&v)).unwrap_or_else(||"un compte".into());
                    format!("{} des accès de {account} à l'équipe {name}",operation(method))
                },
                _=>format!("{} dans l'équipe {name}",operation(method))
            };
            let mut context=Context::team(id,t.club_id,desc);
            if tail.is_empty() && *method==Method::PUT{context.rename_from=Some(t.name);}
            context
        }
        ["trainings",id,..]=>{
            let Ok(id)=Uuid::parse_str(id) else{return Context::admin(generic)};
            let Ok(Some(t))=training::Entity::find_by_id(id).one(&state.db).await
                else{return Context::admin(generic)};
            let desc=format!("{} de la séance {}",operation(method),quoted(&t.name));
            scoped_team_or_personal(state,t.team_id,desc).await
        }
        ["catalog",id,..]=>{
            let Ok(id)=Uuid::parse_str(id) else{return Context::admin(generic)};
            let Ok(Some(c))=catalog_activity::Entity::find_by_id(id).one(&state.db).await
                else{return Context::admin(generic)};
            let desc=format!("{} de l'activité {}",operation(method),quoted(&c.name));
            scoped_team_or_personal(state,c.team_id,desc).await
        }
        ["trainings"]|["catalog"]=>{
            // Creation payloads explicitly carry team_id; missing team_id is personal.
            // If a client did not provide a parseable payload, do not guess a scope.
            let Some(body)=payload else{return Context::admin(generic)};
            if !body.is_object(){return Context::admin(generic)}
            let subject=if parts[0]=="trainings"{"séance"}else{"activité"};
            let name=body.get("name").and_then(Value::as_str).unwrap_or("sans nom");
            let desc=format!("Création de la {subject} {}",quoted(name));
            scoped_team_or_personal(state,payload_team(Some(body)),desc).await
        }
        ["account",..]=>Context::personal("Modification du compte personnel".into()),
        ["preferences",..]=>Context::personal("Modification des préférences".into()),
        ["tactics",..]=>Context::personal(format!("{} d'un schéma tactique",operation(method))),
        ["activity",..]=>Context::personal(format!("{} d'une activité personnelle",operation(method))),
        ["matrix",..]=>Context::personal(format!("{} du tableur personnel",operation(method))),
        ["admin",..]=>Context::admin(format!("{} dans l'administration",operation(method))),
        // Old player routes can mutate club-shared data without a scope in their URL.
        // They stay admin-only until those handlers emit an explicit scoped event.
        _=>Context::admin(generic),
    }
}
async fn scoped_team_or_personal(state:&AppState,team_id:Option<Uuid>,desc:String)->Context{
    let Some(id)=team_id else{return Context::personal(desc)};
    match team::Entity::find_by_id(id).one(&state.db).await {
        Ok(Some(t))=>Context::team(id,t.club_id,desc),
        _=>Context::admin(desc),
    }
}
pub async fn record(
    state:&AppState,actor:Uuid,method:&Method,path:&str,status:StatusCode,
    mut ctx:Context,
){
    if status.is_success(){
        if let (Some(previous),Some(id))=(ctx.rename_from.as_ref(),ctx.team_id){
            if let Ok(Some(updated))=team::Entity::find_by_id(id).one(&state.db).await{
                ctx.description=format!("Renommage de l'équipe {} en {}",
                    quoted(previous),quoted(&updated.name));
            }
        }
    }
    let action=match *method{
        Method::POST=>"create", Method::PUT|Method::PATCH=>"update",
        Method::DELETE=>"delete",_=>"other",
    };
    let timestamp=SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_secs() as i64;
    let row=audit_event::ActiveModel{
        id:Set(Uuid::new_v4()),actor_id:Set(actor),action:Set(action.into()),
        path:Set(path.into()),status:Set(i32::from(status.as_u16())),
        timestamp:Set(timestamp),scope:Set(Some(ctx.scope.into())),
        club_id:Set(ctx.club_id),team_id:Set(ctx.team_id),
        description:Set(Some(ctx.description)),..Default::default()
    };
    if let Err(error)=row.insert(&state.db).await{
        tracing::error!(%error,"Could not persist audit log");
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn action_labels_and_payloads(){
        assert_eq!(operation(&Method::POST),"Création");
        assert_eq!(operation(&Method::DELETE),"Suppression");
        assert_eq!(payload_team(Some(&serde_json::json!({"team_id":"invalid"}))),None);
    }
}
