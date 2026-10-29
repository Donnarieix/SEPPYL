//! Local account endpoints. The authentication UI can later be delegated to SSO.
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

fn url() -> &'static str { option_env!("SEPPYL_API_URL").unwrap_or(if cfg!(debug_assertions) {
        "http://127.0.0.1:3000"
    } else {
        "/api"
    }) }

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub provider: String,
}

#[derive(Serialize)]
struct ProfileUpdate<'a> { username: &'a str }
#[derive(Serialize)]
struct PasswordUpdate<'a> { current_password: &'a str, new_password: &'a str }

pub async fn get(token: &str) -> Result<Account, String> {
    let response = Client::new().get(format!("{}/account", url()))
        .bearer_auth(token).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Compte indisponible (HTTP {}).", response.status()));
    }
    response.json::<Account>().await.map_err(|e| e.to_string())
}
pub async fn update_username(token: &str, username: &str) -> Result<Account, String> {
    let response = Client::new().put(format!("{}/account/profile", url()))
        .bearer_auth(token).json(&ProfileUpdate { username })
        .send().await.map_err(|e| e.to_string())?;
    match response.status() {
        StatusCode::CONFLICT => return Err("Cet identifiant est déjà utilisé.".into()),
        StatusCode::BAD_REQUEST => return Err("L'identifiant doit comporter entre 3 et 80 caractères.".into()),
        status if !status.is_success() => return Err(format!("Modification impossible (HTTP {status}).")),
        _ => {}
    }
    response.json::<Account>().await.map_err(|e| e.to_string())
}
pub async fn change_password(token: &str, current: &str, new_password: &str) -> Result<(), String> {
    let response = Client::new().put(format!("{}/account/password", url()))
        .bearer_auth(token).json(&PasswordUpdate { current_password: current, new_password })
        .send().await.map_err(|e| e.to_string())?;
    match response.status() {
        StatusCode::NO_CONTENT => Ok(()),
        StatusCode::FORBIDDEN => Err("Le mot de passe actuel est incorrect.".into()),
        StatusCode::BAD_REQUEST => Err("Le nouveau mot de passe doit contenir au moins 12 caractères et être différent de l'ancien.".into()),
        status => Err(format!("Changement impossible (HTTP {status}).")),
    }
}
