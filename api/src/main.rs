use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};
use tokio::net::TcpListener;

use crate::{
    middleware::auth,
    routes::{health::health, login::login, me::me, register::register},
    state::AppState,
};

mod entities;
mod middleware;
mod routes;
mod state;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let state = AppState::default().await;

    let protected = Router::new()
        .route("/me", get(me))
        .route_layer(axum_middleware::from_fn(auth));

    let app = Router::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/register", post(register))
        .with_state(state)
        .merge(protected);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
