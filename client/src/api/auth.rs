use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Api {
    pub client: Client,
    pub base_url: String,
    pub auth: Auth,
}

impl Api {
    pub fn new(base_url: String) -> Self {
        Api {
            client: Client::new(),
            base_url,
            auth: Auth {
                username: String::new(),
                password: String::new(),
                token: None,
            },
        }
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<LoginResponse, String> {
        self.auth.username = username.to_string();
        self.auth.password = password.to_string();
        
        let response = self.client
            .post(&format!("{}/login", self.base_url))
            .json(&LoginRequest {
                username: self.auth.username.to_string(),
                password: self.auth.password.to_string(),
            })
            .send()
            .await
            .map_err(|err| err.to_string())?;
    
        match response.status() {
            StatusCode::OK => {
                let login_response = response
                    .json::<LoginResponse>()
                    .await
                    .map_err(|err| err.to_string())?;

                self.auth.token = Some(login_response.token.clone());
                Ok(login_response)
            }
    
            StatusCode::UNAUTHORIZED => {
                Err("Identifiants incorrects".into())
            }
    
            _ => {
                Err(format!("Erreur serveur: {}", response.status()))
            }
        }
    }

    pub async fn register(&mut self, username: &str, password: &str) -> Result<(), String> {
        self.auth.username = username.to_string();
        self.auth.password = password.to_string();

        let response = self.client
            .post(&format!("{}/register", self.base_url))
            .json(&RegisterRequest {
                username: self.auth.username.to_string(),
                password: self.auth.password.to_string(),
            })
            .send()
            .await
            .map_err(|err| err.to_string())?;

        match response.status() {
            StatusCode::OK => Ok(()),
            StatusCode::CONFLICT => Err("Nom d'utilisateur déjà pris".into()),
            _ => Err(format!("Erreur serveur: {}", response.status())),
        }
    }

    pub async fn me(&self) -> Result<(), String> {
        let response = self.client
            .get(&format!("{}/me", self.base_url))
            .bearer_auth(self.auth.token.clone().unwrap_or_default())
            .send()
            .await
            .map_err(|err| err.to_string())?;

        match response.status() {
            StatusCode::OK => Ok(()),
            StatusCode::UNAUTHORIZED => Err("Non autorisé".into()),
            _ => Err(format!("Erreur serveur: {}", response.status())),
        }
    }

    pub async fn auth(&mut self) -> Result<(), String> {
        match self.me().await {
            Ok(_) => Ok(()),
            Err(_) => {
                let username = self.auth.username.clone();
                let password = self.auth.password.clone();

                match self.login(&username, &password).await {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err)
                }
            }
        }
    }
}

#[derive(Clone)]
struct Auth {
    pub username: String,
    pub password: String,
    pub token: Option<String>,
}

#[derive(Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Serialize)]
struct RegisterRequest {
    username: String,
    password: String,
}
