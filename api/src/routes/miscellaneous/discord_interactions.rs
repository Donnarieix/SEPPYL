//! Signed Discord interaction endpoint. No Gateway process or Message Content Intent.
//! Every mutation is attributed to a live SEPPYL administrator/maintainer.
use axum::{Json, body::Bytes, extract::{Path,State,ConnectInfo}, http::{HeaderMap,StatusCode}};
use std::net::SocketAddr;
use ed25519_dalek::{Signature,Verifier,VerifyingKey};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait, ConnectionTrait,
};
use serde_json::{Value,json};
use uuid::Uuid;
use crate::{
    entities::{ticket,ticket_message,user,audit_event},
    middleware::{CurrentRole,Role},
    routes::{authentication::login::Claims,miscellaneous::{ticket_live,tickets,discord_link as link}},
    state::AppState,
};

type R<T> = Result<T,StatusCode>;
fn signing_valid(public_hex:&str,signature_hex:&str,timestamp:&str,body:&[u8],at:i64)->bool{
    let Ok(when)=timestamp.parse::<i64>() else{return false};
    if when.abs_diff(at)>300{return false;}
    let Ok(key)=hex::decode(public_hex) else{return false};
    let Ok(key):Result<[u8;32],_>=key.try_into() else{return false};
    let Ok(key)=VerifyingKey::from_bytes(&key) else{return false};
    let Ok(bytes)=hex::decode(signature_hex) else{return false};
    let Ok(signature)=Signature::from_slice(&bytes) else{return false};
    let mut message=timestamp.as_bytes().to_vec();
    message.extend_from_slice(body);
    key.verify(&message,&signature).is_ok()
}
fn response(message:impl Into<String>)->Json<Value>{
    Json(json!({"type":4,"data":{"content":message.into(),"flags":64,
        "allowed_mentions":{"parse":[]}}}))
}
fn response_buttons(message:String,id:Uuid)->Json<Value>{
    Json(json!({"type":4,"data":{"content":message,"flags":64,
        "allowed_mentions":{"parse":[]},"components":buttons(id)}}))
}
pub(super) fn buttons(id:Uuid)->Value{
    json!([{"type":1,"components":[
        {"type":2,"style":2,"label":"Voir","custom_id":format!("seppyl|voir|{id}")},
        {"type":2,"style":1,"label":"Répondre","custom_id":format!("seppyl|repondre|{id}")},
        {"type":2,"style":2,"label":"Statut","custom_id":format!("seppyl|statut|{id}")},
        {"type":2,"style":2,"label":"Maintainer","custom_id":format!("seppyl|maintainer|{id}")}
    ]}])
}
fn text_option<'a>(opts:&'a [Value],name:&str)->Option<&'a str>{
    opts.iter().find(|v|v["name"]==name)?.get("value")?.as_str()
}
fn bool_option(opts:&[Value],name:&str)->Option<bool>{
    opts.iter().find(|v|v["name"]==name)?.get("value")?.as_bool()
}
fn command(data:&Value)->Option<(&str,&[Value])>{
    if data["name"]!="seppyl"{return None}
    let sub=data["options"].as_array()?.first()?;
    let name=sub["name"].as_str()?;
    Some((name,sub["options"].as_array().map(Vec::as_slice).unwrap_or(&[])))
}
fn modal(id:Uuid)->Json<Value>{
    Json(json!({"type":9,"data":{
        "custom_id":format!("seppyl|modal|{id}"),
        "title":"Répondre au ticket",
        "components":[{"type":1,"components":[{
            "type":4,"custom_id":"message","style":2,
            "label":"Réponse SEPPYL","required":true,"min_length":1,"max_length":2000
        }]}]
    }}))
}
fn status_buttons(id:Uuid,current:&str)->Json<Value>{
    let options=[
        ("open","Ouvert"),("in_progress","En cours"),("resolved","Résolu"),("closed","Fermé")
    ].into_iter().map(|(status,label)|{
        json!({"type":2,"style":if current==status{3}else{2},
            "label":label,"custom_id":format!("seppyl|set|{id}|{status}")})
    }).collect::<Vec<_>>();
    Json(json!({"type":4,"data":{
        "content":format!("Choisis un statut pour le ticket {id} (actuellement : {current})."),
        "flags":64,"allowed_mentions":{"parse":[]},
        "components":[{"type":1,"components":options}]
    }}))
}
fn parse_id(value:Option<&str>)->Option<Uuid>{
    Uuid::parse_str(value?).ok()
}
fn excerpt(value:&str,limit:usize)->String{
    let trimmed=value.trim();
    let mut clipped=trimmed.chars().take(limit).collect::<String>();
    if trimmed.chars().count()>limit{clipped.push('…');}
    clipped
}
async fn check_ticket(state:&AppState,id:Uuid)->R<Option<ticket::Model>>{
    ticket::Entity::find_by_id(id).one(&state.db).await.map_err(link::db)
}
async fn list(state:&AppState)->R<String>{
    let rows=ticket::Entity::find()
        .filter(ticket::COLUMN.status.is_in(["open","in_progress"]))
        .order_by_desc(ticket::COLUMN.updated_at).limit(10)
        .all(&state.db).await.map_err(link::db)?;
    if rows.is_empty(){return Ok("Aucun ticket ouvert ou en cours.".into());}
    let mut lines=vec!["**Tickets à traiter (10 derniers maximum)**".to_string()];
    for item in rows{
        lines.push(format!("• **{}** · {} · [Ouvrir](https://seppyl.donnarieix.fr/tickets/{})",
            excerpt(&item.subject,72),item.status,item.id));
    }
    Ok(lines.join("\n"))
}
async fn detail(state:&AppState,actor:Uuid,id:Uuid)->R<Option<String>>{
    let Some(row)=check_ticket(state,id).await? else{return Ok(None)};
    let account=user::Entity::find_by_id(actor).one(&state.db).await.map_err(link::db)?;
    let role=account.and_then(|a|a.role.as_deref().and_then(Role::parse))
        .unwrap_or(Role::User);
    if !role.is_admin(){return Err(StatusCode::FORBIDDEN);}
    // Reuse the regular ticket route's read receipts, including Maintainer escalation.
    let detail=tickets::get(
        axum::Extension(Claims{sub:actor,exp:u64::MAX}),
        axum::Extension(CurrentRole(role)),
        State(state.clone()),Path(id),
    ).await?;
    let mut parts=vec![format!(
        "**{}**\nStatut : **{}** · Maintainer : **{}**\n[Ouvrir dans SEPPYL](https://seppyl.donnarieix.fr/tickets/{})",
        excerpt(&row.subject,140),row.status,
        if row.maintainer_required{"Oui"}else{"Non"},row.id
    )];
    let recent=detail.0.messages.into_iter()
        .filter(|m|m.deleted_at.is_none()).rev().take(5).collect::<Vec<_>>();
    for item in recent.into_iter().rev(){
        let author=user::Entity::find_by_id(item.author_id).one(&state.db).await.map_err(link::db)?;
        let name=author.map(|u|u.username).unwrap_or_else(||"Compte supprimé".into());
        let content=if item.attachment_name.is_some() && item.message.is_empty(){
            "[Image jointe — voir sur SEPPYL]".to_string()
        }else{excerpt(&item.message,200)};
        parts.push(format!("**{}** : {}",excerpt(&name,32),content));
    }
    let joined=parts.join("\n");
    Ok(Some(excerpt(&joined,1850)))
}
async fn staff_in_transaction(tx:&sea_orm::DatabaseTransaction,actor:Uuid)->R<bool>{
    let account=user::Entity::find_by_id(actor).one(tx).await.map_err(link::db)?;
    Ok(account.is_some_and(|user|!user.disabled.unwrap_or(false)
        && user.role.as_deref().and_then(Role::parse).is_some_and(Role::is_admin)))
}
async fn answer(state:&AppState,actor:Uuid,interaction:&str,id:Uuid,body:&str)->R<String>{
    if body.trim().is_empty() || body.len()>10_000{return Ok("Message vide ou trop long.".into());}
    let tx=state.db.begin().await.map_err(link::db)?;
    if !staff_in_transaction(&tx,actor).await?{
        return Ok("Compte SEPPYL non autorisé : liaison ou rôle à vérifier.".into());
    }
    if !link::reserve_interaction(&tx,interaction).await?{
        return Ok("Cette interaction a déjà été prise en compte.".into());
    }
    let Some(row)=ticket::Entity::find_by_id(id).one(&tx).await.map_err(link::db)? else{
        return Ok("Ticket introuvable.".into());
    };
    ticket_message::ActiveModel{
        id:Set(Uuid::new_v4()),ticket_id:Set(id),author_id:Set(actor),
        message:Set(body.trim().into()),
        created_at:Set(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map(|t|t.as_micros() as i64).unwrap_or_default()),
        ..Default::default()
    }.insert(&tx).await.map_err(link::db)?;
    let mut update:ticket::ActiveModel=row.into();
    update.updated_at=Set(link::now());
    update.update(&tx).await.map_err(link::db)?;
    audit_event::ActiveModel{
        id:Set(Uuid::new_v4()),actor_id:Set(actor),action:Set("create".into()),
        path:Set(format!("/discord/tickets/{id}/messages")),status:Set(201),
        timestamp:Set(link::now()),scope:Set(Some("admin".into())),
        description:Set(Some(format!("Réponse à un ticket depuis Discord ({id})"))),
        ..Default::default()
    }.insert(&tx).await.map_err(link::db)?;
    tx.commit().await.map_err(link::db)?;
    // Discord-origin replies must not loop back into bot notifications.
    ticket_live::changed(id,actor);
    Ok(format!("Réponse enregistrée dans le ticket {id}."))
}
async fn update_status(state:&AppState,actor:Uuid,interaction:&str,id:Uuid,
    desired:Option<&str>,escalate:Option<bool>)->R<String>{
    let tx=state.db.begin().await.map_err(link::db)?;
    if !staff_in_transaction(&tx,actor).await?{
        return Ok("Compte SEPPYL non autorisé : liaison ou rôle à vérifier.".into());
    }
    if !link::reserve_interaction(&tx,interaction).await?{
        return Ok("Cette interaction a déjà été prise en compte.".into());
    }
    let Some(row)=ticket::Entity::find_by_id(id).one(&tx).await.map_err(link::db)? else{
        return Ok("Ticket introuvable.".into());
    };
    if let Some(next)=desired{
        if !matches!(next,"open"|"in_progress"|"resolved"|"closed"){
            return Ok("Statut invalide.".into());
        }
    }
    let status=desired.unwrap_or(&row.status).to_string();
    let mention=escalate.unwrap_or(row.maintainer_required);
    let newly_escalated=mention&&!row.maintainer_required;
    let mut active:ticket::ActiveModel=row.clone().into();
    active.status=Set(status.clone());
    active.maintainer_required=Set(mention);
    if newly_escalated{active.maintainer_generation=Set(row.maintainer_generation+1);}
    active.updated_at=Set(link::now());
    active.update(&tx).await.map_err(link::db)?;
    audit_event::ActiveModel{
        id:Set(Uuid::new_v4()),actor_id:Set(actor),action:Set("update".into()),
        path:Set(format!("/discord/tickets/{id}/status")),status:Set(204),
        timestamp:Set(link::now()),scope:Set(Some("admin".into())),
        description:Set(Some(format!("Statut depuis Discord : {status}; Maintainer : {mention}"))),
        ..Default::default()
    }.insert(&tx).await.map_err(link::db)?;
    tx.commit().await.map_err(link::db)?;
    ticket_live::changed(id,actor);
    Ok(format!("Ticket {id} : statut **{status}**, Maintainer **{}**.",if mention{"oui"}else{"non"}))
}
pub async fn interaction(
    State(state):State<AppState>,headers:HeaderMap,body:Bytes,
)->R<Json<Value>>{
    let key=std::env::var("SEPPYL_DISCORD_PUBLIC_KEY")
        .map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    let signature=headers.get("x-signature-ed25519")
        .and_then(|v|v.to_str().ok()).ok_or(StatusCode::UNAUTHORIZED)?;
    let timestamp=headers.get("x-signature-timestamp")
        .and_then(|v|v.to_str().ok()).ok_or(StatusCode::UNAUTHORIZED)?;
    if !signing_valid(&key,signature,timestamp,&body,link::now()){
        return Err(StatusCode::UNAUTHORIZED);
    }
    let payload:Value=serde_json::from_slice(&body).map_err(|_|StatusCode::BAD_REQUEST)?;
    dispatch_verified(&state,payload).await
}
fn same_secret(expected:&str,received:&str)->bool{
    let left=expected.as_bytes();let right=received.as_bytes();
    left.len()==right.len() && left.iter().zip(right.iter())
        .fold(0u8,|acc,(a,b)|acc | (a ^ b))==0
}
/// Local-only adapter for a Python Discord Gateway bot. An untrusted client
/// cannot supply arbitrary user IDs: this route is never enabled outside an
/// explicitly loopback-bound development API, and requires a private secret.
/// The production /discord/interactions route always requires Ed25519.
pub async fn local_gateway(
    State(state):State<AppState>,
    ConnectInfo(peer):ConnectInfo<SocketAddr>,
    headers:HeaderMap,
    Json(payload):Json<Value>,
)->R<Json<Value>>{
    if !link::local_gateway_enabled() || !peer.ip().is_loopback(){
        return Err(StatusCode::NOT_FOUND);
    }
    let expected=std::env::var("SEPPYL_DISCORD_LOCAL_BRIDGE_TOKEN")
        .map_err(|_|StatusCode::NOT_FOUND)?;
    let provided=headers.get("x-seppyl-bridge-token")
        .and_then(|v|v.to_str().ok()).unwrap_or_default();
    if !same_secret(&expected,provided){return Err(StatusCode::UNAUTHORIZED);}
    dispatch_verified(&state,payload).await
}
async fn dispatch_verified(state:&AppState,payload:Value)->R<Json<Value>>{
    let guild=std::env::var("SEPPYL_DISCORD_GUILD_ID")
        .map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    let application=std::env::var("SEPPYL_DISCORD_APPLICATION_ID")
        .map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    if payload["application_id"].as_str()!=Some(&application){
        return Err(StatusCode::UNAUTHORIZED);
    }
    if payload["type"]==1{return Ok(Json(json!({"type":1})))}
    if payload["guild_id"].as_str()!=Some(&guild){
        return Ok(response("Ce serveur Discord n'est pas autorisé."));
    }
    let Some(discord_id)=payload["member"]["user"]["id"].as_str() else{
        return Ok(response("Cette commande doit être utilisée depuis le serveur Discord configuré."));
    };
    let Some(interaction_id)=payload["id"].as_str() else{return Err(StatusCode::BAD_REQUEST)};
    let data=&payload["data"];
    let actor=link::staff_for_discord(&state,discord_id).await?;
    let typ=payload["type"].as_u64().unwrap_or_default();
    if typ==2{
        let Some((name,options))=command(data) else{return Ok(response("Commande inconnue."))};
        if name=="lier"{
            return Ok(response(link::redeem_code(&state,discord_id,
                text_option(options,"code").unwrap_or_default()).await?));
        }
        if name=="delier"{
            state.db.execute_raw(link::sql(
                "DELETE FROM support_discord_links WHERE discord_user_id=?",
                vec![discord_id.into()]
            )).await.map_err(link::db)?;
            return Ok(response("Compte Discord délié de SEPPYL."));
        }
        let Some(actor)=actor else{return Ok(response(
            "Liaison requise : ouvre Mon compte > Discord sur SEPPYL, puis utilise /seppyl lier."
        ))};
        return match name{
            "liste"=>Ok(response(list(&state).await?)),
            "voir"=>match parse_id(text_option(options,"ticket")){
                Some(id)=>match detail(&state,actor,id).await?{
                    Some(text)=>Ok(response_buttons(text,id)),None=>Ok(response("Ticket introuvable.")),
                },
                None=>Ok(response("UUID de ticket invalide.")),
            },
            "repondre"=>match parse_id(text_option(options,"ticket")){
                Some(id)=>Ok(response(answer(&state,actor,interaction_id,id,
                    text_option(options,"message").unwrap_or_default()).await?)),
                None=>Ok(response("UUID de ticket invalide.")),
            },
            "statut"=>match parse_id(text_option(options,"ticket")){
                Some(id)=>Ok(response(update_status(&state,actor,interaction_id,id,
                    text_option(options,"etat"),None).await?)),
                None=>Ok(response("UUID de ticket invalide.")),
            },
            "maintainer"=>match (parse_id(text_option(options,"ticket")),bool_option(options,"activer")){
                (Some(id),Some(enabled))=>Ok(response(update_status(&state,actor,interaction_id,
                    id,None,Some(enabled)).await?)),
                _=>Ok(response("Paramètres invalides.")),
            },
            _=>Ok(response("Sous-commande inconnue.")),
        };
    }
    if typ==3 || typ==5{
        let Some(actor)=actor else{
            return Ok(response("Liaison SEPPYL requise. Utilise /seppyl lier."));
        };
        let Some(custom_id)=data["custom_id"].as_str() else{return Ok(response("Action inconnue."))};
        let parts=custom_id.split('|').collect::<Vec<_>>();
        if parts.len()<3 || parts[0]!="seppyl"{
            return Ok(response("Action inconnue."));
        }
        let Some(id)=parse_id(parts.get(2).copied()) else{
            return Ok(response("Identifiant de ticket invalide."));
        };
        if typ==5{
            if parts[1]!="modal"{return Ok(response("Formulaire inconnu."))}
            let text=data["components"].as_array()
                .and_then(|rows|rows.iter().flat_map(|r|r["components"].as_array()
                    .into_iter().flatten()).find(|input|input["custom_id"]=="message"))
                .and_then(|input|input["value"].as_str()).unwrap_or_default();
            return Ok(response(answer(&state,actor,interaction_id,id,text).await?));
        }
        return match parts[1]{
            "voir"=>match detail(&state,actor,id).await?{
                Some(text)=>Ok(response_buttons(text,id)),None=>Ok(response("Ticket introuvable.")),
            },
            "repondre"=>if check_ticket(&state,id).await?.is_some(){
                Ok(modal(id))
            }else{Ok(response("Ticket introuvable."))},
            "statut"=>if let Some(t)=check_ticket(&state,id).await?{
                Ok(status_buttons(id,&t.status))
            }else{Ok(response("Ticket introuvable."))},
            "maintainer"=>if let Some(row)=check_ticket(&state,id).await?{
                Ok(response(update_status(&state,actor,interaction_id,id,None,
                    Some(!row.maintainer_required)).await?))
            }else{Ok(response("Ticket introuvable."))},
            "set" if parts.len()==4=>Ok(response(update_status(&state,actor,
                interaction_id,id,Some(parts[3]),None).await?)),
            _=>Ok(response("Action inconnue.")),
        };
    }
    Ok(response("Interaction non prise en charge."))
}

