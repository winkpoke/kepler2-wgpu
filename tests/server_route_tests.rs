//! Offline route-contract tests for the axum server (P0 gap: server routes had
//! only 2 tests). Everything runs through `tower::ServiceExt::oneshot` against
//! `create_router(offline_state())` — no listener, no GPU, no inference worker,
//! no filesystem side effects (mirrors the pattern in
//! native_backend_compatibility_tests.rs).
#![cfg(not(target_arch = "wasm32"))]

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use kepler_wgpu::{
    device::hv::HvDevice,
    gpu::GpuState,
    server::{
        ai::AiService, ai_task::TaskManager, navcomputer::NavState, routes::create_router,
        state::ServerState, WsMessage,
    },
};
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::sync::{broadcast, RwLock};
use tower::ServiceExt;

/// Offline ServerState plus the receiving end of its WS broadcast channel, so
/// tests can assert on messages the handlers broadcast.
struct Offline {
    state: ServerState,
    ws_rx: broadcast::Receiver<WsMessage>,
}

fn offline() -> Offline {
    let (ws_tx, ws_rx) = broadcast::channel(256);
    let state = ServerState {
        ai: Arc::new(AiService::new()),
        tasks: TaskManager::new(),
        volumes: Arc::new(RwLock::new(HashMap::new())),
        start_time: "2026-10-05T00:00:00".into(),
        ws_tx,
        series_dir: PathBuf::from("/private/tmp/kepler-offline-unused-series"),
        output_root: PathBuf::from("/private/tmp/kepler-offline-unused-output"),
        offset: Arc::new(Mutex::new([0.0; 3])),
        ptm: Arc::new(Mutex::new(glam::Mat4::IDENTITY)),
        ct_volume: Arc::new(Mutex::new(None)),
        mask_circle: Arc::new(Mutex::new(None)),
        mask_data: Arc::new(Mutex::new(None)),
        nav: Arc::new(NavState::new()),
        onnx_models: Arc::new(Mutex::new(None)),
        gpu: Arc::new(GpuState::new()),
        hv: Arc::new(Mutex::new(HvDevice::new())),
    };
    Offline { state, ws_rx }
}

