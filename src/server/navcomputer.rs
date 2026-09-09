//! Navigation Computer 桥接挂载点
//! 提供两类挂载点：
//! 1. 出站（本机输入 → Navigation Computer）：`/api/nav/*`
//!    接收浏览器/脚本的输入（JSON、ZIP、MHA），原样转发到 Navigation Computer
//!    （自动附加 `/api/navigation/v1` 路径前缀与 `Authorization: Bearer` 头），
//!    并把远端响应（状态码 + body）透传回调用方。
//! 2. 入站（Navigation Computer → 本机回调）：`/callbacks/*`
//!    接收配准事件（有序、幂等）与实时导航状态，返回 204 确认，
//!    会话数据可通过 `/api/nav/callbacks/{session}` 查看。
//! 远端地址与 Token（及 DRR 预览落盘目录）通过 `POST /api/nav/config` 设置。

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use std::{fs::{self, File}, io, path::PathBuf};
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
use tokio_util::io::ReaderStream;
use tokio::io::AsyncReadExt;
use axum::{
    body::{to_bytes, Body, Bytes},
    extract::{Path, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use crate::server::state::ServerState;

/// 环境变量名：Navigation Computer Bearer Token
pub const NAV_TOKEN_ENV_VAR: &str = "OLpx9hhRTI_dnUOvyMqRncfnbotXEDPXY7p5eUrC62o";

/// 默认基地址
pub const DEFAULT_NAV_URL: &str = "http://10.20.22.19:8765";

/// 远端 API 路径前缀
pub const NAV_PATH_PREFIX: &str = "/api/navigation/v1";

/// DRR 预览默认落盘目录
pub const DEFAULT_DRR_PREVIEW_DIR: &str = "C:/user/Pet/DRR";

/// 出站转发超时：上传大 ZIP + 准备阶段（validate/prepare/publish）可能较慢
const FORWARD_TIMEOUT: Duration = Duration::from_secs(600); // 10 分钟

/// 原始请求体上限：与全局 DefaultBodyLimit 同量级，覆盖大 ZIP / MHA
const RAW_BODY_LIMIT: usize = 2 * 1024 * 1024 * 1024; // 2 GB
type HandlerError = (StatusCode, String);

// ---------------------------------------------------------------------------
// 共享状态
// ---------------------------------------------------------------------------

/// 单个回调会话：按 sequence 有序、幂等存储的事件 + 最新实时状态。
/// 用 BTreeMap 保证事件按键有序，重复 sequence 直接覆盖（幂等确认）。
#[derive(Default, Serialize, Clone)]
pub struct CallbackSession {
    pub events: BTreeMap<u64, Value>,
    pub live_state: Option<Value>,
}

/// Navigation Computer 桥接共享状态
pub struct NavState {
    /// 远端基地址（scheme + host + port，不含路径前缀）
    pub base_url: RwLock<String>,
    /// Bearer Token（空 = 不带认证头）
    pub token: RwLock<String>,
    /// 已接收的回调会话：session -> 事件/实时状态
    pub callbacks: RwLock<HashMap<String, CallbackSession>>,
    /// 共享 HTTP 客户端（连接复用）
    pub client: reqwest::Client,
    /// 压缩文件存放路径
    pub zip_path: PathBuf,
    /// DRR 预览落盘目录（POST /api/nav/config 可改）
    pub drr_preview_dir: RwLock<PathBuf>,
}
impl NavState {
    pub fn new() -> Self {
        let base_url = DEFAULT_NAV_URL.to_string();
        let token = NAV_TOKEN_ENV_VAR.to_string();
        let client = reqwest::Client::builder()
            .timeout(FORWARD_TIMEOUT)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        let zip_path = PathBuf::from("C:/user/kepler_zip");
        Self {
            base_url: RwLock::new(base_url),
            token: RwLock::new(token),
            callbacks: RwLock::new(HashMap::new()),
            client,
            zip_path,
            drr_preview_dir: RwLock::new(PathBuf::from(DEFAULT_DRR_PREVIEW_DIR)),
        }
    }

    /// 规范化后的IP地址
    pub async fn base(&self) -> String {
        let b = self.base_url.read().await.trim().trim_end_matches('/').to_string();
        if b.is_empty() { DEFAULT_NAV_URL.to_string() } else { b }
    }

    /// 给出站请求附加 Bearer 认证（token 为空则原样返回）
    pub async fn with_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let token = self.token.read().await.clone();
        if token.is_empty() { req } else { req.bearer_auth(token) }
    }
}

impl Default for NavState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

fn upstream_error(e: reqwest::Error) -> HandlerError {
    log::error!("Navigation Computer request failed: {e}");
    (StatusCode::BAD_GATEWAY, format!("Navigation Computer 请求失败: {e}"))
}

/// 从 HTTP 请求/响应的 HeaderMap 中读取 Content-Type 请求头，并把它转换成 Option<String> 返回。
fn content_type_of(headers: &HeaderMap) -> Option<String> {
    headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|s| s.to_string())
}

