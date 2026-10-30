use reqwest::{Client, StatusCode};
use serde::{Deserialize,Serialize};
fn base()->&'static str{ option_env!("SEPPYL_API_URL").unwrap_or("http://127.0.0.1:3000") }
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct Club{
    pub id:String,pub name:String,pub is_public:Option<bool>,
    #[serde(default)]pub description:Option<String>,
    #[serde(default)]pub logo_url:Option<String>,
    #[serde(default)]pub logo_x:Option<i32>,
    #[serde(default)]pub logo_y:Option<i32>,
    #[serde(default)]pub logo_zoom:Option<i32>,
}
pub fn club_logo_src(logo:Option<&str>)->Option<String>{
    let src=logo?.trim();
    if src.starts_with("/catalog/images/"){return Some(format!("{}{}",base(),src));}
    if src.starts_with("https://"){return Some(src.to_string());}
    None
}
#[derive(Clone,Debug,Deserialize,PartialEq)]
pub struct Team{
    pub id:String,pub club_id:String,pub name:String,
    #[serde(default)]pub description:Option<String>,
    #[serde(default)]pub logo_url:Option<String>,
    #[serde(default)]pub logo_x:Option<i32>,
    #[serde(default)]pub logo_y:Option<i32>,
    #[serde(default)]pub logo_zoom:Option<i32>,
}
pub fn logo_style(x:Option<i32>,y:Option<i32>,zoom:Option<i32>)->String {
    super::super::routes::logo_picker::image_style(x.unwrap_or(0),y.unwrap_or(0),zoom.unwrap_or(100))
}
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
        StatusCode::CONFLICT=>Err("Une fiche avec le même prénom, nom et numéro existe déjà, ou une autre opération empêche la modification.".into()),
        _ if status.is_success()=>Ok(()),
        _=>Err(format!("Erreur API (HTTP {status})")),
    }
}
pub async fn admin_clubs(token:&str)->Result<Vec<Club>,String>{
    let response=Client::new().get(format!("{}/admin/clubs",base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    check(response.status())?;
    response.json().await.map_err(|e|e.to_string())
}
#[derive(Serialize)]
struct ClubInformation<'a>{
    name:&'a str,description:&'a str,logo_url:Option<&'a str>,is_public:bool,
    logo_x:i32,logo_y:i32,logo_zoom:i32,
}
pub async fn update_club_information(
    token:&str,id:&str,name:&str,description:&str,logo_url:Option<&str>,is_public:bool,
    logo_x:i32,logo_y:i32,logo_zoom:i32,
)->Result<(),String>{
    let response=Client::new().put(format!("{}/clubs/{id}/information",base()))
        .bearer_auth(token)
        .json(&ClubInformation{name,description,logo_url,is_public,logo_x,logo_y,logo_zoom})
        .send().await.map_err(|e|e.to_string())?;
    check(response.status())
}
#[derive(Serialize)]
struct TeamInformation<'a>{
    name:&'a str,description:&'a str,logo_url:Option<&'a str>,
    logo_x:i32,logo_y:i32,logo_zoom:i32,
}
pub async fn update_team_information(
    token:&str,id:&str,name:&str,description:&str,logo_url:Option<&str>,
    x:i32,y:i32,zoom:i32,
)->Result<(),String>{
    let response=Client::new().put(format!("{}/teams/{id}/information",base()))
        .bearer_auth(token)
        .json(&TeamInformation{name,description,logo_url,logo_x:x,logo_y:y,logo_zoom:zoom})
        .send().await.map_err(|error|error.to_string())?;
    check(response.status())
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
    if r.status()==StatusCode::CONFLICT {
        return Err("Cette joueuse appartient déjà à l'effectif de cette équipe.".into());
    }
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

pub async fn rename_team(token:&str,id:&str,name:&str)->Result<(),String>{
    let r=Client::new().put(format!("{}/teams/{id}",base()))
        .bearer_auth(token).json(&Named{name}).send().await.map_err(|e|e.to_string())?;
    check(r.status())
}
pub async fn delete_team(token:&str,id:&str)->Result<(),String>{
    let r=Client::new().delete(format!("{}/teams/{id}",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    match r.status(){
        StatusCode::CONFLICT=>Err("Suppression impossible : cette équipe possède des entraînements ou des activités. Son historique doit être conservé.".into()),
        status=>check(status)
    }
}
#[derive(Serialize)]
struct NewPlayer<'a>{firstname:&'a str,lastname:&'a str,number:u32}
pub async fn create_team_player(token:&str,id:&str,firstname:&str,lastname:&str,number:u32)->Result<(),String>{
    let r=Client::new().post(format!("{}/teams/{id}/players",base()))
        .bearer_auth(token).json(&NewPlayer{firstname,lastname,number})
        .send().await.map_err(|e|e.to_string())?;
    check(r.status())
}

pub async fn club_players(token:&str,club_id:&str)->Result<Vec<crate::api::players::Player>,String>{
    let r=Client::new().get(format!("{}/clubs/{club_id}/players",base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?;
    check(r.status())?;r.json().await.map_err(|e|e.to_string())
}
