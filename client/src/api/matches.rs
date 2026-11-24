use reqwest::{Client,StatusCode};
use serde::{Deserialize,Serialize};
fn url(path:&str)->String{format!("{}{}",super::base_url(),path)}
fn client()->Client{Client::new()}
async fn check(r:reqwest::Response)->Result<reqwest::Response,String>{
    let status=r.status();
    if status.is_success(){return Ok(r);}
    Err(match status{StatusCode::FORBIDDEN=>"Droits insuffisants".into(),
        StatusCode::NOT_FOUND=>"Match introuvable ou inaccessible".into(),
        StatusCode::BAD_REQUEST=>"Chronologie ou informations de match invalides".into(),
        _=>format!("Erreur API : {status}")})
}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Game{pub id:String,pub team_id:String,pub opponent:String,pub date:String,pub half_minutes:i32,pub notes:String}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Participant{pub player_id:String,pub position:String,pub starter:bool}
#[derive(Clone,Debug,PartialEq,Deserialize,Serialize)]
pub struct Event{pub id:String,pub kind:String,pub minute:i32,pub player_id:Option<String>,
    pub player_in_id:Option<String>,pub event_type_id:Option<String>,pub notes:String}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct EventType{pub id:String,pub label:String,pub kind:String}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct PlayingTime{pub player_id:String,pub minutes:i32}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Detail{pub game:Game,pub participants:Vec<Participant>,pub events:Vec<Event>,
    pub event_types:Vec<EventType>,pub playing_times:Vec<PlayingTime>}
#[derive(Serialize)]
struct NewGame<'a>{team_id:&'a str,opponent:&'a str,date:String,half_minutes:i32}
#[derive(Serialize)]
struct NewEvent<'a>{minute:i32,kind:&'a str,event_type_id:Option<&'a str>,
    player_id:Option<&'a str>,player_in_id:Option<&'a str>,notes:&'a str}
#[derive(Serialize)]
struct NewType<'a>{label:&'a str}
#[derive(Serialize)]
struct NewLineup<'a>{players:&'a [LineupPlayer]}
#[derive(Clone,Debug,Serialize)]
pub struct LineupPlayer{pub player_id:String,pub position:String,pub starter:bool}
pub async fn list(token:&str)->Result<Vec<Game>,String>{
    let r=client().get(url("/matches")).bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
pub async fn get(token:&str,id:&str)->Result<Detail,String>{
    let r=client().get(url(&format!("/matches/{id}"))).bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
pub async fn create(token:&str,team_id:&str,opponent:&str,date:&str,half_minutes:i32)->Result<Game,String>{
    let r=client().post(url("/matches")).bearer_auth(token).json(&NewGame{
        team_id,opponent,date:format!("{date}T00:00:00"),half_minutes
    }).send().await.map_err(|e|e.to_string())?;
    check(r).await?.json().await.map_err(|e|e.to_string())
}
pub async fn lineup(token:&str,id:&str,players:&[LineupPlayer])->Result<(),String>{
    let r=client().put(url(&format!("/matches/{id}/lineup"))).bearer_auth(token)
        .json(&NewLineup{players}).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
pub async fn event(token:&str,id:&str,minute:i32,kind:&str,event_type_id:Option<&str>,
    player_id:Option<&str>,player_in_id:Option<&str>,notes:&str)->Result<(),String>{
    let r=client().post(url(&format!("/matches/{id}/events"))).bearer_auth(token)
        .json(&NewEvent{minute,kind,event_type_id,player_id,player_in_id,notes})
        .send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
pub async fn remove_event(token:&str,id:&str,event_id:&str)->Result<(),String>{
    let r=client().delete(url(&format!("/matches/{id}/events/{event_id}")))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
pub async fn custom_type(token:&str,id:&str,label:&str)->Result<(),String>{
    let r=client().post(url(&format!("/matches/{id}/event-types")))
        .bearer_auth(token).json(&NewType{label}).send().await.map_err(|e|e.to_string())?;
    check(r).await.map(|_|())
}
