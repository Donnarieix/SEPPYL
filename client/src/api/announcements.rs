use reqwest::Client;
use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Announcement{pub id:String,pub title:String,pub body:String,pub urgent:bool,
    pub published:bool,pub starts_at:i64,pub ends_at:Option<i64>,pub created_at:i64}
#[derive(Serialize)]
pub struct Payload<'a>{pub title:&'a str,pub body:&'a str,pub urgent:bool,
    pub published:bool,pub starts_at:i64,pub ends_at:Option<i64>}
fn url(path:&str)->String{format!("{}{}",super::base_url(),path)}
async fn check(r:reqwest::Response)->Result<reqwest::Response,String>{
    if r.status().is_success(){Ok(r)}else{Err(format!("Erreur API (HTTP {})",r.status()))}
}
pub async fn active()->Result<Vec<Announcement>,String>{
    let r=Client::new().get(url("/announcements")).send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
pub async fn all(token:&str)->Result<Vec<Announcement>,String>{
    let r=Client::new().get(url("/admin/announcements")).bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
pub async fn create(token:&str,payload:Payload<'_>)->Result<(),String>{
    let r=Client::new().post(url("/admin/announcements")).bearer_auth(token)
        .json(&payload).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
pub async fn update(token:&str,id:&str,payload:Payload<'_>)->Result<(),String>{
    let r=Client::new().put(url(&format!("/admin/announcements/{id}"))).bearer_auth(token)
        .json(&payload).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
pub async fn remove(token:&str,id:&str)->Result<(),String>{
    let r=Client::new().delete(url(&format!("/admin/announcements/{id}"))).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
