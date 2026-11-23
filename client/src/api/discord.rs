use reqwest::{Client,StatusCode};
use serde::Deserialize;

fn endpoint(path:&str)->String { format!("{}{}",super::base_url(),path) }
fn error(status:StatusCode)->String {
    match status {
        StatusCode::FORBIDDEN=>"Réservé aux administrateurs et maintainers.".into(),
        StatusCode::SERVICE_UNAVAILABLE=>"Intégration Discord non configurée sur le serveur.".into(),
        _=>format!("Erreur HTTP {status}"),
    }
}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct LinkStatus {
    pub enabled:bool,
    pub discord_user_id:Option<String>,
}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct LinkCode {
    pub code:String,
    pub expires_at:i64,
}
pub async fn status(token:&str)->Result<LinkStatus,String>{
    let response=Client::new().get(endpoint("/account/discord")).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){return Err(error(response.status()));}
    response.json().await.map_err(|e|e.to_string())
}
pub async fn link_code(token:&str)->Result<LinkCode,String>{
    let response=Client::new().post(endpoint("/account/discord/link-code")).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){return Err(error(response.status()));}
    response.json().await.map_err(|e|e.to_string())
}
pub async fn unlink(token:&str)->Result<(),String>{
    let response=Client::new().delete(endpoint("/account/discord")).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){return Err(error(response.status()));}
    Ok(())
}
