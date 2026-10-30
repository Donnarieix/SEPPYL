use reqwest::{Client,StatusCode};
use serde::{Deserialize,Serialize};

fn base()->&'static str{
    option_env!("SEPPYL_API_URL").unwrap_or("http://127.0.0.1:3000")
}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct Entry {
    pub version:String,pub title:String,pub body:String,
    pub published_at:i64,pub updated_at:i64,
}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct Status {pub version:String,pub unread:bool}
#[derive(Serialize)]
struct Changes<'a>{title:&'a str,body:&'a str}
pub async fn list(token:&str)->Result<Vec<Entry>,String>{
    let response=Client::new().get(format!("{}/changelog",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){
        return Err(format!("Historique indisponible (HTTP {}).",response.status()));
    }
    response.json().await.map_err(|e|e.to_string())
}
pub async fn status(token:&str)->Result<Status,String>{
    let response=Client::new().get(format!("{}/changelog/status",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){
        return Err(format!("État des nouveautés indisponible (HTTP {}).",response.status()));
    }
    response.json().await.map_err(|e|e.to_string())
}
pub async fn read(token:&str,version:&str)->Result<(),String>{
    let response=Client::new().post(format!("{}/changelog/{version}/read",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    match response.status(){
        StatusCode::NO_CONTENT=>Ok(()),
        code=>Err(format!("Lecture non enregistrée (HTTP {code}).")),
    }
}
pub async fn save(token:&str,version:&str,title:&str,body:&str)->Result<Entry,String>{
    let response=Client::new().put(format!("{}/changelog/{version}",base()))
        .bearer_auth(token)
        .json(&Changes{title,body}).send().await.map_err(|e|e.to_string())?;
    match response.status(){
        StatusCode::FORBIDDEN=>Err("Modification réservée aux mainteneurs.".into()),
        StatusCode::BAD_REQUEST=>Err("Vérifiez la version, le titre et le contenu.".into()),
        s if !s.is_success()=>Err(format!("Modification impossible (HTTP {s}).")),
        _=>response.json().await.map_err(|e|e.to_string()),
    }
}
