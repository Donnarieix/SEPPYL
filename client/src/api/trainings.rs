use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

fn base_url() -> &'static str {
    super::base_url()
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Training {
    pub id: String,
    pub name: String,
    pub date: String,
    pub notes: String,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Player {
    pub id: String,
    pub firstname: String,
    pub lastname: String,
    pub number: u32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Attendance {
    pub player_id: String,
    pub present: bool,
    pub score: Option<i32>,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Activity {
    pub id: String,
    pub training_id: String,
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub sort_order: i32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Material {
    pub id: String,
    pub activity_id: String,
    pub name: String,
    pub quantity: i32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Score {
    pub activity_id: String,
    pub player_id: String,
    pub score: i32,
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Detail {
    pub training: Training,
    pub players: Vec<Player>,
    pub attendance: Vec<Attendance>,
    pub activities: Vec<Activity>,
    pub materials: Vec<Material>,
    pub scores: Vec<Score>,
}

#[derive(Serialize)]
struct TrainingRequest<'a> {
    name: &'a str,
    date: String,
}
#[derive(Serialize)]
struct UpdateTrainingRequest<'a> {
    name: &'a str,
    date: String,
    notes: &'a str,
}
#[derive(Serialize)]
struct ActivityRequest<'a> {
    name: &'a str,
    description: &'a str,
    image_url: Option<&'a str>,
}
#[derive(Serialize)]
struct MaterialRequest<'a> {
    name: &'a str,
    quantity: i32,
}
#[derive(Serialize)]
struct AttendanceRequest<'a> {
    player_ids: &'a [String],
}
#[derive(Serialize)]
struct ScoreRequest<'a> {
    player_id: &'a str,
    activity_id: Option<&'a str>,
    score: Option<i32>,
}

fn date_time(date: &str) -> String {
    format!("{date}T00:00:00")
}
fn client() -> Client {
    Client::new()
}
async fn success(response: reqwest::Response) -> Result<(), String> {
    let status = response.status();
    match status {
        StatusCode::UNAUTHORIZED => Err("Session expirée. Reconnectez-vous.".into()),
        StatusCode::FORBIDDEN => Err("Accès refusé.".into()),
        StatusCode::NOT_FOUND => Err("Cet entraînement ou cette activité n'existe pas.".into()),
        StatusCode::BAD_REQUEST => Err("Les informations saisies sont invalides.".into()),
        _ if status.is_success() => Ok(()),
        _ => Err(format!("Erreur du serveur (HTTP {status}).")),
    }
}

pub async fn list(token: &str) -> Result<Vec<Training>, String> {
    let response = client().get(format!("{}/trainings", base_url()))
        .bearer_auth(token).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        success(response).await?;
        return Err("Réponse inattendue.".into());
    }
    response.json::<Vec<Training>>().await.map_err(|e| e.to_string())
}
pub async fn get(token: &str, id: &str) -> Result<Detail, String> {
    let response = client().get(format!("{}/trainings/{id}", base_url()))
        .bearer_auth(token).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        success(response).await?;
        return Err("Réponse inattendue.".into());
    }
    response.json::<Detail>().await.map_err(|e| e.to_string())
}
pub async fn create(token: &str, name: &str, date: &str) -> Result<String, String> {
    let response = client().post(format!("{}/trainings", base_url()))
        .bearer_auth(token)
        .json(&TrainingRequest { name, date: date_time(date) })
        .send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        success(response).await?;
        return Err("Création impossible.".into());
    }
    response.text().await.map_err(|e| e.to_string())
}
pub async fn update(
    token: &str, id: &str, name: &str, date: &str, notes: &str,
) -> Result<(), String> {
    let response = client().put(format!("{}/trainings/{id}", base_url()))
        .bearer_auth(token)
        .json(&UpdateTrainingRequest { name, date: date_time(date), notes })
        .send().await.map_err(|e| e.to_string())?;
    success(response).await
}
pub async fn add_activity(
    token: &str, id: &str, name: &str, description: &str, image_url: Option<&str>,
) -> Result<(), String> {
    let response = client().post(format!("{}/trainings/{id}/activities", base_url()))
        .bearer_auth(token)
        .json(&ActivityRequest { name, description, image_url })
        .send().await.map_err(|e| e.to_string())?;
    success(response).await
}
pub async fn remove_activity(token: &str, id: &str, activity_id: &str) -> Result<(), String> {
    let response = client().delete(format!("{}/trainings/{id}/activities/{activity_id}", base_url()))
        .bearer_auth(token).send().await.map_err(|e| e.to_string())?;
    success(response).await
}
pub async fn add_material(
    token: &str, id: &str, activity_id: &str, name: &str, quantity: i32,
) -> Result<(), String> {
    let response = client().post(format!("{}/trainings/{id}/activities/{activity_id}/materials", base_url()))
        .bearer_auth(token).json(&MaterialRequest { name, quantity })
        .send().await.map_err(|e| e.to_string())?;
    success(response).await
}
pub async fn remove_material(
    token: &str, id: &str, activity_id: &str, material_id: &str,
) -> Result<(), String> {
    let response = client().delete(format!("{}/trainings/{id}/activities/{activity_id}/materials/{material_id}", base_url()))
        .bearer_auth(token).send().await.map_err(|e| e.to_string())?;
    success(response).await
}
pub async fn save_attendance(token: &str, id: &str, player_ids: &[String]) -> Result<(), String> {
    let response = client().put(format!("{}/trainings/{id}/attendance", base_url()))
        .bearer_auth(token).json(&AttendanceRequest { player_ids })
        .send().await.map_err(|e| e.to_string())?;
    success(response).await
}
pub async fn save_score(
    token: &str, id: &str, player_id: &str, activity_id: Option<&str>, score: Option<i32>,
) -> Result<(), String> {
    let response = client().put(format!("{}/trainings/{id}/scores", base_url()))
        .bearer_auth(token)
        .json(&ScoreRequest { player_id, activity_id, score })
        .send().await.map_err(|e| e.to_string())?;
    success(response).await
}
