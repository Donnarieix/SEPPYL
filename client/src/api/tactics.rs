use reqwest::Client;
use serde::{Serialize,Deserialize};
fn base()->&'static str{option_env!("SEPPYL_API_URL").unwrap_or("http://127.0.0.1:3000")}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct Folder {pub id:String,pub owner_id:String,pub parent_id:Option<String>,pub name:String}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct Board {pub id:String,pub owner_id:String,pub folder_id:Option<String>,pub name:String,pub content:String,pub is_public:bool}
#[derive(Clone,Debug,Deserialize)]
pub struct Library {pub folders:Vec<Folder>,pub boards:Vec<Board>}
#[derive(Serialize)]
struct FolderInput<'a>{name:&'a str,parent_id:Option<&'a str>}
#[derive(Serialize)]
struct BoardInput<'a>{name:&'a str,folder_id:Option<&'a str>,content:&'a str,is_public:bool}
pub const EMPTY:&str=r#"{"pitch":"football","markers":[],"arrows":[]}"#;

pub async fn list(token:&str)->Result<Library,String>{
    let resp=Client::new().get(format!("{}/tactics",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !resp.status().is_success(){return Err(format!("Impossible de charger les schémas (HTTP {}).",crate::session::observe_status(resp.status())))}
    resp.json::<Library>().await.map_err(|e|e.to_string())
}
pub async fn create_folder(token:&str,name:&str,parent_id:Option<&str>)->Result<(),String>{
    let resp=Client::new().post(format!("{}/tactics/folders",base()))
        .bearer_auth(token).json(&FolderInput{name,parent_id}).send().await.map_err(|e|e.to_string())?;
    if resp.status().is_success(){Ok(())}else{Err(format!("Création du dossier refusée (HTTP {}).",crate::session::observe_status(resp.status())))}
}
pub async fn save(token:&str,id:Option<&str>,name:&str,folder_id:Option<&str>,
        content:&str,is_public:bool)->Result<(),String>{
    let client=Client::new();
    let req=if let Some(id)=id{client.put(format!("{}/tactics/{id}",base()))}
      else{client.post(format!("{}/tactics",base()))};
    let resp=req.bearer_auth(token).json(&BoardInput{name,folder_id,content,is_public})
      .send().await.map_err(|e|e.to_string())?;
    if resp.status().is_success(){Ok(())}else{Err(format!("Enregistrement refusé (HTTP {}).",crate::session::observe_status(resp.status())))}
}
pub async fn delete(token:&str,id:&str)->Result<(),String>{
    let resp=Client::new().delete(format!("{}/tactics/{id}",base()))
       .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    if resp.status().is_success(){Ok(())}else{Err(format!("Suppression refusée (HTTP {}).",crate::session::observe_status(resp.status())))}
}

pub async fn public_get(id:&str)->Result<Board,String>{
    let response=Client::new().get(format!("{}/public/tactics/{id}",base()))
        .send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){return Err("Schéma introuvable ou privé.".into())}
    response.json::<Board>().await.map_err(|e|e.to_string())
}
