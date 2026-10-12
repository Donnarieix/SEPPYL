use axum::{
    Router, http::{Method, header}, middleware as axum_middleware, routing::{delete, get, post, put},
};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    middleware::auth, routes::{
        authentication::{login::login, register::register}, doc::ApiDoc, entities::{
            activity::{create_activity, delete_activity, get_activities, get_activity, update_activity, update_activity_player}, player::{create_player, delete_player, get_player, get_players, update_player},
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

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:8080".parse::<axum::http::HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

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

        .layer(cors)
        .with_state(state)
        .merge(protected)
        .merge(SwaggerUi::new("/doc").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
