//! Account-level UI preferences, kept independent from the auth provider.
//! Updating a single field preserves all other preferences (including
//! attendance view changes from the training screen).
use axum::{Extension, Json, extract::State, http::StatusCode};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde::{Deserialize, Serialize};

use crate::{
    entities::user_preference,
    routes::authentication::login::Claims,
    state::AppState,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttendanceView { #[default] Grid, List }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme { #[default] System, Light, Dark }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density { #[default] Comfortable, Compact }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPreferences {
    pub attendance_view: AttendanceView,
    pub theme: Theme,
    pub density: Density,
}

// PATCH-like semantics over PUT: old versions of the client still send only
// attendance_view; the theme and density settings must never be reset by it.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePreferences {
    pub attendance_view: Option<AttendanceView>,
    pub theme: Option<Theme>,
    pub density: Option<Density>,
}

fn to_view(value: &str) -> Result<AttendanceView, StatusCode> {
    match value { "grid" => Ok(AttendanceView::Grid), "list" => Ok(AttendanceView::List),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR) }
}
fn to_theme(value: Option<&str>) -> Result<Theme, StatusCode> {
    match value.unwrap_or("system") { "system" => Ok(Theme::System),
        "light" => Ok(Theme::Light), "dark" => Ok(Theme::Dark),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR) }
}
fn to_density(value: Option<&str>) -> Result<Density, StatusCode> {
    match value.unwrap_or("comfortable") { "comfortable" => Ok(Density::Comfortable),
        "compact" => Ok(Density::Compact),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR) }
}
fn from_saved(saved: Option<&user_preference::Model>) -> Result<UserPreferences, StatusCode> {
    Ok(UserPreferences {
        attendance_view: match saved {
            Some(row) => to_view(&row.attendance_view)?,
            None => AttendanceView::Grid,
        },
        theme: to_theme(saved.and_then(|row| row.theme.as_deref()))?,
        density: to_density(saved.and_then(|row| row.density.as_deref()))?,
    })
}
fn view_str(value: AttendanceView) -> String {
    match value { AttendanceView::Grid => "grid", AttendanceView::List => "list" }.into()
}
fn theme_str(value: Theme) -> String {
    match value { Theme::System => "system", Theme::Light => "light", Theme::Dark => "dark" }.into()
}
fn density_str(value: Density) -> String {
    match value { Density::Comfortable => "comfortable", Density::Compact => "compact" }.into()
}

pub async fn get(
    Extension(claims): Extension<Claims>, State(state): State<AppState>,
) -> Result<Json<UserPreferences>, StatusCode> {
    let saved = user_preference::Entity::find_by_id(claims.sub)
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(from_saved(saved.as_ref())?))
}

pub async fn update(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(patch): Json<UpdatePreferences>,
) -> Result<Json<UserPreferences>, StatusCode> {
    let saved = user_preference::Entity::find_by_id(claims.sub)
        .one(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let old = from_saved(saved.as_ref())?;
    let updated = UserPreferences {
        attendance_view: patch.attendance_view.unwrap_or(old.attendance_view),
        theme: patch.theme.unwrap_or(old.theme),
        density: patch.density.unwrap_or(old.density),
    };
    match saved {
        Some(row) => {
            let mut active: user_preference::ActiveModel = row.into();
            active.attendance_view = Set(view_str(updated.attendance_view));
            active.theme = Set(Some(theme_str(updated.theme)));
            active.density = Set(Some(density_str(updated.density)));
            active.update(&state.db).await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        None => {
            user_preference::ActiveModel {
                user_id: Set(claims.sub),
                attendance_view: Set(view_str(updated.attendance_view)),
                theme: Set(Some(theme_str(updated.theme))),
                density: Set(Some(density_str(updated.density))),
                ..Default::default()
            }
            .insert(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    Ok(Json(updated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::user;
    use sea_orm::{ActiveModelTrait, Database};
    use uuid::Uuid;

    #[tokio::test]
    async fn preferences_are_account_scoped_and_partial_updates_preserve_other_values() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*").sync(&db).await.unwrap();
        let state = AppState { db };
        let alice = Uuid::new_v4();
        let bob = Uuid::new_v4();
        for (id, name) in [(alice, "alice"), (bob, "bob")] {
            user::ActiveModel {
                id: Set(id), username: Set(name.to_owned()),
                password: Set("test-only".to_owned()), ..Default::default()
            }.insert(&state.db).await.unwrap();
        }
        let alice_claims = Claims { sub: alice, exp: u64::MAX };
        let bob_claims = Claims { sub: bob, exp: u64::MAX };
        assert_eq!(get(Extension(alice_claims.clone()), State(state.clone())).await.unwrap().0,
            UserPreferences::default());
        update(Extension(alice_claims.clone()), State(state.clone()), Json(UpdatePreferences {
            attendance_view: Some(AttendanceView::List), ..Default::default()
        })).await.unwrap();
        update(Extension(alice_claims.clone()), State(state.clone()), Json(UpdatePreferences {
            theme: Some(Theme::Dark), density: Some(Density::Compact), ..Default::default()
        })).await.unwrap();
        let alice_settings = get(Extension(alice_claims.clone()), State(state.clone())).await.unwrap().0;
        assert_eq!(alice_settings.attendance_view, AttendanceView::List);
        assert_eq!(alice_settings.theme, Theme::Dark);
        assert_eq!(alice_settings.density, Density::Compact);
        assert_eq!(get(Extension(bob_claims), State(state.clone())).await.unwrap().0,
            UserPreferences::default());
        update(Extension(alice_claims.clone()), State(state.clone()), Json(UpdatePreferences {
            attendance_view: Some(AttendanceView::Grid), ..Default::default()
        })).await.unwrap();
        let final_settings = get(Extension(alice_claims), State(state)).await.unwrap().0;
        assert_eq!(final_settings.attendance_view, AttendanceView::Grid);
        assert_eq!(final_settings.theme, Theme::Dark);
    }

    #[test]
    fn invalid_enum_values_rejected() {
        assert!(serde_json::from_str::<UpdatePreferences>(r#"{"theme":"midnight"}"#).is_err());
        assert!(serde_json::from_str::<UpdatePreferences>(r#"{"density":"huge"}"#).is_err());
        assert!(serde_json::from_str::<UpdatePreferences>(r#"{"attendance_view":"kanban"}"#).is_err());
    }
}
