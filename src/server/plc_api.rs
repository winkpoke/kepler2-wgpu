//! PLC REST/WS surface (native only).
//!
//! 与 `handlers.rs` 的 `hv_*` 同风格：handlers 拿 `State<ServerState>`，
//! 错误统一返回 `{"ok": false, "error": ...}`；状态推送复用现有
//! `ws_tx` 广播通道（`WsMessage::PlcStatus`），前端状态灯/圆盘只消费事件。
//!
//! 路由总览（挂到现有 router：`routes.rs` 里 `.merge(plc_api::router())`）：
//! - POST /plc/connect            连接 + 申请控制权 + 心跳 + 状态泵（"设备状态-点击连接"）
//! - POST /plc/disconnect         断开并释放控制权
//! - GET  /plc/status             一次性状态快照（联锁/模式/五轴/故障）
//! - POST /plc/axis/:n/move       绝对定位（暂存→COMMIT→COMMAND，返回 command_id）
//! - POST /plc/axis/:n/stop       单轴停止
//! - POST /plc/axis/:n/home       回参考位（"当前视角复位"）
//! - POST /plc/axis/:n/jog        点动（内部维持 JOG_KEEPALIVE）
//! - POST /plc/axis/:n/jog/stop   点动停止
//! - POST /plc/all-stop           全轴停止（优先受理）
//! - POST /plc/reset              整机回参考位
//! - POST /plc/fault-reset        清故障
//! - POST /plc/mode               切模式（1=CBCT 2=DR 3=FLU 4=维护）
//! - POST /plc/cbct/params        CBCT 扫描几何参数组
//! - GET  /plc/scan               扫描进度（已发脉冲/阶段）
//! - WS   /ws（复用）             `WsMessage::PlcStatus` 300ms 推送

#![cfg(not(target_arch = "wasm32"))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use tokio::sync::broadcast;

use crate::device::plc_client::{CbctParams, PlcHandle};
use crate::server::state::ServerState;
use crate::server::WsMessage;

/// 进程级标志：状态泵只起一个（PLC 单实例，够用且免新增 state 字段）
static STATUS_PUMP_STARTED: AtomicBool = AtomicBool::new(false);

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/plc/connect", post(plc_connect))
        .route("/plc/disconnect", post(plc_disconnect))
        .route("/plc/status", get(plc_status))
        .route("/plc/axis/:n/move", post(plc_axis_move))
        .route("/plc/axis/:n/stop", post(plc_axis_stop))
        .route("/plc/axis/:n/home", post(plc_axis_home))
        .route("/plc/axis/:n/jog", post(plc_axis_jog))
        .route("/plc/axis/:n/jog/stop", post(plc_axis_jog_stop))
        .route("/plc/all-stop", post(plc_all_stop))
        .route("/plc/reset", post(plc_machine_reset))
        .route("/plc/fault-reset", post(plc_fault_reset))
        .route("/plc/mode", post(plc_set_mode))
        .route("/plc/cbct/params", post(plc_cbct_params))
        .route("/plc/scan", get(plc_scan_progress))
}

// ======================================================================
// 请求体
// ======================================================================
#[derive(serde::Deserialize)]
pub struct PlcConnectReq {
    /// 可选：覆盖 PLC_ADDR 环境变量（"设备状态-点击连接"时前端可传）
    pub addr: Option<String>,
    /// 可选：覆盖 PLC_PORT（协议明确不得默认 502，必须显式给出）
    pub port: Option<u16>,
    /// 可选：覆盖 PLC_UNIT（默认 1）
    pub unit: Option<u8>,
    /// 心跳周期 ms（Sheet2 未定，默认 500）
    pub heartbeat_ms: Option<u64>,
}

#[derive(serde::Deserialize)]
pub struct AxisMoveReq {
    /// 物理值：角轴 °（含负值），直线轴 mm
    pub position: f64,
    /// 可选：定位速度（°/s 或 mm/s），不传则沿用 PLC 已生效值
    pub speed: Option<f64>,
}

#[derive(serde::Deserialize)]
pub struct AxisJogReq {
    /// +1 = 正点动，-1 = 负点动
    pub direction: i8,
    /// 点动速度（°/s 或 mm/s）
    pub speed: f64,
    /// JOG_KEEPALIVE 周期 ms（默认 200）
    pub keepalive_ms: Option<u64>,
}

#[derive(serde::Deserialize)]
pub struct ModeReq {
    pub mode: u16,
}

#[derive(serde::Deserialize)]
pub struct CbctParamsReq {
    pub start_deg: f64,
    pub end_deg: f64,
    pub direction: u16,
    pub speed_dps: f64,
    pub pulses: u16,
    pub trigger_mode: u16,
}

