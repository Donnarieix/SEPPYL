use reqwest::{Client,StatusCode};
use serde::Deserialize;

fn url(path:&str)->String{format!("{}{}",super::base_url(),path)}
fn error(status:StatusCode)->String{
    match status {
        StatusCode::FORBIDDEN=>"Compte Administrateur ou Maintainer requis.".into(),
        StatusCode::SERVICE_UNAVAILABLE=>"Discord n'est pas encore configuré côté serveur.".into(),
        _=>format!("Impossible de contacter SEPPYL (HTTP {status})."),
    }
}
#[derive(Clone,Debug,Deserialize)]
pub struct DiscordStatus{pub enabled:bool,pub discord_user_id:Option<String>}
#[derive(Clone,Debug,Deserialize)]
pub struct DiscordCode{pub code:String,pub expires_at:i64}
pub async fn status(token:&str)->Result<DiscordStatus,String>{
    let reply=Client::new().get(url("/account/discord")).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !reply.status().is_success(){return Err(error(reply.status()));}
    reply.json().await.map_err(|e|e.to_string())
}
pub async fn new_code(token:&str)->Result<DiscordCode,String>{
    let reply=Client::new().post(url("/account/discord/code")).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !reply.status().is_success(){return Err(error(reply.status()));}
    reply.json().await.map_err(|e|e.to_string())
}
pub async fn unlink(token:&str)->Result<(),String>{
    let reply=Client::new().delete(url("/account/discord")).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !reply.status().is_success(){return Err(error(reply.status()));}
    Ok(())
}
