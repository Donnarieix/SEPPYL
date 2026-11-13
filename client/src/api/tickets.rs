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
pub struct Message{pub id:String,pub author_id:String,pub message:String,pub created_at:i64,
    #[serde(default)]pub edited_at:Option<i64>,#[serde(default)]pub deleted_at:Option<i64>,
    #[serde(default)]pub attachment_name:Option<String>}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Detail{pub ticket:Ticket,pub requester_name:String,pub messages:Vec<Message>}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct UnreadTicket{pub ticket_id:String,pub count:u64}
#[derive(Clone,Debug,PartialEq,Deserialize,Default)]
pub struct UnreadSummary{pub total:u64,pub tickets:Vec<UnreadTicket>}
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

pub async fn unread(token:&str)->Result<UnreadSummary,String>{
    let r=Client::new().get(url("/tickets/unread")).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}

#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct MessageRevision {
    pub id:String,pub ticket_id:String,pub message_id:String,pub edited_by:String,
    pub previous_text:String,pub previous_attachment_name:Option<String>,
    pub action:String,pub changed_at:i64,
}
pub async fn edit_message(token:&str,ticket_id:&str,message_id:&str,message:&str)->Result<(),String>{
    let r=Client::new().put(url(&format!("/tickets/{ticket_id}/messages/{message_id}")))
        .bearer_auth(token).json(&Reply{message}).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
pub async fn delete_message(token:&str,ticket_id:&str,message_id:&str)->Result<(),String>{
    let r=Client::new().delete(url(&format!("/tickets/{ticket_id}/messages/{message_id}")))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
pub async fn message_history(token:&str,ticket_id:&str,message_id:&str)->Result<Vec<MessageRevision>,String>{
    let r=Client::new().get(url(&format!("/tickets/{ticket_id}/messages/{message_id}/history")))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