/// 幂等键来源：Idempotency-Key 请求头优先，回退到 ?idempotency_key= 查询参数
fn idempotency_key_of(
    parts: &axum::http::request::Parts,
    query: &HashMap<String, String>,
) -> Option<String> {
    parts
        .headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| query.get("idempotency_key").map(|s| s.trim()))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// 非空 body 必须是合法 JSON，否则提前 400（输入挂载点的本地校验）
fn validate_json_body(bytes: &Bytes) -> Result<(), HandlerError> {
    if bytes.is_empty() {
        return Ok(());
    }
    serde_json::from_slice::<Value>(bytes).map_err(|e| (StatusCode::BAD_REQUEST, format!("JSON 无效: {e}")))?;
    Ok(())
}

/// 宽松解析回调 body：空 → Null；非法 JSON → 原样存为字符串
fn parse_lenient(bytes: &Bytes) -> Value {
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(bytes).into_owned()))
}

/// 把远端响应（状态码 + Content-Type + body）透传回调用方
async fn relay_response(resp: reqwest::Response) -> Response {
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);
    if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE).cloned() {
        builder = builder.header(header::CONTENT_TYPE, ct);
    }
    let body = resp.bytes().await.unwrap_or_default();
    builder.body(Body::from(body)).unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

/// 远端响应的透传要素：状态码 + Content-Type + Location（202 异步任务的轮询地址）+ body
struct Relayed {
    status: StatusCode,
    content_type: Option<String>,
    location: Option<String>,
    body: Bytes,
}

/// 通用转发：发出请求并取回（状态码 + Content-Type + Location + body 字节），不做透传封装
async fn relay_raw(
    state: &ServerState,
    method: reqwest::Method,
    path: &str,
    body: Bytes,
    content_type: Option<String>,
    idempotency_key: Option<&str>,
) -> Result<Relayed, HandlerError> {
    let url = format!("{}{}{}", state.nav.base().await, NAV_PATH_PREFIX, path); // 拼接远端完整 URL：`{base}{/api/navigation/v1}{path}`
    let mut req = state.nav.with_auth(state.nav.client.request(method, &url)).await;
    if !body.is_empty() {
        req = req
            .header(header::CONTENT_TYPE, content_type.clone().unwrap_or_else(|| "application/json".into()))
            .body(body);
    } else if let Some(ct) = &content_type {
        req = req.header(header::CONTENT_TYPE, ct.clone());
    }
    if let Some(key) = idempotency_key {
        req = req.header("Idempotency-Key", key);
    }
    let resp = req.send().await.map_err(upstream_error)?;
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let location = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body = resp.bytes().await.unwrap_or_default();
    Ok(Relayed {
        status,
        content_type,
        location,
        body,
    })
}

/// JSON 端点通用转发：方法 + 路径 + 原始 body + 原始 Content-Type（透传，含 202 的 Location 头）
async fn relay(
    state: &ServerState,
    method: reqwest::Method,
    path: &str,
    body: Bytes,
    content_type: Option<String>,
    idempotency_key: Option<&str>,
) -> Result<Response, HandlerError> {
    let relayed = relay_raw(state, method, path, body, content_type, idempotency_key).await?;
    let mut builder = Response::builder().status(relayed.status);
    if let Some(ct) = relayed.content_type {
        builder = builder.header(header::CONTENT_TYPE, ct);
    }
    if let Some(loc) = relayed.location {
        builder = builder.header(header::LOCATION, loc);
    }
    Ok(builder
        .body(Body::from(relayed.body))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response()))
}

