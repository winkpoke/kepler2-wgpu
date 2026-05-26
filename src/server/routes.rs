use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use crate::server::{handlers, state::ServerState};

pub fn create_router(state: ServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::OPTIONS, axum::http::Method::PUT])
        .allow_headers(Any);

    Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/pkg", ServeDir::new("pkg"))
        .route("/api/health", get(handlers::health_check))
        .route("/api/volumes/upload", post(handlers::upload_volume))
        .route("/api/dicom/build/:id", get(handlers::build_ct_dicom_axum))
        .route("/ws", get(handlers::ws_handler))
        .with_state(state)
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024 * 1024))
}