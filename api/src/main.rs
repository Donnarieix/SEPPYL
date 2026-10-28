use axum::{
    Router,
    http::{HeaderValue, Method, header},
    middleware as axum_middleware,
    routing::{delete, get, post, put},
};
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    middleware::auth,
    routes::{
        authentication::{login::login, register::register},
        doc::ApiDoc,
        entities::{
            activity::{
                create_activity, delete_activity, get_activities, get_activity, update_activity,
                update_activity_player,
            },
            matrix::{create_matrix_activity, get_matrix, update_matrix_result},
            training,
            player::{create_player, delete_player, get_player, get_players, update_player},
        },
        miscellaneous::{health::health, me::me},
    },
    state::AppState,
};

mod entities;
mod middleware;
mod routes;
mod services;
mod state;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    // dx serve may choose a different local port on every run.
    // Permit HTTP loopback origins for development without allowing arbitrary websites.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| is_local_dev_origin(origin)))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let state = AppState::default().await;

    // All routes extracting Claims must pass through the auth middleware.
    let protected = Router::new()
        .route("/me", get(me))
        .route("/player/list", get(get_players))
        .route("/player/get/{id}", get(get_player))
        .route("/player/create", post(create_player))
        .route("/player/update", put(update_player))
        .route("/player/delete/{id}", delete(delete_player))
        .route("/activity/list", get(get_activities))
        .route("/activity/get/{id}", get(get_activity))
        .route("/activity/create", post(create_activity))
        .route("/activity/update", put(update_activity))
        .route("/activity_player/update", put(update_activity_player))
        .route("/activity/delete/{id}", delete(delete_activity))
        .route("/matrix", get(get_matrix))
        .route("/matrix/activity", post(create_matrix_activity))
        .route("/matrix/result", put(update_matrix_result))
        .route("/trainings", get(training::list).post(training::create))
        .route("/trainings/{id}", get(training::get).put(training::update))
        .route("/trainings/{id}/activities", post(training::add_activity))
        .route("/trainings/{id}/activities/{activity_id}", delete(training::remove_activity))
        .route("/trainings/{id}/activities/{activity_id}/materials", post(training::add_material))
        .route("/trainings/{id}/activities/{activity_id}/materials/{material_id}", delete(training::remove_material))
        .route("/trainings/{id}/attendance", put(training::set_attendance))
        .route("/trainings/{id}/scores", put(training::set_score))
        .route_layer(axum_middleware::from_fn(auth));

    let app = Router::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/register", post(register))
        .merge(protected)
        .merge(SwaggerUi::new("/doc").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Apply CORS after merging, so it covers protected routes too.
        .layer(cors)
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


// A browser's Origin header must match the exact local hostname and (optional)
// numeric port. Prefix matching alone would also allow localhost.evil.example.
fn is_local_dev_origin(origin: &HeaderValue) -> bool {
    let Ok(origin) = origin.to_str() else {
        return false;
    };

    for hostname in ["http://localhost", "http://127.0.0.1"] {
        if origin == hostname {
            return true;
        }
        if let Some(port) = origin.strip_prefix(hostname).and_then(|rest| rest.strip_prefix(':')) {
            if port.parse::<u16>().is_ok() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod cors_tests {
    use super::*;

    #[test]
    fn accepts_loopback_dev_origins_on_dynamic_ports() {
        for origin in [
            "http://localhost",
            "http://localhost:8080",
            "http://127.0.0.1:50987",
            "http://127.0.0.1:8080",
        ] {
            assert!(is_local_dev_origin(&origin.parse().unwrap()), "{origin}");
        }
    }

    #[test]
    fn rejects_external_or_lookalike_origins() {
        for origin in [
            "https://attacker.example",
            "http://localhost.evil.example:8080",
            "http://127.0.0.1.evil.example",
            "http://localhost:8080.evil.example",
            "http://192.168.1.100:8080",
            "https://localhost:8080",
            "null",
        ] {
            assert!(!is_local_dev_origin(&origin.parse().unwrap()), "{origin}");
        }
    }
}
