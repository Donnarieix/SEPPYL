use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

fn api_url() -> &'static str {
    option_env!("SEPPYL_API_URL").unwrap_or(if cfg!(debug_assertions) {
        "http://127.0.0.1:3000"
    } else {
        "/api"
    })
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Player {
    pub id: String,
    pub firstname: String,
    pub lastname: String,
    pub number: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub date: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ActivityResult {
    pub player_id: String,
    pub activity_id: String,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Matrix {
    pub players: Vec<Player>,
    pub activities: Vec<Activity>,
    pub results: Vec<ActivityResult>,
}

#[derive(Serialize)]
struct CellUpdate<'a> {
    player_id: &'a str,
    activity_id: &'a str,
    result: Option<&'a str>,
}

pub async fn load(token: &str) -> Result<Matrix, String> {
    let response = Client::new()
        .get(format!("{}/matrix", api_url()))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| format!("Impossible de joindre l'API : {error}"))?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Err("Session expirée. Reconnectez-vous.".into());
    }
    if !response.status().is_success() {
        return Err(format!("Chargement impossible (HTTP {}).", response.status()));
    }
    response.json::<Matrix>().await.map_err(|error| error.to_string())
}

pub async fn save(
    token: &str,
    player_id: &str,
    activity_id: &str,
    result: &str,
) -> Result<(), String> {
    let response = Client::new()
        .put(format!("{}/matrix/result", api_url()))
        .bearer_auth(token)
        .json(&CellUpdate {
            player_id,
            activity_id,
            result: if result.trim().is_empty() { None } else { Some(result.trim()) },
        })
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Sauvegarde impossible (HTTP {}).", response.status()));
    }
    Ok(())
}

#[derive(Serialize)]
struct Credentials<'a> {
    username: &'a str,
    password: &'a str,
}
#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

pub async fn login(username: &str, password: &str) -> Result<String, String> {
    let response = Client::new()
        .post(format!("{}/login", api_url()))
        .json(&Credentials { username, password })
        .send()
        .await
        .map_err(|error| format!("API inaccessible : {error}"))?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Err("Identifiant ou mot de passe incorrect.".into());
    }
    if !response.status().is_success() {
        return Err(format!("Connexion impossible (HTTP {}).", response.status()));
    }
    response.json::<LoginResponse>().await
        .map(|response| response.token)
        .map_err(|error| error.to_string())
}


#[derive(Serialize)]
struct NewPlayer<'a> {
    firstname: &'a str,
    lastname: &'a str,
    number: u32,
}

pub async fn create_player(
    token: &str,
    firstname: &str,
    lastname: &str,
    number: u32,
) -> Result<(), String> {
    let response = Client::new()
        .post(format!("{}/player/create", api_url()))
        .bearer_auth(token)
        .json(&NewPlayer { firstname, lastname, number })
        .send()
        .await
        .map_err(|error| format!("Impossible de contacter l'API : {error}"))?;

    match response.status() {
        StatusCode::CREATED => Ok(()),
        StatusCode::UNAUTHORIZED => Err("Session expirée. Reconnectez-vous.".into()),
        StatusCode::FORBIDDEN => Err("Vous n'avez pas le droit d'ajouter cette joueuse.".into()),
        StatusCode::CONFLICT => Err("Cette joueuse existe déjà.".into()),
        StatusCode::INTERNAL_SERVER_ERROR =>
            Err("Création impossible. Vérifiez notamment si ce prénom existe déjà dans la base.".into()),
        status => Err(format!("Impossible de créer la joueuse (HTTP {status}).")),
    }
}


/// Create a separate account (e.g. Valentin and another coach).
/// Each account has private data until shared-team support is added.
pub async fn register(username: &str, password: &str) -> Result<(), String> {
    let response = Client::new()
        .post(format!("{}/register", api_url()))
        .json(&Credentials { username, password })
        .send()
        .await
        .map_err(|error| format!("API inaccessible : {error}"))?;
    match response.status() {
        StatusCode::OK => Ok(()),
        StatusCode::CONFLICT => Err("Cet identifiant existe déjà.".into()),
        status => Err(format!("Inscription impossible (HTTP {status}).")),
    }
}
