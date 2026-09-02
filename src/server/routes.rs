use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use crate::server::{handlers, navcomputer as nav, state::ServerState};

pub fn create_router(state: ServerState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::OPTIONS, axum::http::Method::PUT, axum::http::Method::DELETE])
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
        .route("/api/nav/config", get(nav::nav_get_config).post(nav::nav_set_config))
        .route("/api/nav/health", get(nav::nav_health))
        .route("/api/nav/status", get(nav::nav_status))
        .route("/api/nav/prepared-ct-lookups", post(nav::nav_prepared_ct_lookup))
        .route("/api/nav/prepared-cts", post(nav::nav_prepared_ct_upload))
        .route("/api/nav/setup-registrations", post(nav::nav_setup_registration))
        .route("/api/nav/setup-registrations/:id", post(nav::nav_target_observation))
        .route("/api/nav/setup-registrations/:id/drr-previews/:frame", get(nav::nav_drr_preview_save))
        .route("/api/nav/navigation-sessions/:id", get(nav::nav_navigation_session))
        .route("/callbacks/:session/events/:sequence", put(nav::nav_callback_event))
        .route("/callbacks/:session/live-state", put(nav::nav_callback_live_state))
        .route("/api/nav/callbacks/:session", get(nav::nav_callback_inspect).delete(nav::nav_callback_clear))
        .route("/ws", get(handlers::ws_handler))
        .fallback_service(ServeDir::new("static"))
        .with_state(state)
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024 * 1024))
}