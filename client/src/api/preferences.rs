//! Account preferences, independent of identity provider (local JWT / future SSO).
use reqwest::Client;
use serde::{Deserialize, Serialize};

fn api_url() -> &'static str {
    option_env!("SEPPYL_API_URL").unwrap_or("http://127.0.0.1:3000")
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceView { #[default] Grid, List }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme { #[default] System, Light, Dark, Contrast }
impl Theme {
    pub fn as_str(self) -> &'static str {
        match self { Self::System => "system", Self::Light => "light", Self::Dark => "dark", Self::Contrast => "contrast" }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density { #[default] Comfortable, Compact }
impl Density {
    pub fn as_str(self) -> &'static str {
        match self { Self::Comfortable => "comfortable", Self::Compact => "compact" }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPreferences {
    pub attendance_view: AttendanceView,
    pub theme: Theme,
    pub density: Density,
}
#[derive(Serialize)]
struct PreferencePatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    attendance_view: Option<AttendanceView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    theme: Option<Theme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    density: Option<Density>,
}

pub async fn get(token: &str) -> Result<UserPreferences, String> {
    let response = Client::new().get(format!("{}/preferences", api_url()))
        .bearer_auth(token).send().await
        .map_err(|e| format!("Impossible de charger les préférences : {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Préférences indisponibles (HTTP {}).", response.status()));
    }
    response.json::<UserPreferences>().await
        .map_err(|e| format!("Préférences invalides : {e}"))
}

async fn patch(token: &str, change: PreferencePatch) -> Result<UserPreferences, String> {
    let response = Client::new().put(format!("{}/preferences", api_url()))
        .bearer_auth(token).json(&change).send().await
        .map_err(|e| format!("Impossible de sauvegarder la préférence : {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Préférence non enregistrée (HTTP {}).", response.status()));
    }
    response.json::<UserPreferences>().await.map_err(|e| e.to_string())
}

/// Compatibility API for the attendance view selection.
pub async fn load(token: &str) -> Result<AttendanceView, String> {
    get(token).await.map(|prefs| prefs.attendance_view)
}
pub async fn save(token: &str, attendance_view: AttendanceView) -> Result<(), String> {
    patch(token, PreferencePatch { attendance_view: Some(attendance_view), theme: None, density: None })
        .await.map(|_| ())
}
pub async fn save_appearance(token: &str, theme: Theme, density: Density)
    -> Result<UserPreferences, String> {
    patch(token, PreferencePatch { attendance_view: None, theme: Some(theme), density: Some(density) }).await
}
