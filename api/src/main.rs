use axum::{
    Router, middleware as axum_middleware,
    routing::{delete, get, post, put},
};
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    middleware::auth, routes::{
        authentication::{login::login, register::register}, doc::ApiDoc, entities::{
            activity::{create_activity, delete_activity, get_activities, get_activity, update_activity, update_activity_player},
            player::{create_player, delete_player, get_player, get_players, update_player},
        }, miscellaneous::{health::health, me::me},
    }, state::AppState,
};

mod entities;
mod middleware;
mod routes;
mod state;
mod services;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET is required");
    assert!(secret.len() >= 32 && secret != "SECRET", "JWT_SECRET must be at least 32 bytes and non-default");
    let bind = std::env::var("SEPPYL_BIND").unwrap_or_else(|_| "127.0.0.1:3101".to_string());

    let state = AppState::default().await;
    let protected = Router::new()
        .route("/me", get(me))
        .route_layer(axum_middleware::from_fn(auth));

    let app = Router::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/register", post(register))
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
        .with_state(state)
        .merge(protected)
        .merge(SwaggerUi::new("/doc").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let listener = TcpListener::bind(&bind).await.expect("Cannot bind API listener");
    axum::serve(listener, app).await.expect("API server failed");
}
