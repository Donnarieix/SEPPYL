use reqwest::{Client,StatusCode};
use serde::{Serialize,Deserialize};
fn url(path:&str)->String{format!("{}{}",super::base_url(),path)}
async fn check(r:reqwest::Response)->Result<reqwest::Response,String>{
    let s=r.status();
    if s.is_success(){Ok(r)}else{Err(match s{
        StatusCode::FORBIDDEN=>"Opération non autorisée".into(),
        StatusCode::NOT_FOUND=>"Ticket introuvable".into(),
        StatusCode::CONFLICT=>"Ticket fermé".into(),
        _=>format!("Erreur API HTTP {s}")})}
}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Ticket{pub id:String,pub author_id:String,pub subject:String,pub status:String,
    pub maintainer_required:bool,pub created_at:i64,pub updated_at:i64}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Message{pub id:String,pub author_id:String,pub message:String,pub created_at:i64}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Detail{pub ticket:Ticket,pub messages:Vec<Message>}
#[derive(Serialize)]
struct Create<'a>{subject:&'a str,message:&'a str}
#[derive(Serialize)]
struct Reply<'a>{message:&'a str}
#[derive(Serialize)]
struct Status<'a>{status:&'a str,maintainer_required:bool}
pub async fn list(token:&str)->Result<Vec<Ticket>,String>{
    let r=Client::new().get(url("/tickets")).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
pub async fn get(token:&str,id:&str)->Result<Detail,String>{
    let r=Client::new().get(url(&format!("/tickets/{id}"))).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
pub async fn create(token:&str,subject:&str,message:&str)->Result<Ticket,String>{
    let r=Client::new().post(url("/tickets")).bearer_auth(token)
        .json(&Create{subject,message}).send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
pub async fn reply(token:&str,id:&str,message:&str)->Result<(),String>{
    let r=Client::new().post(url(&format!("/tickets/{id}/messages"))).bearer_auth(token)
        .json(&Reply{message}).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
pub async fn status(token:&str,id:&str,status:&str,maintainer_required:bool)->Result<(),String>{
    let r=Client::new().put(url(&format!("/tickets/{id}/status"))).bearer_auth(token)
        .json(&Status{status,maintainer_required}).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
