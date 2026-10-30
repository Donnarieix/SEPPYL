use reqwest::{Client,StatusCode};
use serde::{Deserialize,Serialize};
fn base()->&'static str{super::base_url()}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct DrawingObject {
    pub id:String,pub kind:String,pub x:f64,pub y:f64,
    pub end_x:Option<f64>,pub end_y:Option<f64>,pub label:Option<String>,
    #[serde(default)] pub color:Option<String>,
    #[serde(default)] pub size:Option<f64>,
    #[serde(default)] pub control_x:Option<f64>,
    #[serde(default)] pub control_y:Option<f64>,
    #[serde(default)] pub dashed:bool,
}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
pub struct Diagram { pub id:String,pub name:String,pub layout:String,pub objects:Vec<DrawingObject> }
#[derive(Serialize)]
struct Payload<'a>{name:&'a str,layout:&'a str,objects:&'a [DrawingObject]}
fn checked(status:StatusCode)->Result<(),String>{
    if status.is_success(){Ok(())}else{Err(format!("Erreur du tableau tactique (HTTP {status})."))}
}
pub async fn list(token:&str)->Result<Vec<Diagram>,String>{
    let r=Client::new().get(format!("{}/tactics",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    checked(r.status())?;
    r.json().await.map_err(|e|e.to_string())
}
pub async fn save(token:&str,id:Option<&str>,name:&str,layout:&str,objects:&[DrawingObject])->Result<Diagram,String>{
    let client=Client::new();
    let req=match id {
        Some(id)=>client.put(format!("{}/tactics/{id}",base())),
        None=>client.post(format!("{}/tactics",base())),
    };
    let r=req.bearer_auth(token).json(&Payload{name,layout,objects})
        .send().await.map_err(|e|e.to_string())?;
    checked(r.status())?;
    r.json().await.map_err(|e|e.to_string())
}
pub async fn remove(token:&str,id:&str)->Result<(),String>{
    let r=Client::new().delete(format!("{}/tactics/{id}",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    checked(r.status())
}
