use reqwest::{Client, StatusCode};
use serde::{Serialize,Deserialize};
pub fn url()->&'static str { option_env!("SEPPYL_API_URL").unwrap_or("http://127.0.0.1:3000") }
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct Player { pub id:String, pub firstname:String, pub lastname:String, pub number:u32 }
#[derive(Serialize)]
struct Create<'a> { firstname:&'a str, lastname:&'a str, number:u32 }
#[derive(Serialize)]
struct Update<'a> { id:&'a str, firstname:Option<&'a str>, lastname:Option<&'a str>, number:Option<u32> }
pub async fn list(token:&str)->Result<Vec<Player>,String>{
    let response=Client::new().get(format!("{}/player/list",url()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){return Err(format!("Chargement impossible (HTTP {}).",response.status()));}
    response.json::<Vec<Player>>().await.map_err(|e|e.to_string())
}
pub async fn save(token:&str,id:Option<&str>,firstname:&str,lastname:&str,number:u32)->Result<(),String>{
    let client=Client::new();
    let req=if let Some(id)=id {
        client.put(format!("{}/player/update",url())).bearer_auth(token)
            .json(&Update{id,firstname:Some(firstname),lastname:Some(lastname),number:Some(number)})
    } else {
        client.post(format!("{}/player/create",url())).bearer_auth(token)
            .json(&Create{firstname,lastname,number})
    };
    let res=req.send().await.map_err(|e|e.to_string())?;
    if res.status().is_success(){Ok(())} else {Err(format!("Impossible d'enregistrer la joueuse (HTTP {}).",res.status()))}
}
pub async fn delete(token:&str,id:&str)->Result<(),String>{
    let res=Client::new().delete(format!("{}/player/delete/{id}",url()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    match res.status(){
        status if status.is_success()=>Ok(()),
        StatusCode::CONFLICT=>Err("Cette joueuse possède des présences ou des notes : suppression bloquée pour préserver l'historique.".into()),
        status=>Err(format!("Suppression impossible (HTTP {status}).")),
    }
}
