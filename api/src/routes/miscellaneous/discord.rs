//! Discord interactions: signed HTTP commands, explicit staff linking and replies.
//! No Discord bot process, Gateway permission, or client-side secret is required.
use axum::{
    Extension, Json, body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, EntityTrait,
    Statement, TransactionTrait,
};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    entities::{ticket, user},
    middleware::{CurrentRole, Role},
    routes::{authentication::login::Claims, miscellaneous::ticket_live},
    state::AppState,
};

type R<T> = Result<T, StatusCode>;
const LINK_LIFETIME: i64 = 600;
const INTERACTION_SKEW: i64 = 300;

fn db(error: sea_orm::DbErr) -> StatusCode {
    tracing::error!(%error, "Discord integration database failure");
    StatusCode::INTERNAL_SERVER_ERROR
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}
fn configured() -> bool {
    std::env::var("SEPPYL_DISCORD_PUBLIC_KEY").is_ok_and(|s| !s.is_empty())
        && std::env::var("SEPPYL_DISCORD_GUILD_ID").is_ok_and(|s| !s.is_empty())
}
fn sql(text: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Sqlite, text, values)
}
fn staff(role: Role) -> R<()> {
    if role.is_admin() { Ok(()) } else { Err(StatusCode::FORBIDDEN) }
}

#[derive(Serialize)]
pub struct LinkStatus {
    enabled: bool,
    discord_user_id: Option<String>,
}
#[derive(Serialize)]
pub struct LinkCode {
    code: String,
    expires_at: i64,
}

/// Only the current administrator/maintainer may create a short-lived link code.
pub async fn status(
    Extension(claims): Extension<Claims>,
    Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>,
) -> R<Json<LinkStatus>> {
    staff(role.0)?;
    let row = state.db.query_one_raw(sql(
        "SELECT discord_user_id FROM support_discord_links WHERE seppyl_user_id = ?",
        vec![claims.sub.into()],
    )).await.map_err(db)?;
    let discord_user_id = row
        .map(|row| row.try_get("", "discord_user_id"))
        .transpose().map_err(db)?;
    Ok(Json(LinkStatus { enabled: configured(), discord_user_id }))
}

pub async fn link_code(
    Extension(claims): Extension<Claims>,
    Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>,
) -> R<Json<LinkCode>> {
    staff(role.0)?;
    if !configured() { return Err(StatusCode::SERVICE_UNAVAILABLE); }
    let code = Uuid::new_v4().simple().to_string();
    let expires_at = now() + LINK_LIFETIME;
    let tx = state.db.begin().await.map_err(db)?;
    tx.execute_raw(sql(
        "DELETE FROM support_discord_link_codes WHERE seppyl_user_id = ?",
        vec![claims.sub.into()],
    )).await.map_err(db)?;
    tx.execute_raw(sql(
        "INSERT INTO support_discord_link_codes (code, seppyl_user_id, expires_at) VALUES (?, ?, ?)",
        vec![code.clone().into(), claims.sub.into(), expires_at.into()],
    )).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    Ok(Json(LinkCode { code, expires_at }))
}