/// 将目录递归打包成 ZIP
fn create_zip_from_dir(
    source_dir: impl AsRef<std::path::Path>,
    zip_path: impl AsRef<std::path::Path>,
) -> io::Result<()> {
    let source_dir = source_dir.as_ref();
    let zip_path = zip_path.as_ref();
    if !source_dir.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound,format!("源目录不存在: {source_dir:?}")));
    }
    let file = File::create(zip_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    add_dir_to_zip(&mut zip, source_dir, source_dir, options)?;
    zip.finish()?;
    Ok(())
}

/// 递归添加目录内容
fn add_dir_to_zip(
    zip: &mut ZipWriter<File>,
    root_dir: &std::path::Path,
    current_dir: &std::path::Path,
    options: SimpleFileOptions,
) -> io::Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(root_dir).map_err(io::Error::other)?;
        let zip_name = relative_path.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            let dir_name = format!("{zip_name}/");
            zip.add_directory(&dir_name, options)?;
            add_dir_to_zip(zip, root_dir, &path, options)?;
        } else if path.is_file() {
            log::info!("ZIP add file: {}", zip_name);
            zip.start_file(&zip_name, options)?;
            let mut input = File::open(&path)?;
            io::copy(&mut input, zip)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 挂载点清单 / 配置
// ---------------------------------------------------------------------------

/// POST /api/nav/config 请求体：base_url、token、drr_preview_dir 均可选，只更新提供的字段
#[derive(Deserialize)]
pub struct SetNavConfigBody {
    pub base_url: Option<String>,
    pub token: Option<String>,
    pub drr_preview_dir: Option<String>,
}

/// GET /api/nav/config 响应：token 脱敏返回
#[derive(Serialize)]
pub struct NavConfig {
    pub base_url: String,
    pub path_prefix: String,
    pub token_set: bool,
    pub token_masked: Option<String>,
    pub drr_preview_dir: String,
}

fn mask_token(token: &str) -> Option<String> {
    if token.is_empty() {
        None
    } else {
        let prefix: String = token.chars().take(6).collect();
        Some(format!("{prefix}…"))
    }
}

async fn nav_config_of(state: &ServerState) -> NavConfig {
    let token = state.nav.token.read().await.clone();
    NavConfig {
        base_url: state.nav.base().await,
        path_prefix: NAV_PATH_PREFIX.to_string(),
        token_set: !token.is_empty(),
        token_masked: mask_token(&token),
        drr_preview_dir: state.nav.drr_preview_dir.read().await.to_string_lossy().into_owned(),
    }
}

/// GET /api/nav/config — 当前基地址与 Token（脱敏）
pub async fn nav_get_config(State(state): State<ServerState>) -> Json<NavConfig> {
    Json(nav_config_of(&state).await)
}

/// POST /api/nav/config — 设置基地址 / Bearer Token（空 token 表示清除）
pub async fn nav_set_config(
    State(state): State<ServerState>,
    Json(body): Json<SetNavConfigBody>,
) -> Result<Json<NavConfig>, HandlerError> {
    if let Some(base) = body.base_url.as_deref() {
        let url = base.trim().trim_end_matches('/').to_string();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err((StatusCode::BAD_REQUEST, "base_url 必须以 http:// 或 https:// 开头".to_string()));
        }
        *state.nav.base_url.write().await = url.clone();
        log::info!("Navigation Computer base URL set to {url}");
    }
    if let Some(token) = body.token.as_deref() {
        *state.nav.token.write().await = token.trim().to_string();
        log::info!("Navigation Computer bearer token updated (len={})", token.len());
    }
    if let Some(dir) = body.drr_preview_dir.as_deref() {
        let dir = dir.trim();
        let path = if dir.is_empty() { PathBuf::from(DEFAULT_DRR_PREVIEW_DIR) } else { PathBuf::from(dir) };
        log::info!("DRR preview save dir set to {}", path.display());
        *state.nav.drr_preview_dir.write().await = path;
    }
    Ok(Json(nav_config_of(&state).await))
}