async fn req_json(router: &axum::Router, method: &str, uri: &str, body: Value) -> axum::response::Response {
    let builder = match method {
        "GET" => Request::get(uri),
        "POST" => Request::post(uri),
        other => panic!("unsupported method {other}"),
    };
    let request = if body.is_null() {
        builder.body(Body::empty()).unwrap()
    } else {
        builder
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    router.clone().oneshot(request).await.unwrap()
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 65536).await.unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = to_bytes(resp.into_body(), 65536).await.unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn assert_close(a: f32, b: f32, label: &str) {
    assert!((a - b).abs() < 1e-3, "{label}: expected {b}, got {a}");
}

// ---------------------------------------------------------------------------
// /api/nav/config
// ---------------------------------------------------------------------------

#[tokio::test]
async fn nav_config_get_returns_hardcoded_default_token() {
    let router = create_router(offline().state);
    let resp = req_json(&router, "GET", "/api/nav/config", Value::Null).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    // NavState::new() seeds the token from the (misleadingly named)
    // NAV_TOKEN_ENV_VAR constant — a hardcoded credential, not an env lookup —
    // so a fresh offline state reports token_set = true.
    assert_eq!(body["token_set"], json!(true));
    assert!(body["token_masked"].as_str().is_some());
    assert!(!body["base_url"].as_str().unwrap().is_empty());
    assert!(!body["drr_preview_dir"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn nav_config_post_rejects_non_http_base_url() {
    let router = create_router(offline().state);
    let resp = req_json(
        &router,
        "POST",
        "/api/nav/config",
        json!({"base_url": "ftp://nav.example.com"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let msg = body_string(resp).await;
    assert!(msg.contains("http"), "error should mention http(s): {msg}");
}

#[tokio::test]
async fn nav_config_post_normalizes_base_url_and_masks_token() {
    let router = create_router(offline().state);
    let resp = req_json(
        &router,
        "POST",
        "/api/nav/config",
        json!({"base_url": "http://nav.example.com/", "token": "secret-token-123"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["base_url"], json!("http://nav.example.com")); // trailing slash trimmed
    assert_eq!(body["token_set"], json!(true));
    let masked = body["token_masked"].as_str().unwrap();
    assert!(masked.starts_with("secret"), "masked token: {masked}");
    assert_ne!(masked, "secret-token-123", "token must not be echoed in full");
}

// ---------------------------------------------------------------------------
// /api/nav/health + /api/nav/status — unreachable upstream relays as 502
// ---------------------------------------------------------------------------

#[tokio::test]
async fn nav_health_relays_unreachable_upstream_as_bad_gateway() {
    let router = create_router(offline().state);
    let resp = req_json(&router, "GET", "/api/nav/health", Value::Null).await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let msg = body_string(resp).await;
    assert!(
        msg.contains("Navigation Computer") || msg.is_empty(),
        "relay error body: {msg}"
    );
}

#[tokio::test]
async fn nav_status_relays_unreachable_upstream_as_bad_gateway() {
    let router = create_router(offline().state);
    let resp = req_json(&router, "GET", "/api/nav/status", Value::Null).await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
}

// ---------------------------------------------------------------------------
// /api/needle_point_offset + /api/upload_needle_params (behavior level)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn needle_point_offset_roundtrips_offset_and_ptm() {
    let router = create_router(offline().state);
    let ptm: [f32; 16] =
        glam::Mat4::from_translation(glam::Vec3::new(5.0, 6.0, 7.0)).to_cols_array();
    let resp = req_json(
        &router,
        "POST",
        "/api/needle_point_offset",
        json!({"x": 1.5, "y": -2.0, "z": 3.0, "ptm": ptm}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body[0], json!([1.5, -2.0, 3.0])); // echoed offset
    let echoed_ptm = body[1].as_array().unwrap();
    assert_eq!(echoed_ptm.len(), 16);
    assert!((echoed_ptm[12].as_f64().unwrap() - 5.0).abs() < 1e-5); // translation column
}

#[tokio::test]
async fn upload_needle_params_without_body_is_bad_request() {
    let router = create_router(offline().state);
    let resp = router
        .oneshot(
            // ConnectInfo must be injected or the extractor fails before the
            // handler and turns the contract into a 500.
            Request::post("/api/upload_needle_params")
                .extension(ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 41001))))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn upload_needle_params_broadcasts_transformed_needle_set() {
    // Behavior-level contract: entry/tip must survive the documented pipeline
    //   model --axis remap (x,y,z)->(-x,z,y)--> PTM --> offset.
    // With zero orientation angles the direction is (0,0,-1) and len 400 mm,
    // so every value below is exact in f32.
    let Offline { state, mut ws_rx } = offline();
    let router = create_router(state);

    let resp = router
        .oneshot(
            Request::post("/api/upload_needle_params")
                .header("content-type", "application/json")
                .extension(ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 41000))))
                .body(Body::from(
                    r#"{"position":{"x":10.0,"y":20.0,"z":30.0},
                        "orientation":{"a":0.0,"b":0.0,"c":0.0},
                        "frame_sequence":7}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let msg = ws_rx.try_recv().expect("NeedleSet must be broadcast");
    let WsMessage::NeedleSet {
        id,
        x,
        y,
        z,
        lx,
        ly,
        lz,
        r,
        g,
        b,
        dir,
        len_mm,
    } = msg
    else {
        panic!("expected NeedleSet, got a different WsMessage variant");
    };

    assert_eq!(id, 7);
    // Model space: tip (10,20,30), end = tip + 400mm*(0,0,-1) = (10,20,-370).
    // Axis remap (x,y,z)->(-x,z,y): tip_t = (-10,30,20), end_t = (-10,-370,20).
    // Offset (0,0,0) + identity PTM keep them unchanged.
    assert_close(x, -10.0, "entry.x");
    assert_close(y, -370.0, "entry.y");
    assert_close(z, 20.0, "entry.z");
    assert_close(lx, -10.0, "tip.x");
    assert_close(ly, 30.0, "tip.y");
    assert_close(lz, 20.0, "tip.z");
    assert_close(dir.0, 0.0, "dir.x");
    assert_close(dir.1, 0.0, "dir.y");
    assert_close(dir.2, -1.0, "dir.z");
    assert_close(len_mm, 400.0, "len_mm");
    assert_close(r, 0.2, "r");
    assert_close(g, 0.9, "g");
    assert_close(b, 0.2, "b");
}

// ---------------------------------------------------------------------------
// /api/segment task lifecycle (offline: task registered, never completes)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn segment_task_registers_then_progress_200_and_result_409() {
    let router = create_router(offline().state);
    let resp = req_json(
        &router,
        "POST",
        "/api/segment",
        json!({"series_id": "series-offline", "model": "total_seg"}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], json!("accepted"));
    let task_id = body["task_id"].as_str().unwrap().to_string();
    assert!(!task_id.is_empty());

    // Progress is registered immediately: 200 with a valid status string.
    let resp = req_json(
        &router,
        "GET",
        &format!("/api/segment/progress/{task_id}"),
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let progress = body_json(resp).await;
    assert_eq!(progress["task_id"], json!(task_id));
    let status = progress["status"].as_str().unwrap();
    assert!(
        ["pending", "running", "completed", "failed", "cancelled"].contains(&status),
        "unexpected status {status}"
    );

    // The task can never reach Completed offline (no model), so meta is 409.
    let resp = req_json(
        &router,
        "GET",
        &format!("/api/segment/result/{task_id}"),
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Raw mask download: task exists but no mask offline -> 500 per current contract.
    let resp = req_json(
        &router,
        "GET",
        &format!("/api/segment/result/{task_id}/raw"),
        Value::Null,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn segment_progress_unknown_task_is_500_with_reason() {
    let router = create_router(offline().state);
    let resp = req_json(&router, "GET", "/api/segment/progress/no-such-task", Value::Null).await;
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let msg = body_string(resp).await;
    assert!(msg.contains("not found"), "body: {msg}");
}

#[tokio::test]
async fn segment_result_meta_unknown_task_is_404() {
    let router = create_router(offline().state);
    let resp = req_json(&router, "GET", "/api/segment/result/no-such-task", Value::Null).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn segment_start_with_invalid_json_is_422() {
    let router = create_router(offline().state);
    let resp = router
        .oneshot(
            Request::post("/api/segment")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"series_id": 42}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ---------------------------------------------------------------------------
// Method guards, CORS preflight, fallback, hv status, WS handshake
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_on_post_only_endpoint_is_method_not_allowed() {
    let router = create_router(offline().state);
    let resp = router
        .oneshot(
            Request::get("/api/volumes/upload").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn cors_preflight_for_health_is_accepted() {
    let router = create_router(offline().state);
    let resp = router
        .oneshot(
            Request::options("/api/health")
                .header("origin", "http://localhost:8080")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_path_falls_back_to_static_dir_and_404s_offline() {
    let router = create_router(offline().state);
    let resp = router
        .oneshot(
            Request::get("/definitely-not-a-real-path")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn hv_status_reports_disconnected_defaults() {
    let router = create_router(offline().state);
    let resp = req_json(&router, "GET", "/hv/status", Value::Null).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["connected"], json!(false));
    assert!(body["port"].is_string());
    assert!(body["baud"].is_u64() || body["baud"].is_i64());
}

#[tokio::test]
async fn hv_send_while_disconnected_reports_ok_false() {
    let router = create_router(offline().state);
    let resp = req_json(&router, "POST", "/hv/send", json!({"cmd": "0103"})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], json!(false));
    assert!(body["error"].is_string());
}

#[tokio::test]
async fn ws_upgrade_without_websocket_headers_is_rejected() {
    let router = create_router(offline().state);
    let resp = router
        .oneshot(Request::get("/ws").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