pub async fn unlink(
    Extension(claims): Extension<Claims>,
    Extension(role): Extension<CurrentRole>,
    State(state): State<AppState>,
) -> R<StatusCode> {
    staff(role.0)?;
    let tx = state.db.begin().await.map_err(db)?;
    tx.execute_raw(sql(
        "DELETE FROM support_discord_links WHERE seppyl_user_id = ?",
        vec![claims.sub.into()],
    )).await.map_err(db)?;
    tx.execute_raw(sql(
        "DELETE FROM support_discord_link_codes WHERE seppyl_user_id = ?",
        vec![claims.sub.into()],
    )).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Discord signs the timestamp concatenated with the exact raw request body.
/// Never trust the user_id, guild_id or message fields before this verification.
fn verify_signature(public_hex: &str, signature_hex: &str, timestamp: &str, body: &[u8], current: i64) -> bool {
    let Ok(sent_at) = timestamp.parse::<i64>() else { return false; };
    if (sent_at - current).abs() > INTERACTION_SKEW { return false; }
    let Ok(raw_key) = hex::decode(public_hex) else { return false; };
    let Ok(key_bytes): Result<[u8; 32], _> = raw_key.try_into() else { return false; };
    let Ok(key) = VerifyingKey::from_bytes(&key_bytes) else { return false; };
    let Ok(raw_signature) = hex::decode(signature_hex) else { return false; };
    let Ok(signature) = Signature::from_slice(&raw_signature) else { return false; };
    let mut signed = timestamp.as_bytes().to_vec();
    signed.extend_from_slice(body);
    key.verify(&signed, &signature).is_ok()
}
fn ephemeral(message: &str) -> Json<Value> {
    Json(json!({"type":4,"data":{"content":message,"flags":64,
        "allowed_mentions":{"parse":[]}}}))
}
fn option<'a>(options: &'a [Value], name: &str) -> Option<&'a str> {
    options.iter().find(|entry| entry["name"] == name)?.get("value")?.as_str()
}
fn subcommand(data: &Value) -> Option<(&str, &[Value])> {
    if data["name"] != "seppyl" { return None; }
    let values = data["options"].as_array()?;
    let selected = values.first()?;
    let name = selected["name"].as_str()?;
    let args = selected["options"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    Some((name, args))
}
async fn linked_staff(state: &AppState, discord_user_id: &str) -> R<Option<Uuid>> {
    let row = state.db.query_one_raw(sql(
        "SELECT seppyl_user_id FROM support_discord_links WHERE discord_user_id = ?",
        vec![discord_user_id.into()],
    )).await.map_err(db)?;
    let Some(row) = row else { return Ok(None); };
    let account_id: Uuid = row.try_get("", "seppyl_user_id").map_err(db)?;
    let account = user::Entity::find_by_id(account_id).one(&state.db).await.map_err(db)?;
    let valid = account.is_some_and(|account| {
        !account.disabled.unwrap_or(false) &&
        account.role.as_deref().and_then(Role::parse).is_some_and(Role::is_admin)
    });
    Ok(valid.then_some(account_id))
}
async fn link(state: &AppState, discord_user_id: &str, code: &str) -> R<&'static str> {
    if code.len() != 32 || !code.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Ok("Code invalide.");
    }
    let tx = state.db.begin().await.map_err(db)?;
    let record = tx.query_one_raw(sql(
        "SELECT seppyl_user_id, expires_at FROM support_discord_link_codes WHERE code = ?",
        vec![code.to_string().into()],
    )).await.map_err(db)?;
    let Some(record) = record else { return Ok("Code inconnu ou déjà utilisé."); };
    let account_id: Uuid = record.try_get("", "seppyl_user_id").map_err(db)?;
    let expires: i64 = record.try_get("", "expires_at").map_err(db)?;
    if expires < now() { return Ok("Code expiré. Générez-en un nouveau dans SEPPYL."); }
    let account = user::Entity::find_by_id(account_id).one(&tx).await.map_err(db)?;
    let valid = account.is_some_and(|account| {
        !account.disabled.unwrap_or(false) &&
        account.role.as_deref().and_then(Role::parse).is_some_and(Role::is_admin)
    });
    if !valid { return Ok("Ce compte SEPPYL n'est pas autorisé."); }
    // Two-way unique mapping: neither Discord user nor SEPPYL account may link twice.
    tx.execute_raw(sql(
        "DELETE FROM support_discord_links WHERE discord_user_id = ? OR seppyl_user_id = ?",
        vec![discord_user_id.into(), account_id.into()],
    )).await.map_err(db)?;
    tx.execute_raw(sql(
        "INSERT INTO support_discord_links (discord_user_id, seppyl_user_id, linked_at) VALUES (?, ?, ?)",
        vec![discord_user_id.into(), account_id.into(), now().into()],
    )).await.map_err(db)?;
    tx.execute_raw(sql(
        "DELETE FROM support_discord_link_codes WHERE seppyl_user_id = ?",
        vec![account_id.into()],
    )).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    Ok("Compte Discord lié à SEPPYL. Vous pouvez répondre aux tickets.")
}
async fn reply(state: &AppState, discord_user_id: &str, interaction_id: &str, id: &str, content: &str) -> R<&'static str> {
    let Some(author_id) = linked_staff(state, discord_user_id).await? else {
        return Ok("Liez d'abord votre compte dans Mon compte > Discord sur SEPPYL.");
    };
    let Ok(ticket_id) = Uuid::parse_str(id) else { return Ok("Identifiant de ticket invalide."); };
    if content.trim().is_empty() || content.len() > 10_000 {
        return Ok("Le message doit contenir entre 1 et 10 000 octets.");
    }
    let Some(ticket_row) = ticket::Entity::find_by_id(ticket_id).one(&state.db).await.map_err(db)? else {
        return Ok("Ticket introuvable.");
    };
    // Discord interactions may be retried; the unique interaction ID prevents duplicates.
    let tx = state.db.begin().await.map_err(db)?;
    let inserted = tx.execute_raw(sql(
        "INSERT OR IGNORE INTO support_ticket_messages (id, ticket_id, author_id, message, created_at, discord_interaction_id) VALUES (?, ?, ?, ?, ?, ?)",
        vec![Uuid::new_v4().into(), ticket_id.into(), author_id.into(),
            content.trim().to_owned().into(), (now() * 1_000_000).into(), interaction_id.into()],
    )).await.map_err(db)?.rows_affected() > 0;
    if !inserted {
        tx.rollback().await.map_err(db)?;
        return Ok("Cette réponse a déjà été enregistrée.");
    }
    let mut updated: ticket::ActiveModel = ticket_row.into();
    updated.updated_at = Set(now());
    updated.update(&tx).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    // Do not repost Discord-origin messages to the Discord webhook.
    ticket_live::changed(ticket_id, author_id);
    Ok("Réponse ajoutée au ticket SEPPYL.")
}