// 1. 健康检查与状态
/// GET /api/nav/health → 远端 GET /health
pub async fn nav_health(State(state): State<ServerState>) -> Result<Response, HandlerError> {
    relay(&state, reqwest::Method::GET, "/health", Bytes::new(), None, None).await
}

/// GET /api/nav/status → 远端 GET /status
pub async fn nav_status(State(state): State<ServerState>) -> Result<Response, HandlerError> {
    relay(&state, reqwest::Method::GET, "/status", Bytes::new(), None, None).await
}

// 2. 预准备 CT 查询（缓存优先）
/// POST /api/nav/prepared-ct-lookups — JSON 直通转发
pub async fn nav_prepared_ct_lookup(
    State(state): State<ServerState>,
    req: Request,
) -> Result<Response, HandlerError> {
    let (parts, body) = req.into_parts();
    let bytes = to_bytes(body, RAW_BODY_LIMIT)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("读取请求体失败: {e}")))?;
    validate_json_body(&bytes)?;
    relay(&state, reqwest::Method::POST, "/prepared-ct-lookups", bytes, content_type_of(&parts.headers), None).await
}

// 3. 上传并准备 CT（缓存未命中时）
#[derive(Deserialize)]
pub struct IdempotencyQuery {
    pub idempotency_key: String,
    /// 可选：服务器端源目录（未提供时使用 NavState.zip_path）
    #[serde(default)]
    pub path: Option<String>,
}

/// 通用：把服务器端目录打包成 ZIP，流式上传到远端挂载点
/// （自动附加 Bearer 认证与 Idempotency-Key 头），透传远端响应
async fn zip_dir_and_upload(
    state: &ServerState,
    source_dir: &std::path::Path,
    remote_path: &str,
    idempotency_key: &str,
    zip_prefix: &str,
    multipart: bool
) -> Result<Response, HandlerError> {
    let zip_file = source_dir.parent().unwrap_or(source_dir).join(format!("{zip_prefix}_{idempotency_key}.zip"));
    let zip_file_for_task = zip_file.clone();
    let source_dir_for_task = source_dir.to_path_buf();

    tokio::task::spawn_blocking(move || {
        create_zip_from_dir(&source_dir_for_task, &zip_file_for_task)
    }).await.map_err(|e| {
        log::error!("create ZIP task panicked: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, format!("创建 ZIP 任务失败: {e}"))
    })?.map_err(|e| {
        log::error!("create ZIP failed: source={source_dir:?}, error={e}");
        (StatusCode::INTERNAL_SERVER_ERROR, format!("创建 ZIP 失败: {e}"))
    })?;

    let file = tokio::fs::File::open(&zip_file).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("打开 ZIP 文件失败: path={zip_file:?}, error={e}"))
    })?;

    let url = format!("{}{}{}", state.nav.base().await, NAV_PATH_PREFIX, remote_path);
    let zip_len = file.metadata().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("读取打包大小失败: {e}"))
    })?.len();
    let mut upstream = if multipart {
        let boundary = format!("------------------------{}", &Uuid::new_v4().simple().to_string()[..22]);
        let file_name = zip_file.file_name().and_then(|s| s.to_str()).unwrap_or("raw.zip").to_string();
        let prefix = format!(
            "--{boundary}\r\nContent-Disposition: attachment; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        );
        let suffix = format!("\r\n--{boundary}--\r\n");
        let total_len = prefix.len() as u64 + zip_len + suffix.len() as u64;
        let body_reader = std::io::Cursor::new(prefix.into_bytes())
            .chain(file)
            .chain(std::io::Cursor::new(suffix.into_bytes()));
        state.nav.client
            .post(&url)
            .header(header::CONTENT_TYPE, format!("application/zip; boundary={boundary}"))
            .header(header::CONTENT_LENGTH, total_len)
            .body(reqwest::Body::wrap_stream(ReaderStream::new(body_reader)))
    } else {
        let stream = ReaderStream::new(file);
        let body = reqwest::Body::wrap_stream(stream);
        state.nav.client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/zip")
            .header(header::CONTENT_LENGTH, zip_len)
            .body(body)
    };

    upstream = state.nav.with_auth(upstream).await;
    upstream = upstream.header("Idempotency-Key", idempotency_key);
    log::info!(
        "{remote_path} upload: url={}, zip_len={}, source_dir={:?}, zip_file={:?}, key={:?}",
        url,zip_len, source_dir, zip_file, idempotency_key
    );
    let resp = upstream.send().await.map_err(upstream_error)?;
    if let Err(e) = tokio::fs::remove_file(&zip_file).await {
        log::warn!(
            "删除已上传的 ZIP 失败（不影响响应）: path={:?}, error={}",
            zip_file, e
        );
    }
    Ok(relay_response(resp).await)
}

