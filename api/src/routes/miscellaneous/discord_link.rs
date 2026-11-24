//! Links a Discord identity to a live staff account, without trusting Discord roles.
//! SEPPYL remains authoritative for permissions on every interaction.
use axum::{Extension, Json, extract::State, http::StatusCode};
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait, Statement,
    TransactionTrait, Value,
};
use serde::Serialize;
use uuid::Uuid;
use crate::{
    entities::user, middleware::{CurrentRole,Role},
    routes::authentication::login::Claims, state::AppState,
};
type R<T> = Result<T,StatusCode>;

pub(super) fn db(error:sea_orm::DbErr)->StatusCode{
    tracing::error!(%error,"Discord database operation failed");
    StatusCode::INTERNAL_SERVER_ERROR
}
pub(super) fn now()->i64{
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|time|time.as_secs() as i64).unwrap_or_default()
}
pub(super) fn sql(query:&str,params:Vec<Value>)->Statement{
    Statement::from_sql_and_values(DbBackend::Sqlite,query,params)
}
/// A local Gateway bridge is allowed only when the API itself binds to
/// loopback, outside the production working directory. Merely setting the
/// shared secret must never enable a public impersonation endpoint.
pub(super) fn local_gateway_enabled()->bool {
    let bind=std::env::var("SEPPYL_BIND").unwrap_or_default();
    let Ok(address)=bind.parse::<std::net::SocketAddr>() else{return false};
    if !address.ip().is_loopback() || address.port()==3101{return false;}
    if std::env::var("SEPPYL_ENV").is_ok_and(|v|v.eq_ignore_ascii_case("production")){return false;}
    if std::env::current_dir().is_ok_and(|p|p==std::path::Path::new("/var/lib/seppyl")){return false;}
    std::env::var("SEPPYL_DISCORD_LOCAL_BRIDGE_TOKEN")
        .is_ok_and(|token|token.len()>=32 && token.len()<=256 && token.is_ascii())
}
pub(super) fn configured()->bool{
    let ids=["SEPPYL_DISCORD_GUILD_ID","SEPPYL_DISCORD_APPLICATION_ID"]
        .iter().all(|key|std::env::var(key).is_ok_and(|v|!v.is_empty()));
    ids && (std::env::var("SEPPYL_DISCORD_PUBLIC_KEY").is_ok_and(|key|!key.is_empty())
        || local_gateway_enabled())
}
fn require_staff(role:Role)->R<()>{
    if role.is_admin(){Ok(())}else{Err(StatusCode::FORBIDDEN)}
}
#[derive(Serialize)]
pub struct DiscordStatus{
    pub enabled:bool,
    pub discord_user_id:Option<String>,
}
#[derive(Serialize)]
pub struct DiscordCode{
    pub code:String,
    pub expires_at:i64,
}
pub async fn status(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,
)->R<Json<DiscordStatus>>{
    require_staff(role.0)?;
    let found=state.db.query_one_raw(sql(
        "SELECT discord_user_id FROM support_discord_links WHERE user_id = ?",
        vec![claims.sub.to_string().into()]
    )).await.map_err(db)?;
    let discord_user_id=found.map(|row|row.try_get("","discord_user_id"))
        .transpose().map_err(db)?;
    Ok(Json(DiscordStatus{enabled:configured(),discord_user_id}))
}
pub async fn generate_code(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,
)->R<Json<DiscordCode>>{
    require_staff(role.0)?;
    if !configured(){return Err(StatusCode::SERVICE_UNAVAILABLE);}
    let expires_at=now()+600;
    let code=Uuid::new_v4().simple().to_string();
    let tx=state.db.begin().await.map_err(db)?;
    tx.execute_raw(sql("DELETE FROM support_discord_codes WHERE user_id = ?",
        vec![claims.sub.to_string().into()])).await.map_err(db)?;
    tx.execute_raw(sql("INSERT INTO support_discord_codes(code,user_id,expires_at) VALUES(?,?,?)",
        vec![code.clone().into(),claims.sub.to_string().into(),expires_at.into()]))
        .await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    Ok(Json(DiscordCode{code,expires_at}))
}
pub async fn unlink(
    Extension(claims):Extension<Claims>,
    Extension(role):Extension<CurrentRole>,
    State(state):State<AppState>,
)->R<StatusCode>{
    require_staff(role.0)?;
    let tx=state.db.begin().await.map_err(db)?;
    for table in ["support_discord_codes","support_discord_links"] {
        tx.execute_raw(sql(&format!("DELETE FROM {table} WHERE user_id=?"),
            vec![claims.sub.to_string().into()])).await.map_err(db)?;
    }
    tx.commit().await.map_err(db)?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn staff_for_discord(state:&AppState,discord_id:&str)->R<Option<Uuid>>{
    let row=state.db.query_one_raw(sql(
        "SELECT user_id FROM support_discord_links WHERE discord_user_id=?",
        vec![discord_id.into()]
    )).await.map_err(db)?;
    let Some(row)=row else{return Ok(None)};
    let id:String=row.try_get("","user_id").map_err(db)?;
    let Ok(id)=Uuid::parse_str(&id) else{return Ok(None)};
    let account=user::Entity::find_by_id(id).one(&state.db).await.map_err(db)?;
    Ok(account.and_then(|account|{
        let role=account.role.as_deref().and_then(Role::parse)?;
        if !account.disabled.unwrap_or(false) && role.is_admin(){Some(id)}else{None}
    }))
}
pub(super) async fn redeem_code(state:&AppState,discord_id:&str,code:&str)->R<&'static str>{
    if code.len()!=32 || !code.bytes().all(|b|b.is_ascii_hexdigit()){
        return Ok("Code de liaison invalide.");
    }
    let tx=state.db.begin().await.map_err(db)?;
    let Some(row)=tx.query_one_raw(sql(
        "SELECT user_id,expires_at FROM support_discord_codes WHERE code=?",
        vec![code.to_ascii_lowercase().into()]
    )).await.map_err(db)? else{return Ok("Code expiré, inconnu ou déjà utilisé.")};
    let id:String=row.try_get("","user_id").map_err(db)?;
    let expiry:i64=row.try_get("","expires_at").map_err(db)?;
    if expiry<now(){return Ok("Code expiré. Génère-en un nouveau dans SEPPYL.");}
    let Ok(id)=Uuid::parse_str(&id) else{return Ok("Compte SEPPYL invalide.")};
    let account=user::Entity::find_by_id(id).one(&tx).await.map_err(db)?;
    let allowed=account.is_some_and(|a|!a.disabled.unwrap_or(false)
        && a.role.as_deref().and_then(Role::parse).is_some_and(Role::is_admin));
    if !allowed{return Ok("Ce compte SEPPYL n'a plus les droits nécessaires.");}
    tx.execute_raw(sql(
        "DELETE FROM support_discord_links WHERE user_id=? OR discord_user_id=?",
        vec![id.to_string().into(),discord_id.into()]
    )).await.map_err(db)?;
    tx.execute_raw(sql(
        "INSERT INTO support_discord_links(user_id,discord_user_id,linked_at) VALUES(?,?,?)",
        vec![id.to_string().into(),discord_id.into(),now().into()]
    )).await.map_err(db)?;
    tx.execute_raw(sql("DELETE FROM support_discord_codes WHERE user_id=?",
        vec![id.to_string().into()])).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    Ok("Compte Discord lié. Tu peux désormais gérer les tickets SEPPYL.")
}
pub(super) async fn reserve_interaction(
    tx:&DatabaseTransaction,interaction_id:&str,
)->R<bool>{
    if interaction_id.len()>32 || !interaction_id.bytes().all(|b|b.is_ascii_digit()){
        return Err(StatusCode::BAD_REQUEST);
    }
    let result=tx.execute_raw(sql(
        "INSERT OR IGNORE INTO support_discord_processed(interaction_id,created_at) VALUES(?,?)",
        vec![interaction_id.into(),now().into()]
    )).await.map_err(db)?;
    Ok(result.rows_affected()==1)
}