// ======================================================================
// Handlers
// ======================================================================
/// `POST /plc/connect` — 连接 + 申请控制权 + 启动心跳与状态泵。
/// 平时不在 ServerState::new 里自动连接：设备离线态由前端"点击连接"触发。
pub async fn plc_connect(
    State(state): State<ServerState>,
    Json(req): Json<PlcConnectReq>,
) -> Json<serde_json::Value> {
    let plc = Arc::clone(&state.plc);
    if let Some(addr) = &req.addr {
        let port = req.port.unwrap_or(0);
        if let Err(e) = plc.set_endpoint(addr, port, req.unit.unwrap_or(1)) {
            return Json(serde_json::json!({ "ok": false, "error": e }));
        }
    } else if let Some(unit) = req.unit {
        let (ip, port, _) = plc.info();
        if let Err(e) = plc.set_endpoint(&ip, port, unit) {
            return Json(serde_json::json!({ "ok": false, "error": e }));
        }
    }

    if let Err(e) = plc.connect().await {
        return Json(serde_json::json!({ "ok": false, "error": e }));
    }
    if let Err(e) = plc.take_control().await {
        plc.disconnect().await;
        return Json(serde_json::json!({ "ok": false, "error": format!("控制权申请失败: {e}") }));
    }

    let hb = Duration::from_millis(req.heartbeat_ms.unwrap_or(500).max(50));
    plc.spawn_heartbeat(hb);
    spawn_plc_pump(Arc::clone(&plc), state.ws_tx.clone());

    let (ip, port, unit) = plc.info();
    Json(serde_json::json!({ "ok": true, "addr": ip, "port": port, "unit": unit }))
}

/// `POST /plc/disconnect` — 释放控制权并断链。
pub async fn plc_disconnect(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let plc = Arc::clone(&state.plc);
    let _ = plc.release_control().await;
    plc.disconnect().await;
    Json(serde_json::json!({ "ok": true }))
}

/// `GET /plc/status` — 一次性状态快照（前端也可以直接走 WS 推送）。
pub async fn plc_status(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.plc.snapshot().await {
        Ok(s) => Json(serde_json::to_value(&s).unwrap_or_else(|_| serde_json::json!({ "ok": false }))),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

/// `POST /plc/axis/:n/move` — 绝对定位。
/// 返回 command_id；终态通过 `WsMessage::PlcStatus`（axes[n].result）推送。
pub async fn plc_axis_move(
    State(state): State<ServerState>,
    Path(n): Path<u8>,
    Json(req): Json<AxisMoveReq>,
) -> Json<serde_json::Value> {
    match state.plc.move_axis_absolute(n, req.position, req.speed).await {
        Ok(cmd_id) => Json(serde_json::json!({ "ok": true, "axis": n, "command_id": cmd_id })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_axis_stop(State(state): State<ServerState>, Path(n): Path<u8>) -> Json<serde_json::Value> {
    match state.plc.stop_axis(n).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_axis_home(State(state): State<ServerState>, Path(n): Path<u8>) -> Json<serde_json::Value> {
    match state.plc.home_axis(n).await {
        Ok(cmd_id) => Json(serde_json::json!({ "ok": true, "axis": n, "command_id": cmd_id })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_axis_jog(
    State(state): State<ServerState>,
    Path(n): Path<u8>,
    Json(req): Json<AxisJogReq>,
) -> Json<serde_json::Value> {
    let ka = req.keepalive_ms.unwrap_or(200);
    match state.plc.jog_start(n, req.direction, req.speed, ka).await {
        Ok(()) => Json(serde_json::json!({ "ok": true, "keepalive_ms": ka })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_axis_jog_stop(State(state): State<ServerState>, Path(n): Path<u8>) -> Json<serde_json::Value> {
    match state.plc.jog_stop(n).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_all_stop(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.plc.all_axes_stop().await {
        Ok(()) => Json(serde_json::json!({ "ok": true })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_machine_reset(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.plc.machine_reset().await {
        Ok(cmd_id) => Json(serde_json::json!({ "ok": true, "command_id": cmd_id })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_fault_reset(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.plc.fault_reset().await {
        Ok(()) => Json(serde_json::json!({ "ok": true })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_set_mode(State(state): State<ServerState>, Json(req): Json<ModeReq>) -> Json<serde_json::Value> {
    match state.plc.set_mode(req.mode).await {
        Ok(()) => Json(serde_json::json!({ "ok": true, "requested_mode": req.mode })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_cbct_params(State(state): State<ServerState>, Json(req): Json<CbctParamsReq>) -> Json<serde_json::Value> {
    let p = CbctParams {
        start_deg: req.start_deg,
        end_deg: req.end_deg,
        direction: req.direction,
        speed_dps: req.speed_dps,
        pulses: req.pulses,
        trigger_mode: req.trigger_mode,
    };
    match state.plc.set_cbct_params(p).await {
        Ok(()) => Json(serde_json::json!({ "ok": true })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

pub async fn plc_scan_progress(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.plc.scan_progress().await {
        Ok((sent, frames, phase)) => Json(serde_json::json!({
            "ok": true,
            "pulses_sent": sent,
            "frames_acquired_plc": frames,
            "phase": phase,
        })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

// ======================================================================
// 状态泵：300ms 快照 → WsMessage::PlcStatus 广播（前端状态灯 / 圆盘指针）
// ======================================================================
pub fn spawn_plc_pump(plc: PlcHandle, ws_tx: broadcast::Sender<WsMessage>) {
    if STATUS_PUMP_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(300));
        loop {
            tick.tick().await;
            if !plc.is_connected() {
                continue;
            }
            match plc.snapshot().await {
                Ok(s) => {
                    let payload = serde_json::to_value(&s).unwrap_or_default();
                    let _ = ws_tx.send(WsMessage::PlcStatus { snapshot: payload });
                }
                Err(e) => log::debug!("[plc] snapshot 失败: {e}"),
            }
        }
    });
}