/// POST /api/nav/prepared-cts — 上传 ZIP 路径，带幂等键
pub async fn nav_prepared_ct_upload(
    State(state): State<ServerState>,
    Query(q): Query<IdempotencyQuery>,
) -> Result<Response, HandlerError> {
    let source_dir = q
        .path
        .map(PathBuf::from)
        .unwrap_or_else(|| state.nav.zip_path.clone());
    if !source_dir.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("目录不存在: {source_dir:?}"),
        ));
    }
    zip_dir_and_upload(&state, &source_dir, "/prepared-cts", &q.idempotency_key, "prepared_ct", false).await
}

// 4. 设置配准（Registration）
/// POST /api/nav/setup-registrations — multipart: payload(JSON) + fixed_mha(F1) + moving_mha(F2)
pub async fn nav_setup_registration(
    State(state): State<ServerState>,
    Query(q): Query<IdempotencyQuery>,
    Json(metadata): Json<serde_json::Value>,
) -> Result<Response, HandlerError> {
    let projection_f1_path = PathBuf::from("C:/user/Pet/DR_0/DR.mha");
    let projection_f2_path = PathBuf::from("C:/user/Pet/DR_90/DR.mha");
    for path in [&projection_f1_path, &projection_f2_path] {
        if !path.is_file() {
            log::error!("setup-registration file not found: {:?}", path);
            return Err((
                StatusCode::BAD_REQUEST,
                format!("文件不存在: {:?}", path),
            ));
        }
    }

    let metadata_json = serde_json::to_string(&metadata)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("JSON 序列化失败: {e}")))?;

    let projection_f1 = reqwest::multipart::Part::file(&projection_f1_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("读取 projection-f1 失败: {:?}: {}", projection_f1_path, e),
            )
        })?
        .mime_str("application/octet-stream")
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("设置 projection-f1 MIME 类型失败: {e}"),
            )
        })?;

    let projection_f2 = reqwest::multipart::Part::file(&projection_f2_path)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("读取 projection-f2 失败: {:?}: {}", projection_f2_path, e),
            )
        })?
        .mime_str("application/octet-stream")
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("设置 projection-f2 MIME 类型失败: {e}"),
            )
        })?;

    let metadata_part = reqwest::multipart::Part::text(metadata_json.clone())
        .file_name("metadata.json")
        .mime_str("application/json")
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("设置 metadata MIME 类型失败: {e}"),
            )
        })?;

    let form = reqwest::multipart::Form::new()
        .part("metadata", metadata_part)
        .part("projection-f1", projection_f1)
        .part("projection-f2", projection_f2);

    let url = format!(
        "{}{}/setup-registrations",
        state.nav.base().await,
        NAV_PATH_PREFIX
    );
    let mut upstream = state.nav.client.post(&url);
    upstream = state.nav.with_auth(upstream).await;
    upstream = upstream.header("Idempotency-Key", q.idempotency_key.clone());
    let resp = upstream
        .multipart(form)
        .send()
        .await
        .map_err(upstream_error)?;
    Ok(relay_response(resp).await)
}

// 5. 实时导航与目标观察
/// POST /api/nav/setup-registrations/{id}
pub async fn nav_target_observation(
    State(state): State<ServerState>,
    Path(id): Path<String>
) -> Result<Response, HandlerError> {
    let path = format!("/setup-registrations/{id}");
    relay(&state, reqwest::Method::GET, &path, Bytes::new(), None, None).await
}

