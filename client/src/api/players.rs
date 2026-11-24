use reqwest::{Client, StatusCode};
use serde::{Serialize,Deserialize};
pub fn url()->&'static str { super::base_url() }
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
    match res.status(){
        status if status.is_success()=>Ok(()),
        StatusCode::BAD_REQUEST=>Err("Vérifiez le prénom, le nom et le numéro de la joueuse.".into()),
        StatusCode::FORBIDDEN=>Err("Vous n'avez pas les droits pour modifier cette joueuse dans toutes ses équipes.".into()),
        StatusCode::NOT_FOUND=>Err("Cette fiche n'existe plus ou n'est plus accessible.".into()),
        StatusCode::CONFLICT=>Err("Une joueuse possède déjà exactement ce prénom, ce nom et ce numéro. Ouvrez sa fiche existante.".into()),
        StatusCode::INTERNAL_SERVER_ERROR=>Err("L'enregistrement a échoué côté serveur. Consultez les logs de l'API pour connaître la cause.".into()),
        status=>Err(format!("Impossible d'enregistrer la joueuse (HTTP {status}).")),
    }
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

#[derive(Clone,Debug,PartialEq,serde::Deserialize)]
pub struct TimeTotals{
    pub player_id:String,
    pub training_minutes:i64,
    pub match_minutes:i64,
    pub training_attended:i32,
    pub trainings_without_duration:i32,
    pub matches_played:i32,
}
pub async fn time_totals(token:&str,id:&str)->Result<TimeTotals,String>{
    let r=reqwest::Client::new().get(format!("{}/players/{id}/time-totals",super::base_url()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    if !r.status().is_success(){return Err(format!("Erreur API (HTTP {})",r.status()));}
    r.json().await.map_err(|e|e.to_string())
}
