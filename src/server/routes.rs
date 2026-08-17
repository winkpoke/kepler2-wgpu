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
        .nest_service("/pkg", ServeDir::new("pkg"))
        .route("/api/health", get(handlers::health_check))
        .route("/api/volumes/upload", post(handlers::upload_volume))
        .route("/api/upload_needle_params", post(handlers::upload_needle_params))
        .route("/api/needle_point_offset", post(handlers::needle_point_offset))
        .route("/api/upload_obj", post(handlers::upload_obj))
        .route("/api/dicom/build/:id", get(handlers::build_ct_dicom_axum))
        .route("/api/segment", post(handlers::start_segmentation))
        .route("/api/segment/progress/:id", get(handlers::segment_progress))
        .route("/api/segment/result/:id", get(handlers::segment_result_meta))
        .route("/api/segment/result/:id/raw", get(handlers::segment_result_raw))
        .route("/ws", get(handlers::ws_handler))
        .fallback_service(ServeDir::new("static"))
        .with_state(state)
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024 * 1024))
}