/// GET /api/nav/setup-registrations/{id}/drr-previews/{frame}
/// 远端 GET /setup-registrations/{id}/drr-previews/{frame} — DRR 预览 MHA 字节流透传（frame: F1/F2）
pub async fn nav_drr_preview_save(
    State(state): State<ServerState>,
    Path((id, frame)): Path<(String, String)>,
) -> Result<Response, HandlerError> {
    if frame != "F1" && frame != "F2" {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("frame 必须是 F1 或 F2，收到: {frame}"),
        ));
    }
    let remote_path = format!("/setup-registrations/{id}/drr-previews/{frame}");
    let Relayed {
        status,
        content_type: ct,
        body: data,
        ..
    } = relay_raw(
        &state,
        reqwest::Method::GET,
        &remote_path,
        Bytes::new(),
        None,
        None,
    )
    .await?;
    if !status.is_success() {
        return Err((
            status,
            format!("远端 DRR 预览拉取失败（HTTP {status}），未落盘"),
        ));
    }
    let len = data.len();
    let dir = state.nav.drr_preview_dir.read().await.clone();
    let file_path = dir.join(format!("drr-preview-{frame}.mha"));
    let dir_for_task = dir.clone();
    let file_for_task = file_path.clone();
    let data_for_task = data.clone();
    tokio::task::spawn_blocking(move || {
        fs::create_dir_all(&dir_for_task)?;
        fs::write(&file_for_task, &data_for_task)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("落盘任务失败: {e}"),
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("写入 DRR 预览失败: {file_path:?}: {e}"),
        )
    })?;
    log::info!("DRR preview saved: {file_path:?} ({len} bytes, registration {id})");
    let mut builder = Response::builder().status(StatusCode::OK);
    if let Some(ct) = ct {
        builder = builder.header(header::CONTENT_TYPE, ct);
    }
    Ok(builder
        .body(Body::from(data))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response()))
}

// 6. 查询导航会话（Navigation Session）
/// GET /api/nav/navigation-sessions/{id} → 远端 GET /navigation-sessions/{id}
pub async fn nav_navigation_session(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Response, HandlerError> {
    let path = format!("/navigation-sessions/{id}");
    relay(&state, reqwest::Method::GET, &path, Bytes::new(), None, None).await
}
/// POST /api/nav/navigation-sessions/{id}/completion — JSON 直通转发
pub async fn nav_finish_navigation_session(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    req: Request,
) -> Result<Response, HandlerError> {
    let (parts, body) = req.into_parts();
    let bytes = to_bytes(body, RAW_BODY_LIMIT).await.map_err(|e| (StatusCode::BAD_REQUEST, format!("读取请求体失败: {e}")))?;
    validate_json_body(&bytes)?;
    let idem = idempotency_key_of(&parts, &query);
    if let Some(key) = &idem {
        log::info!("completion relay: idempotency key={key}, session={id}");
    }
    let path = format!("/navigation-sessions/{id}/completion");
    let ct = content_type_of(&parts.headers).or_else(|| Some("application/json".into()));
    relay(&state, reqwest::Method::POST, &path, bytes, ct, idem.as_deref()).await
}

// 7. 3D 配准流程（mag_to_intraop_ct_3d）：原始采集 + 初始观察 + 异步 setup
/// POST /api/nav/raw-acquisitions?path=<服务器端原始数据目录>&idempotency_key=<键>
/// 把服务器端目录打包成 ZIP，流式上传到远端 /raw-acquisitions（同键重放远端幂等返回 200）
#[derive(Deserialize)]
pub struct RawAcquisitionQuery {
    pub path: String,
    pub idempotency_key: String,
}
pub async fn nav_raw_acquisition_upload(
    State(state): State<ServerState>,
    Query(q): Query<RawAcquisitionQuery>,
) -> Result<Response, HandlerError> {
    let source_dir = PathBuf::from(&q.path);
    if !source_dir.is_dir() {
        return Err((StatusCode::BAD_REQUEST, format!("目录不存在: {source_dir:?}")));
    }
    zip_dir_and_upload(&state, &source_dir, "/raw-acquisitions", &q.idempotency_key, "raw_acq", true).await
}

/// POST /api/nav/initial-observations — JSON 直通转发（幂等键：Idempotency-Key 头或 ?idempotency_key=）
pub async fn nav_initial_observations(
    State(state): State<ServerState>,
    Query(query): Query<HashMap<String, String>>,
    req: Request,
) -> Result<Response, HandlerError> {
    let (parts, body) = req.into_parts();
    let bytes = to_bytes(body, RAW_BODY_LIMIT).await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("读取请求体失败: {e}")))?;
    validate_json_body(&bytes)?;
    let idem = idempotency_key_of(&parts, &query);
    relay(&state, reqwest::Method::POST, "/initial-observations", bytes,
        content_type_of(&parts.headers),
        idem.as_deref(),
    ).await
}

