//! Safe, human-readable audit snapshots. The scope is captured before mutations,
//! so deleting an entity cannot silently erase its club association.
use axum::http::{Method,StatusCode};
use sea_orm::{ActiveModelTrait,ActiveValue::Set,EntityTrait,QueryFilter};
use serde_json::Value;
use std::time::{SystemTime,UNIX_EPOCH};
use uuid::Uuid;
use crate::{entities::{audit_event,club,team,training,catalog_activity,player,user,club_player,team_player,training_activity,catalog_material,training_material,tactical_diagram,activity},state::AppState};

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
fn body_string<'a>(payload:Option<&'a Value>,key:&str)->Option<&'a str>{
    payload?.get(key)?.as_str()
}
fn id(value:&str)->Option<Uuid>{Uuid::parse_str(value).ok()}
fn body_id(payload:Option<&Value>,key:&str)->Option<Uuid>{
    body_string(payload,key).and_then(id)
}
fn label_player(p:&player::Model)->String{
    format!("{} {} (n° {})",p.firstname,p.lastname,p.number)
}
fn change_text(old:&str,new:Option<&str>,field:&str)->Option<String>{
    new.filter(|value|*value!=old)
        .map(|value|format!("{field} : {} → {}",quoted(old),quoted(value)))
}
async fn scope_player(state:&AppState,actor:Uuid,pid:Uuid,description:String)->Context{
    let relations=club_player::Entity::find()
        .filter(club_player::COLUMN.player_id.eq(pid)).all(&state.db).await;
    if let Ok(mut links)=relations{
        links.sort_by_key(|l|l.club_id);
        links.dedup_by_key(|l|l.club_id);
        if links.len()==1 {
            let club_id=links[0].club_id;
            if let Ok(teams)=team_player::Entity::find()
                .filter(team_player::COLUMN.player_id.eq(pid)).all(&state.db).await {
                if teams.len()==1 {
                    if let Ok(Some(t))=team::Entity::find_by_id(teams[0].team_id).one(&state.db).await{
                        return Context::team(t.id,t.club_id,description);
                    }
                }
            }
            return Context::club(club_id,description);
        }
        if links.len()>1{return Context::admin(description);}
    }
    // Personal players only appear to their owner in the audit feed;
    // a missing/ambiguous scope must never leak across clubs.
    let _=actor;
    Context::personal(description)
}
fn payload_team(payload:Option<&Value>)->Option<Uuid> {
    payload?.get("team_id")?.as_str().and_then(|v|Uuid::parse_str(v).ok())
}
pub async fn capture(
    state:&AppState, actor:Uuid, method:&Method, path:&str, payload:Option<&Value>,
)->Context{
    let parts:Vec<&str>=path.trim_matches('/').split('/').collect();
    let generic=format!("{} d'une opération non catégorisée",operation(method));
    match parts.as_slice(){
        ["player","create"]=>{
            let who=match (body_string(payload,"firstname"),body_string(payload,"lastname")){
                (Some(first),Some(last))=>quoted(&format!("{first} {last}")),
                _=>"une joueuse".into()
            };
            let n=payload.and_then(|v|v.get("number")).and_then(Value::as_u64);
            Context::personal(format!("Création de la fiche de {who}{}",n.map(|v|format!(" (n° {v})")).unwrap_or_default()))
        }
        ["player","update"]=>{
            let Some(pid)=body_id(payload,"id") else{
                return Context::admin("Modification d'une fiche de joueuse (identité non déterminée)".into())
            };
            let Ok(Some(p))=player::Entity::find_by_id(pid).one(&state.db).await else{
                return Context::admin("Modification d'une fiche de joueuse introuvable".into())
            };
            let old=label_player(&p);
            let mut differences=Vec::new();
            if let Some(text)=change_text(&p.firstname,body_string(payload,"firstname"),"prénom"){differences.push(text);}
            if let Some(text)=change_text(&p.lastname,body_string(payload,"lastname"),"nom"){differences.push(text);}
            if let Some(n)=payload.and_then(|v|v.get("number")).and_then(Value::as_u64){
                if n!=p.number as u64{differences.push(format!("numéro : {} → {n}",p.number));}
            }
            let detail=if differences.is_empty(){"aucun changement".into()}else{differences.join(", ")};
            scope_player(state,actor,pid,format!("Modification de la joueuse {} — {detail}",quoted(&old))).await
        }
        ["player","delete",pid]=>{
            let Some(pid)=id(pid) else{return Context::admin("Suppression d'une fiche de joueuse".into())};
            match player::Entity::find_by_id(pid).one(&state.db).await{
                Ok(Some(p))=>scope_player(state,actor,pid,
                    format!("Suppression de la fiche de la joueuse {}",quoted(&label_player(&p)))).await,
                _=>Context::admin("Suppression d'une fiche de joueuse introuvable".into()),
            }
        }
        ["matrix","result"]|["activity_player","update"]=>{
            let who=if let Some(pid)=body_id(payload,"player_id"){
                player::Entity::find_by_id(pid).one(&state.db).await.ok().flatten()
                    .map(|p|quoted(&label_player(&p))).unwrap_or_else(||"une joueuse".into())
            }else{"une joueuse".into()};
            let act=if let Some(aid)=body_id(payload,"activity_id"){
                if let Ok(Some(a))=catalog_activity::Entity::find_by_id(aid).one(&state.db).await{
                    quoted(&a.name)
                }else if let Ok(Some(a))=activity::Entity::find_by_id(aid).one(&state.db).await{
                    quoted(&a.name)
                }else{"une activité".into()}
            }else{"une activité".into()};
            Context::personal(format!("Mise à jour de l'évaluation de {who} pour {act}"))
        }
        ["matrix","select"]=>{
            let name=if let Some(aid)=body_id(payload,"activity_id"){
                catalog_activity::Entity::find_by_id(aid).one(&state.db).await.ok().flatten()
                    .map(|a|quoted(&a.name)).unwrap_or_else(||"une activité".into())
            }else{"une activité".into()};
            Context::personal(format!("Sélection de {name} dans le tableur"))
        }
        ["matrix","activity"]=>Context::personal("Ajout d'une activité au tableur".into()),
        ["activity","create"]=>{
            let name=body_string(payload,"name").unwrap_or("sans nom");
            Context::personal(format!("Création de l'activité personnelle {}",quoted(name)))
        }
        ["activity","update"]=>{
            let old=if let Some(aid)=body_id(payload,"id"){
                activity::Entity::find_by_id(aid).one(&state.db).await.ok().flatten()
                    .map(|a|a.name).unwrap_or_else(||"inconnue".into())
            }else{"inconnue".into()};
            let next=body_string(payload,"name");
            let detail=change_text(&old,next,"nom").unwrap_or_else(||"consignes ou date modifiées".into());
            Context::personal(format!("Modification de l'activité {} — {detail}",quoted(&old)))
        }
        ["activity","delete",activity_id]=>{
            let name=if let Some(aid)=id(activity_id){
                activity::Entity::find_by_id(aid).one(&state.db).await.ok().flatten()
                    .map(|a|a.name).unwrap_or_else(||"inconnue".into())
            }else{"inconnue".into()};
            Context::personal(format!("Suppression de l'activité {}",quoted(&name)))
        }
        ["catalog","upload"]=>Context::personal("Téléversement d'une illustration d'activité".into()),
        ["tactics"]=>{
            let name=body_string(payload,"name").unwrap_or("sans nom");
            Context::personal(format!("Création du schéma tactique {}",quoted(name)))
        }
        ["tactics",diagram_id]=>{
            let existing=id(diagram_id).and_then(|id|Some(id));
            let old=if let Some(id)=existing{
                tactical_diagram::Entity::find_by_id(id).one(&state.db).await.ok().flatten()
                    .map(|d|d.name).unwrap_or_else(||"inconnu".into())
            }else{"inconnu".into()};
            let detail=change_text(&old,body_string(payload,"name"),"nom")
                .unwrap_or_else(||"objets ou trajectoires modifiés".into());
            Context::personal(if *method==Method::DELETE{
                format!("Suppression du schéma tactique {}",quoted(&old))
            }else{format!("Modification du schéma tactique {} — {detail}",quoted(&old))})
        }
        ["admin","users",target,"role"]=>{
            let username=if let Some(uid)=id(target){
                user::Entity::find_by_id(uid).one(&state.db).await.ok().flatten()
                    .map(|u|u.username).unwrap_or_else(||"inconnu".into())
            }else{"inconnu".into()};
            let role=body_string(payload,"role").unwrap_or("non précisé");
            Context::admin(format!("Attribution du rôle {role} au compte {}",quoted(&username)))
        }
        ["account","profile"]=>{
            let old=user::Entity::find_by_id(actor).one(&state.db).await.ok().flatten()
                .map(|u|u.username).unwrap_or_else(||"compte".into());
            let detail=change_text(&old,body_string(payload,"username"),"identifiant")
                .unwrap_or_else(||"profil personnel modifié".into());
            Context::personal(format!("Modification du profil de {} — {detail}",quoted(&old)))
        }
        ["account","password"]=>Context::personal("Changement du mot de passe du compte".into()),
        ["preferences"]=>Context::personal("Mise à jour des préférences d'affichage".into()),
        ["clubs"] if *method==Method::POST=>{
            let name=payload.and_then(|v|v.get("name")).and_then(Value::as_str);
            Context::admin(match name{
                Some(name)=>format!("Création du club {}",quoted(name)),
                None=>"Création d'un club".into(),
            })
        }
        ["clubs",id,tail @ ..]=>{
            let Ok(id)=Uuid::parse_str(id) else {return Context::admin(generic)};
            let Ok(Some(c))=club::Entity::find_by_id(id).one(&state.db).await
                else{return Context::admin(generic)};
            let name=quoted(&c.name);
            let desc=match *tail {
                ["teams"]=>{
                    let new_name=payload.and_then(|v|v.get("name")).and_then(Value::as_str);
                    match new_name{
                        Some(new_name)=>format!("Création de l'équipe {} dans le club {name}",quoted(new_name)),
                        None=>format!("Création d'une équipe dans le club {name}"),
                    }
                },
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
                ["players"]=>{
                    let first=payload.and_then(|p|p.get("firstname")).and_then(Value::as_str);
                    let last=payload.and_then(|p|p.get("lastname")).and_then(Value::as_str);
                    match (first,last){
                        (Some(first),Some(last))=>
                            format!("Création de la joueuse {} dans l'équipe {name}",
                                quoted(&format!("{first} {last}"))),
                        _=>format!("Création d'une joueuse dans l'équipe {name}"),
                    }
                },
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
        ["trainings",training_id,rest @ ..]=>{
            let Some(tid)=id(training_id) else{return Context::admin("Opération sur une séance inconnue".into())};
            let Ok(Some(t))=training::Entity::find_by_id(tid).one(&state.db).await
                else{return Context::admin("Opération sur une séance introuvable".into())};
            let what=match *rest{
                []=>match *method{
                    Method::PUT=>format!("Modification de la séance {}",quoted(&t.name)),
                    _=>format!("{} de la séance {}",operation(method),quoted(&t.name)),
                },
                ["attendance"]=>format!("Modification des présences de la séance {}",quoted(&t.name)),
                ["scores"]=>{
                    let who=if let Some(pid)=body_id(payload,"player_id"){
                        player::Entity::find_by_id(pid).one(&state.db).await.ok().flatten()
                            .map(|p|quoted(&label_player(&p))).unwrap_or_else(||"une joueuse".into())
                    }else{"une joueuse".into()};
                    format!("Notation de {who} dans la séance {}",quoted(&t.name))
                },
                ["activities"]=>format!("Ajout d'une activité à la séance {}",quoted(&t.name)),
                ["catalog",catalog_id]=>{
                    let name=if let Some(aid)=id(catalog_id){
                        catalog_activity::Entity::find_by_id(aid).one(&state.db).await.ok().flatten()
                            .map(|c|c.name).unwrap_or_else(||"activité".into())
                    }else{"activité".into()};
                    format!("Ajout de l'activité {} à la séance {}",quoted(&name),quoted(&t.name))
                },
                ["activities","order"]=>format!("Réorganisation des activités de la séance {}",quoted(&t.name)),
                ["activities",aid,rest @ ..]=>{
                    let name=if let Some(aid)=id(aid){
                        training_activity::Entity::find_by_id(aid).one(&state.db).await.ok().flatten()
                            .map(|a|a.name).unwrap_or_else(||"activité introuvable".into())
                    }else{"activité".into()};
                    match *rest{
                        [] if *method==Method::DELETE=>
                            format!("Retrait de l'activité {} de la séance {}",quoted(&name),quoted(&t.name)),
                        []=>{
                            let change=change_text(&name,body_string(payload,"name"),"nom")
                                .unwrap_or_else(||"consignes ou illustration modifiées".into());
                            format!("Modification de l'activité {} dans la séance {} — {change}",quoted(&name),quoted(&t.name))
                        },
                        ["materials"]=>format!("Ajout de matériel à l'activité {} dans la séance {}",quoted(&name),quoted(&t.name)),
                        ["materials",mid]=>{
                            let material=if let Some(mid)=id(mid){
                                if let Ok(Some(m))=training_material::Entity::find_by_id(mid).one(&state.db).await {
                                    m.name
                                }else if let Ok(Some(m))=catalog_material::Entity::find_by_id(mid).one(&state.db).await {
                                    m.name
                                }else{"matériel".into()}
                            }else{"matériel".into()};
                            format!("Retrait du matériel {} de l'activité {} dans la séance {}",quoted(&material),quoted(&name),quoted(&t.name))
                        }
                        _=>format!("{} de l'activité {} dans la séance {}",operation(method),quoted(&name),quoted(&t.name))
                    }
                },
                _=>format!("{} de la séance {}",operation(method),quoted(&t.name))
            };
            scoped_team_or_personal(state,t.team_id,what).await
        }
        ["catalog",catalog_id,rest @ ..]=>{
            let Some(aid)=id(catalog_id) else{return Context::admin("Opération sur une activité inconnue".into())};
            let Ok(Some(c))=catalog_activity::Entity::find_by_id(aid).one(&state.db).await
                else{return Context::admin("Opération sur une activité introuvable".into())};
            let what=match *rest{
                [] if *method==Method::DELETE=>format!("Suppression de l'activité {}",quoted(&c.name)),
                []=>{
                    let change=change_text(&c.name,body_string(payload,"name"),"nom")
                        .unwrap_or_else(||"description, image ou équipe modifiée".into());
                    format!("Modification de l'activité {} — {change}",quoted(&c.name))
                },
                ["materials"]=>{
                    let material=body_string(payload,"name").unwrap_or("matériel");
                    format!("Ajout du matériel {} à l'activité {}",quoted(material),quoted(&c.name))
                },
                ["materials",material_id]=>{
                    let name=if let Some(mid)=id(material_id){
                        catalog_material::Entity::find_by_id(mid).one(&state.db).await.ok().flatten()
                            .map(|m|m.name).unwrap_or_else(||"matériel introuvable".into())
                    }else{"matériel".into()};
                    format!("{} du matériel {} de l'activité {}",operation(method),quoted(&name),quoted(&c.name))
                },
                _=>format!("{} de l'activité {}",operation(method),quoted(&c.name))
            };
            scoped_team_or_personal(state,c.team_id,what).await
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
