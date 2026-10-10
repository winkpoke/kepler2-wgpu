//! Offline release-router contracts and independent native compute ownership.
#![cfg(not(target_arch = "wasm32"))]

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use kepler_wgpu::{
    gpu::{run_volume_upload, GpuContext, GpuState},
    server::{
        ai::AiService, ai_task::TaskManager, navcomputer::NavState, routes::create_router,
        state::ServerState,
    },
};
use parking_lot::Mutex;
use serde_json::{json, Value};
use tokio::sync::{broadcast, RwLock};
use tower::ServiceExt;

// Avoid ServerState::new(): its release startup deletes the configured series directory.
// No test below starts a listener, inference worker, or navigation/device operation.
fn offline_state() -> ServerState {
    let (ws_tx, _) = broadcast::channel(256);
    ServerState {
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
    }
}

#[tokio::test]
async fn release_routes_keep_health_dto_methods_and_missing_task_status() {
    let router = create_router(offline_state());
    let response = router
        .clone()
        .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap(),
        json!({
            "status": "ok", "loaded_volumes": 0, "uptime": "2026-10-05T00:00:00"
        })
    );
    let response = router
        .clone()
        .oneshot(Request::post("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    let response = router
        .oneshot(
            Request::get("/api/segment/result/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        to_bytes(response.into_body(), 4096).await.unwrap(),
        "Task missing not found"
    );
}

#[tokio::test]
#[ignore = "requires a physical native GPU; run explicitly"]
async fn compute_owner_is_independent_and_router_compute_readback_is_exact() {
    let state = offline_state();
    let compute = state.gpu.context().await.unwrap();
    let independent_viewer_owner = GpuContext::new().await.unwrap();
    assert!(!Arc::ptr_eq(&compute, &independent_viewer_owner));
    // Device IDs can collide across wgpu Instances. Prove ownership by destroying
    // the independent owner and then successfully using the server device.
    independent_viewer_owner.device().destroy();
    assert!(Arc::ptr_eq(&compute, &state.gpu.context().await.unwrap()));
    let voxels = [i16::MIN, -1024, -1, 0, 1, i16::MAX];
    let result = run_volume_upload(&compute, &voxels).await.unwrap();
    assert_eq!(result.voxel_count, voxels.len());
    assert_eq!(result.sample, voxels);
    let response = create_router(state)
        .oneshot(
            Request::post("/api/gpu/test")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"data":[-2.5,0.0,3.25]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap(),
        json!({"data":[-5.0,0.0,6.5]})
    );
}