/// POST /api/nav/setup-registrations-3d — JSON 直通转发（method=mag_to_intraop_ct_3d，
/// 引用 prepared_ct_id / raw_acquisition_id / initial_observations_id），透传 202 与 Location 头
pub async fn nav_setup_registration_3d(
    State(state): State<ServerState>,
    Query(query): Query<HashMap<String, String>>,
    req: Request,
) -> Result<Response, HandlerError> {
    let (parts, body) = req.into_parts();
    let bytes = to_bytes(body, RAW_BODY_LIMIT)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("读取请求体失败: {e}")))?;
    validate_json_body(&bytes)?;
    let idem = idempotency_key_of(&parts, &query);
    relay(&state, reqwest::Method::POST, "/setup-registrations-3d", bytes,
        content_type_of(&parts.headers).or_else(|| Some("application/json".into())),
        idem.as_deref(),
    ).await
}

/// GET /api/nav/setup-registrations-3d/{id} → 远端 GET /setup-registrations-3d/{id}
/// 轮询异步状态机：accepted → running/failed → succeeded（含 result 或 failure）
pub async fn nav_setup_registration_3d_status(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Response, HandlerError> {
    let path = format!("/setup-registrations-3d/{id}");
    relay(&state, reqwest::Method::GET, &path, Bytes::new(), None, None).await
}

// 8. 注册事件回调（Durable registration events，Navigation Computer → 本机）
/// PUT /callbacks/{session}/events/{sequence} — 有序、幂等接收，204 确认
pub async fn nav_callback_event(
    State(state): State<ServerState>,
    Path((session, sequence)): Path<(String, u64)>,
    body: Bytes,
) -> StatusCode {
    let value = parse_lenient(&body);
    let mut callbacks = state.nav.callbacks.write().await;
    let entry = callbacks.entry(session.clone()).or_default();
    let duplicate = entry.events.insert(sequence, value).is_some();
    if duplicate {
        log::warn!("callback event replay: session={session} sequence={sequence}");
    } else {
        log::info!("callback event: session={session} sequence={sequence}");
    }
    StatusCode::NO_CONTENT
}

/// PUT /callbacks/{session}/live-state — 接收实时导航状态，204 确认
pub async fn nav_callback_live_state(
    State(state): State<ServerState>,
    Path(session): Path<String>,
    body: Bytes,
) -> StatusCode {
    let value = parse_lenient(&body);
    let mut callbacks = state.nav.callbacks.write().await;
    callbacks.entry(session.clone()).or_default().live_state = Some(value);
    log::info!("callback live-state: session={session}");
    StatusCode::NO_CONTENT
}

/// GET /api/nav/callbacks/{session} — 查看已接收的事件与实时状态
pub async fn nav_callback_inspect(
    State(state): State<ServerState>,
    Path(session): Path<String>,
) -> Result<Json<CallbackSession>, StatusCode> {
    let callbacks = state.nav.callbacks.read().await;
    callbacks.get(&session).cloned().map(Json).ok_or(StatusCode::NOT_FOUND)
}

/// DELETE /api/nav/callbacks/{session} — 清空会话回调数据（重放测试用）
pub async fn nav_callback_clear(
    State(state): State<ServerState>,
    Path(session): Path<String>,
) -> StatusCode {
    state.nav.callbacks.write().await.remove(&session);
    log::info!("callback session cleared: {session}");
    StatusCode::OK
}
