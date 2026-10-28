//! Preferences persisted per account by the API, independent of authentication
//! provider (local JWT now, Donnarieix SSO later).
use reqwest::Client;
use serde::{Deserialize, Serialize};

fn api_url() -> &'static str {
    option_env!("SEPPYL_API_URL").unwrap_or("http://127.0.0.1:3000")
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceView {
    #[default]
    Grid,
    List,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UserPreferences {
    attendance_view: AttendanceView,
}

pub async fn load(token: &str) -> Result<AttendanceView, String> {
    let response = Client::new()
        .get(format!("{}/preferences", api_url()))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("Impossible de charger la préférence : {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Chargement de la préférence impossible (HTTP {}).", response.status()));
    }
    response.json::<UserPreferences>().await
        .map(|preferences| preferences.attendance_view)
        .map_err(|e| format!("Préférence invalide : {e}"))
}

pub async fn save(token: &str, attendance_view: AttendanceView) -> Result<(), String> {
    let response = Client::new()
        .put(format!("{}/preferences", api_url()))
        .bearer_auth(token)
        .json(&UserPreferences { attendance_view })
        .send()
        .await
        .map_err(|e| format!("Impossible de sauvegarder la préférence : {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Sauvegarde de la préférence impossible (HTTP {}).", response.status()));
    }
    Ok(())
}