/// Unauthenticated HTTP route: the Ed25519 signature IS the authentication.
/// No JWT (a Discord request has none), and no developer-controlled bypass.
pub async fn interaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> R<Json<Value>> {
    let public = std::env::var("SEPPYL_DISCORD_PUBLIC_KEY")
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let guild = std::env::var("SEPPYL_DISCORD_GUILD_ID")
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let signature = headers.get("x-signature-ed25519")
        .and_then(|v| v.to_str().ok()).ok_or(StatusCode::UNAUTHORIZED)?;
    let timestamp = headers.get("x-signature-timestamp")
        .and_then(|v| v.to_str().ok()).ok_or(StatusCode::UNAUTHORIZED)?;
    if !verify_signature(&public, signature, timestamp, &body, now()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let payload: Value = serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    match payload["type"].as_u64() {
        Some(1) => return Ok(Json(json!({"type":1}))), // Discord endpoint PING
        Some(2) => {}
        _ => return Ok(ephemeral("Interaction non prise en charge.")),
    }
    if payload["guild_id"].as_str() != Some(guild.as_str()) {
        return Ok(ephemeral("Ce serveur Discord n'est pas autorisé."));
    }
    let Some(discord_id) = payload["member"]["user"]["id"].as_str() else {
        return Ok(ephemeral("Commande réservée aux membres du serveur."));
    };
    let interaction_id = payload["id"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let Some((cmd, args)) = subcommand(&payload["data"]) else {
        return Ok(ephemeral("Commande inconnue."));
    };
    let text = match cmd {
        "lier" => {
            let code = option(args, "code").unwrap_or_default();
            link(&state, discord_id, code).await?
        }
        "repondre" => {
            let ticket_id = option(args, "ticket").unwrap_or_default();
            let message = option(args, "message").unwrap_or_default();
            reply(&state, discord_id, interaction_id, ticket_id, message).await?
        }
        _ => "Sous-commande inconnue.",
    };
    Ok(ephemeral(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    #[test]
    fn signatures_require_exact_body_and_recent_timestamp() {
        let seed = [7_u8; 32];
        let key = SigningKey::from_bytes(&seed);
        let moment = 1_800_000_000_i64;
        let body = br#"{"type":1}"#;
        let signed = format!("{moment}{}", String::from_utf8_lossy(body));
        let sig = key.sign(signed.as_bytes());
        let pubkey = hex::encode(key.verifying_key().to_bytes());
        let hexsig = hex::encode(sig.to_bytes());
        assert!(verify_signature(&pubkey, &hexsig, &moment.to_string(), body, moment));
        assert!(!verify_signature(&pubkey, &hexsig, &moment.to_string(), b"changed", moment));
        assert!(!verify_signature(&pubkey, &hexsig, &moment.to_string(), body, moment+301));
        assert!(!verify_signature(&pubkey, "invalid", &moment.to_string(), body, moment));
    }
    #[tokio::test]
    async fn linked_maintainer_can_reply_exactly_once_and_loses_access_when_demoted() {
        use sea_orm::{ActiveModelTrait, ActiveValue::Set, Database, QueryFilter, ColumnTrait};
        use crate::entities::ticket_message;
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        crate::migrations::add_discord_support(&db).await.unwrap();
        let staff_id=Uuid::new_v4();
        let requester_id=Uuid::new_v4();
        for (id, username, role) in [
            (staff_id, "test-maintainer-discord", "maintainer"),
            (requester_id, "test-requester-discord", "user")
        ] {
            user::ActiveModel {
                id:Set(id), username:Set(username.to_owned()), password:Set("irrelevant".into()),
                role:Set(Some(role.to_owned())), disabled:Set(None), ..Default::default()
            }.insert(&db).await.unwrap();
        }
        let ticket_id=Uuid::new_v4();
        ticket::ActiveModel {
            id:Set(ticket_id),author_id:Set(requester_id),subject:Set("Question".into()),
            status:Set("open".into()),maintainer_required:Set(false),
            maintainer_generation:Set(0),created_at:Set(1),updated_at:Set(1),
            ..Default::default()
        }.insert(&db).await.unwrap();
        let code=Uuid::new_v4().simple().to_string();
        db.execute_raw(sql(
            "INSERT INTO support_discord_link_codes (code, seppyl_user_id, expires_at) VALUES (?, ?, ?)",
            vec![code.clone().into(),staff_id.into(),(now()+60).into()],
        )).await.unwrap();
        let state=AppState{db:db.clone()};
        assert!(link(&state, "987654321012345678", &code).await.unwrap().contains("lié"));
        assert!(link(&state, "987654321012345678", &code).await.unwrap().contains("déjà utilisé"));
        let id=ticket_id.to_string();
        assert!(reply(&state,"987654321012345678","interaction-1",&id,"**Bonjour**")
            .await.unwrap().contains("ajoutée"));
        assert!(reply(&state,"987654321012345678","interaction-1",&id,"**Bonjour**")
            .await.unwrap().contains("déjà"));
        let messages=ticket_message::Entity::find()
            .filter(ticket_message::COLUMN.ticket_id.eq(ticket_id))
            .all(&db).await.unwrap();
        assert_eq!(messages.len(),1);
        assert_eq!(messages[0].author_id,staff_id);
        assert_eq!(messages[0].message,"**Bonjour**");
        let staff=user::Entity::find_by_id(staff_id).one(&db).await.unwrap().unwrap();
        let mut active:user::ActiveModel=staff.into();
        active.role=Set(Some("user".into()));
        active.update(&db).await.unwrap();
        assert!(reply(&state,"987654321012345678","interaction-2",&id,"Tentative")
            .await.unwrap().contains("Liez d'abord"));
        assert_eq!(ticket_message::Entity::find()
            .filter(ticket_message::COLUMN.ticket_id.eq(ticket_id))
            .all(&db).await.unwrap().len(),1);
    }

    #[test]
    fn only_explicit_subcommands_can_be_parsed() {
        let command = json!({"name":"seppyl","options":[{"name":"repondre","type":1,
            "options":[{"name":"ticket","value":"abc"},{"name":"message","value":"Salut"}]}]});
        let (name, options) = subcommand(&command).unwrap();
        assert_eq!(name, "repondre");
        assert_eq!(option(options, "message"), Some("Salut"));
        assert!(subcommand(&json!({"name":"other"})).is_none());
    }
}
