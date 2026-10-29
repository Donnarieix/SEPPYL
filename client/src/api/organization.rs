use reqwest::{Client, StatusCode};
use serde::{Deserialize,Serialize};
fn base()->&'static str{ option_env!("SEPPYL_API_URL").unwrap_or("http://127.0.0.1:3000") }
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct Club{pub id:String,pub name:String,pub is_public:Option<bool>}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct Team{pub id:String,pub club_id:String,pub name:String}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct User{pub id:String,pub username:String,pub role:String}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct ClubMember{pub club_id:String,pub user_id:String,pub role:String}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct TeamMember{pub team_id:String,pub user_id:String,pub role:String}
#[derive(Serialize)]
struct Named<'a>{name:&'a str}
#[derive(Serialize)]
struct RolePayload<'a>{role:&'a str}
fn check(status:StatusCode)->Result<(),String>{
    match status {
        StatusCode::FORBIDDEN=>Err("Vous ne disposez pas des droits nécessaires.".into()),
        _ if status.is_success()=>Ok(()),
        _=>Err(format!("Erreur API (HTTP {status})")),
    }
}
pub async fn clubs(token:&str)->Result<Vec<Club>,String>{
    let r=Client::new().get(format!("{}/clubs",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())?;r.json().await.map_err(|e|e.to_string())
}
pub async fn teams(token:&str,club_id:&str)->Result<Vec<Team>,String>{
    let r=Client::new().get(format!("{}/clubs/{club_id}/teams",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())?;r.json().await.map_err(|e|e.to_string())
}
pub async fn users(token:&str)->Result<Vec<User>,String>{
    let r=Client::new().get(format!("{}/admin/users",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())?;r.json().await.map_err(|e|e.to_string())
}
pub async fn create_club(token:&str,name:&str)->Result<(),String>{
    let r=Client::new().post(format!("{}/clubs",base())).bearer_auth(token)
        .json(&Named{name}).send().await.map_err(|e|e.to_string())?;
    check(r.status())
}
pub async fn create_team(token:&str,club_id:&str,name:&str)->Result<(),String>{
    let r=Client::new().post(format!("{}/clubs/{club_id}/teams",base())).bearer_auth(token)
        .json(&Named{name}).send().await.map_err(|e|e.to_string())?;
    check(r.status())
}
pub async fn club_members(token:&str,id:&str)->Result<Vec<ClubMember>,String>{
    let r=Client::new().get(format!("{}/clubs/{id}/members",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())?;r.json().await.map_err(|e|e.to_string())
}
pub async fn team_members(token:&str,id:&str)->Result<Vec<TeamMember>,String>{
    let r=Client::new().get(format!("{}/teams/{id}/members",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())?;r.json().await.map_err(|e|e.to_string())
}
pub async fn grant_club(token:&str,club_id:&str,user_id:&str,role:&str)->Result<(),String>{
    let r=Client::new().put(format!("{}/clubs/{club_id}/members/{user_id}",base()))
        .bearer_auth(token).json(&RolePayload{role})
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())
}
pub async fn grant_team(token:&str,team_id:&str,user_id:&str,role:&str)->Result<(),String>{
    let r=Client::new().put(format!("{}/teams/{team_id}/members/{user_id}",base()))
        .bearer_auth(token).json(&RolePayload{role})
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())
}
pub async fn assign_global_role(token:&str,user_id:&str,role:&str)->Result<(),String>{
    let r=Client::new().put(format!("{}/admin/users/{user_id}/role",base()))
        .bearer_auth(token).json(&RolePayload{role})
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())
}
pub async fn remove_member(token:&str,scope:&str,id:&str,user_id:&str)->Result<(),String>{
    let group=if scope=="club"{"clubs"}else{"teams"};
    let r=Client::new().delete(format!("{}/{group}/{id}/members/{user_id}",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r.status())
}

#[derive(Serialize)]
struct Visibility{is_public:bool}
pub async fn set_public(token:&str,club_id:&str,is_public:bool)->Result<(),String>{
    let response=Client::new().put(format!("{}/clubs/{club_id}/visibility",base()))
        .bearer_auth(token).json(&Visibility{is_public})
        .send().await.map_err(|e|e.to_string())?;
    check(response.status())
}
pub async fn public_clubs()->Result<Vec<Club>,String>{
    let response=Client::new().get(format!("{}/public/clubs",base()))
        .send().await.map_err(|e|e.to_string())?;
    check(response.status())?;
    response.json().await.map_err(|e|e.to_string())
}
pub async fn public_teams(club_id:&str)->Result<Vec<Team>,String>{
    let response=Client::new().get(format!("{}/public/clubs/{club_id}/teams",base()))
        .send().await.map_err(|e|e.to_string())?;
    check(response.status())?;
    response.json().await.map_err(|e|e.to_string())
}
pub async fn team_players(token:&str,team_id:&str)->Result<Vec<crate::api::players::Player>,String>{
    let response=Client::new().get(format!("{}/teams/{team_id}/players",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(response.status())?;
    response.json().await.map_err(|e|e.to_string())
}
pub async fn add_team_player(token:&str,team_id:&str,player_id:&str)->Result<(),String>{
    let r=Client::new().put(format!("{}/teams/{team_id}/players/{player_id}",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r.status())
}
pub async fn remove_team_player(token:&str,team_id:&str,player_id:&str)->Result<(),String>{
    let r=Client::new().delete(format!("{}/teams/{team_id}/players/{player_id}",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r.status())
}

pub async fn eligible_users(token:&str,club_id:&str)->Result<Vec<User>,String>{
    let r=Client::new().get(format!("{}/clubs/{club_id}/eligible-users",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r.status())?;r.json().await.map_err(|e|e.to_string())
}

#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct ClubCapabilities{
    pub can_manage:bool,
    pub can_create_team:bool,
    pub can_write_teams:Vec<String>,
}
pub async fn capabilities(token:&str,club_id:&str)->Result<ClubCapabilities,String>{
    let r=Client::new().get(format!("{}/clubs/{club_id}/capabilities",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r.status())?;r.json().await.map_err(|e|e.to_string())
}
