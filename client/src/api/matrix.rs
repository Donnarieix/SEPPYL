use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

const API_URL: &str = "http://127.0.0.1:3000";

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
        .get(format!("{API_URL}/matrix"))
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
        .put(format!("{API_URL}/matrix/result"))
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
        .post(format!("{API_URL}/login"))
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