#[cfg(test)]
mod tests{
    use super::*;
    use ed25519_dalek::{SigningKey,Signer};
    #[test]
    fn valid_signature_is_bound_to_body_and_fresh_timestamp(){
        let key=SigningKey::from_bytes(&[9u8;32]);
        let body=br#"{"type":1}"#;
        let ts="1800000000";
        let mut signed=ts.as_bytes().to_vec();signed.extend_from_slice(body);
        let signature=hex::encode(key.sign(&signed).to_bytes());
        let public=hex::encode(key.verifying_key().to_bytes());
        assert!(signing_valid(&public,&signature,ts,body,1800000000));
        assert!(!signing_valid(&public,&signature,ts,b"changed",1800000000));
        assert!(!signing_valid(&public,&signature,ts,body,1800000301));
        assert!(!signing_valid(&public,"garbage",ts,body,1800000000));
    }
    #[tokio::test]
    async fn staff_link_can_reply_and_escalate_once_and_is_revoked_by_role_change(){
        use sea_orm::{ActiveModelTrait,ActiveValue::Set,Database,QueryFilter,ColumnTrait};
        let db=Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        crate::migrations::add_discord_integration(&db).await.unwrap();
        let staff_id=Uuid::new_v4();
        let requester_id=Uuid::new_v4();
        for (id,name,role) in [
            (staff_id,"discord-staff-test","maintainer"),
            (requester_id,"discord-requester-test","user")
        ]{
            user::ActiveModel{
                id:Set(id),username:Set(name.into()),password:Set("no-login".into()),
                role:Set(Some(role.into())),disabled:Set(None),..Default::default()
            }.insert(&db).await.unwrap();
        }
        let ticket_id=Uuid::new_v4();
        ticket::ActiveModel{
            id:Set(ticket_id),author_id:Set(requester_id),subject:Set("Test".into()),
            status:Set("open".into()),maintainer_required:Set(false),
            maintainer_generation:Set(0),created_at:Set(1),updated_at:Set(1),
            ..Default::default()
        }.insert(&db).await.unwrap();
        let code=Uuid::new_v4().simple().to_string();
        db.execute_raw(link::sql(
            "INSERT INTO support_discord_codes(code,user_id,expires_at) VALUES(?,?,?)",
            vec![code.clone().into(),staff_id.to_string().into(),(link::now()+600).into()]
        )).await.unwrap();
        let state=AppState{db:db.clone()};
        assert!(link::redeem_code(&state,"123456789012345678",&code)
            .await.unwrap().contains("lié"));
        assert!(link::redeem_code(&state,"123456789012345678",&code)
            .await.unwrap().contains("déjà utilisé"));
        assert_eq!(link::staff_for_discord(&state,"123456789012345678")
            .await.unwrap(),Some(staff_id));
        assert!(answer(&state,staff_id,"123456789000001",ticket_id,"Bonjour")
            .await.unwrap().contains("enregistrée"));
        assert!(answer(&state,staff_id,"123456789000001",ticket_id,"Bonjour")
            .await.unwrap().contains("déjà"));
        let messages=ticket_message::Entity::find()
            .filter(ticket_message::COLUMN.ticket_id.eq(ticket_id))
            .all(&db).await.unwrap();
        assert_eq!(messages.len(),1);
        assert_eq!(messages[0].author_id,staff_id);
        let outcome=update_status(&state,staff_id,"123456789000002",
            ticket_id,Some("in_progress"),Some(true)).await.unwrap();
        assert!(outcome.contains("in_progress"));
        let row=ticket::Entity::find_by_id(ticket_id).one(&db).await.unwrap().unwrap();
        assert_eq!(row.maintainer_generation,1);
        assert!(row.maintainer_required);
        assert_eq!(row.status,"in_progress");
        update_status(&state,staff_id,"123456789000002",ticket_id,
            Some("closed"),Some(false)).await.unwrap();
        assert!(ticket::Entity::find_by_id(ticket_id).one(&db).await.unwrap()
            .unwrap().maintainer_required);
        let staff=user::Entity::find_by_id(staff_id).one(&db).await.unwrap().unwrap();
        let mut active:user::ActiveModel=staff.into();
        active.role=Set(Some("user".into()));
        active.update(&db).await.unwrap();
        assert_eq!(link::staff_for_discord(&state,"123456789012345678")
            .await.unwrap(),None);
    }

    #[test]
    fn buttons_are_scoped_to_ticket_ids(){
        let id=Uuid::new_v4();
        assert!(buttons(id).to_string().contains(&id.to_string()));
        assert!(parse_id(Some("not-a-uuid")).is_none());
    }
}
