//! Account-level UI preferences. These are intentionally independent of the
//! authentication provider so that Donnarieix SSO can replace local login.
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
pub enum AttendanceView {
    #[default]
    Grid,
    List,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPreferences {
    pub attendance_view: AttendanceView,
}

pub async fn get(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<UserPreferences>, StatusCode> {
    let saved = user_preference::Entity::find_by_id(claims.sub)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // A missing row is a valid state for existing accounts.
    let attendance_view = match saved.as_ref().map(|row| row.attendance_view.as_str()) {
        None | Some("grid") => AttendanceView::Grid,
        Some("list") => AttendanceView::List,
        Some(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    Ok(Json(UserPreferences { attendance_view }))
}

pub async fn update(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(preferences): Json<UserPreferences>,
) -> Result<Json<UserPreferences>, StatusCode> {
    let mode = match preferences.attendance_view {
        AttendanceView::Grid => "grid",
        AttendanceView::List => "list",
    };
    let saved = user_preference::Entity::find_by_id(claims.sub)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match saved {
        Some(row) => {
            let mut active: user_preference::ActiveModel = row.into();
            active.attendance_view = Set(mode.to_owned());
            active.update(&state.db).await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        None => {
            user_preference::ActiveModel {
                user_id: Set(claims.sub),
                attendance_view: Set(mode.to_owned()),
                ..Default::default()
            }
            .insert(&state.db).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    Ok(Json(preferences))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::user;
    use sea_orm::{ActiveModelTrait, Database};
    use uuid::Uuid;

    #[tokio::test]
    async fn preference_defaults_to_grid_then_persists_per_account() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.get_schema_registry("api::entities::*")
            .sync(&db).await.unwrap();
        let state = AppState { db };
        let alice = Uuid::new_v4();
        let bob = Uuid::new_v4();
        for (id, name) in [(alice, "alice"), (bob, "bob")] {
            user::ActiveModel {
                id: Set(id),
                username: Set(name.to_owned()),
                password: Set("test-only".to_owned()),
                ..Default::default()
            }
            .insert(&state.db).await.unwrap();
        }
        let alice_claims = Claims { sub: alice, exp: u64::MAX };
        let bob_claims = Claims { sub: bob, exp: u64::MAX };
        let initial = get(Extension(alice_claims.clone()), State(state.clone()))
            .await.unwrap().0;
        assert_eq!(initial.attendance_view, AttendanceView::Grid);
        update(
            Extension(alice_claims.clone()),
            State(state.clone()),
            Json(UserPreferences { attendance_view: AttendanceView::List }),
        ).await.unwrap();
        let reloaded = get(Extension(alice_claims.clone()), State(state.clone()))
            .await.unwrap().0;
        assert_eq!(reloaded.attendance_view, AttendanceView::List);
        let untouched = get(Extension(bob_claims), State(state.clone()))
            .await.unwrap().0;
        assert_eq!(untouched.attendance_view, AttendanceView::Grid);
        update(
            Extension(alice_claims.clone()),
            State(state.clone()),
            Json(UserPreferences { attendance_view: AttendanceView::Grid }),
        ).await.unwrap();
        let reset = get(Extension(alice_claims), State(state))
            .await.unwrap().0;
        assert_eq!(reset.attendance_view, AttendanceView::Grid);
    }

    #[test]
    fn only_supported_modes_deserialize() {
        assert!(serde_json::from_str::<UserPreferences>(r#"{"attendance_view":"grid"}"#).is_ok());
        assert!(serde_json::from_str::<UserPreferences>(r#"{"attendance_view":"list"}"#).is_ok());
        assert!(serde_json::from_str::<UserPreferences>(r#"{"attendance_view":"kanban"}"#).is_err());
    }
}
