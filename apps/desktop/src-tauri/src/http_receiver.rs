use std::{
    collections::HashMap, fs, future::Future, io::ErrorKind, path::PathBuf, pin::Pin, sync::Arc,
    time::Duration,
};

use anyhow::Context;
use chrono::{TimeZone, Utc};
use echoisland_core::{AgentStatus, EventEnvelope, ResponseEnvelope, SessionRecord};
use echoisland_ipc::{IpcAuth, default_token_path};
use echoisland_runtime::SharedRuntime;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, tcp::OwnedReadHalf},
    sync::Mutex,
    time::{sleep, timeout},
};
use tracing::warn;

use crate::{
    diagnostics,
    native_ui_refresh::{maybe_refresh_native_ui_for_event, refresh_native_ui_after_event},
};

pub const DEFAULT_HTTP_RECEIVER_ADDR: &str = "127.0.0.1:37892";
const HTTP_RECEIVER_STATUS_FILE_NAME: &str = "http-receiver.json";
const MAX_HTTP_REQUEST_BYTES: usize = 1_048_576;
const MAX_HTTP_HEALTH_RESPONSE_BYTES: usize = 4_096;
const HTTP_RECEIVER_BIND_RETRY_ATTEMPTS: usize = 12;
const HTTP_RECEIVER_BIND_RETRY_DELAY: Duration = Duration::from_millis(500);
const HTTP_RECEIVER_HEALTH_TIMEOUT: Duration = Duration::from_millis(300);
type DisconnectMonitor = Pin<Box<dyn Future<Output = bool> + Send>>;
type AgentEventStore = Arc<Mutex<HashMap<String, HashMap<String, SessionRecord>>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpReceiverStatus {
    pub addr: String,
    pub event_path: String,
    pub agent_event_path: String,
    pub token_path: String,
    pub status_path: String,
    pub event_url: String,
    pub agent_event_url: String,
    pub fallback: bool,
}

pub fn default_http_receiver_status() -> HttpReceiverStatus {
    http_receiver_status_for_addr(DEFAULT_HTTP_RECEIVER_ADDR, false)
}

pub fn current_http_receiver_status() -> HttpReceiverStatus {
    read_http_receiver_status_file().unwrap_or_else(default_http_receiver_status)
}

pub fn http_receiver_status_path() -> PathBuf {
    echoisland_paths::echoisland_app_dir().join(HTTP_RECEIVER_STATUS_FILE_NAME)
}

fn http_receiver_status_for_addr(addr: &str, fallback: bool) -> HttpReceiverStatus {
    HttpReceiverStatus {
        addr: addr.to_string(),
        event_path: "/event".to_string(),
        agent_event_path: "/agent/events".to_string(),
        token_path: default_token_path().display().to_string(),
        status_path: http_receiver_status_path().display().to_string(),
        event_url: format!("http://{addr}/event"),
        agent_event_url: format!("http://{addr}/agent/events"),
        fallback,
    }
}

fn read_http_receiver_status_file() -> Option<HttpReceiverStatus> {
    let raw = fs::read_to_string(http_receiver_status_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_http_receiver_status_file(status: &HttpReceiverStatus) -> anyhow::Result<()> {
    let path = http_receiver_status_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = serde_json::to_vec_pretty(status)?;
    fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))
}

pub fn spawn_http_receiver<R: tauri::Runtime + 'static>(
    app_handle: tauri::AppHandle<R>,
    runtime: Arc<SharedRuntime>,
) {
    tauri::async_runtime::spawn(async move {
        let auth = match IpcAuth::from_default_storage() {
            Ok(auth) => auth,
            Err(error) => {
                let _ = fs::remove_file(http_receiver_status_path());
                exit_after_http_receiver_failure(
                    &app_handle,
                    "http_receiver_auth_init_failed",
                    &error.to_string(),
                );
                return;
            }
        };

        let agent_events = Arc::new(Mutex::new(HashMap::new()));
        if let Err(error) = serve_http(
            DEFAULT_HTTP_RECEIVER_ADDR,
            app_handle.clone(),
            runtime,
            auth,
            agent_events,
        )
        .await
        {
            let _ = fs::remove_file(http_receiver_status_path());
            exit_after_http_receiver_failure(
                &app_handle,
                "http_receiver_failed",
                &error.to_string(),
            );
        }
    });
}

fn exit_after_http_receiver_failure<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    reason: &'static str,
    error: &str,
) {
    let message = echoisland_i18n::t(if reason == "http_receiver_auth_init_failed" {
        "error.http_auth"
    } else {
        "error.http"
    });
    diagnostics::log_diagnostic_event_always(
        "fatal_http_receiver_error",
        &[
            ("reason", reason.to_string()),
            ("error", error.to_string()),
            (
                "diagnostic_log",
                diagnostics::diagnostic_log_path().display().to_string(),
            ),
        ],
    );
    tracing::error!(reason, error, "fatal http receiver error; exiting app");
    let _ = app_handle.emit("ipc-error", message);
    app_handle.exit(1);
}

async fn serve_http<R: tauri::Runtime + 'static>(
    addr: &str,
    app_handle: tauri::AppHandle<R>,
    runtime: Arc<SharedRuntime>,
    auth: IpcAuth,
    agent_events: AgentEventStore,
) -> anyhow::Result<()> {
    let Some(binding) = bind_http_receiver_listener(addr).await? else {
        tracing::info!("http receiver already active on {addr}; exiting duplicate app instance");
        app_handle.exit(0);
        return Ok(());
    };
    let status = http_receiver_status_for_addr(&binding.addr, binding.fallback);
    if let Err(error) = write_http_receiver_status_file(&status) {
        tracing::warn!(error = %error, "failed to write http receiver status file");
    }
    tracing::info!(
        addr = status.addr,
        fallback = status.fallback,
        "http receiver listening"
    );

    loop {
        let (socket, peer) = binding.listener.accept().await?;
        let app_handle = app_handle.clone();
        let runtime = runtime.clone();
        let auth = auth.clone();
        let agent_events = agent_events.clone();
        tokio::spawn(async move {
            if let Err(error) =
                handle_connection(socket, app_handle, runtime, auth, agent_events).await
            {
                warn!("http receiver connection {peer:?} failed: {error:#}");
            }
        });
    }
}

struct HttpReceiverBinding {
    listener: TcpListener,
    addr: String,
    fallback: bool,
}

async fn bind_http_receiver_listener(addr: &str) -> anyhow::Result<Option<HttpReceiverBinding>> {
    for attempt in 1..=HTTP_RECEIVER_BIND_RETRY_ATTEMPTS {
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                return Ok(Some(HttpReceiverBinding {
                    listener,
                    addr: addr.to_string(),
                    fallback: false,
                }));
            }
            Err(error) if error.kind() == ErrorKind::AddrInUse => {
                if existing_http_receiver_is_healthy(addr).await {
                    return Ok(None);
                }
                warn!(
                    addr,
                    attempt,
                    max_attempts = HTTP_RECEIVER_BIND_RETRY_ATTEMPTS,
                    "http receiver port is in use; retrying"
                );
                sleep(HTTP_RECEIVER_BIND_RETRY_DELAY).await;
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to bind http receiver on {addr}"));
            }
        }
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .with_context(|| format!("failed to bind fallback http receiver after {addr} was busy"))?;
    let fallback_addr = listener
        .local_addr()
        .map(|addr| addr.to_string())
        .context("failed to resolve fallback http receiver address")?;
    diagnostics::log_diagnostic_event_always(
        "http_receiver_fallback_port",
        &[
            ("requested_addr", addr.to_string()),
            ("fallback_addr", fallback_addr.clone()),
        ],
    );
    tracing::warn!(
        requested_addr = addr,
        fallback_addr,
        "http receiver default port remained busy; using fallback port"
    );
    Ok(Some(HttpReceiverBinding {
        listener,
        addr: fallback_addr,
        fallback: true,
    }))
}

async fn existing_http_receiver_is_healthy(addr: &str) -> bool {
    let Ok(Ok(mut stream)) = timeout(HTTP_RECEIVER_HEALTH_TIMEOUT, TcpStream::connect(addr)).await
    else {
        return false;
    };
    let request = b"GET /health HTTP/1.1\r\nHost: echoisland\r\nConnection: close\r\n\r\n";
    if timeout(HTTP_RECEIVER_HEALTH_TIMEOUT, stream.write_all(request))
        .await
        .ok()
        .and_then(Result::ok)
        .is_none()
    {
        return false;
    }

    let mut response = Vec::with_capacity(512);
    let mut buffer = [0_u8; 512];
    loop {
        let read = match timeout(HTTP_RECEIVER_HEALTH_TIMEOUT, stream.read(&mut buffer)).await {
            Ok(Ok(read)) => read,
            Ok(Err(_)) | Err(_) => return http_receiver_health_response_is_ok(&response),
        };
        if read == 0 {
            break;
        }
        response.extend_from_slice(&buffer[..read]);
        if http_receiver_health_response_is_ok(&response) {
            return true;
        }
        if response.len() >= MAX_HTTP_HEALTH_RESPONSE_BYTES {
            break;
        }
    }
    http_receiver_health_response_is_ok(&response)
}

fn http_receiver_health_response_is_ok(response: &[u8]) -> bool {
    let response = String::from_utf8_lossy(response);
    response.starts_with("HTTP/1.1 200") && response.contains("echoisland-http-receiver")
}

async fn handle_connection<R: tauri::Runtime + 'static>(
    mut socket: TcpStream,
    app_handle: tauri::AppHandle<R>,
    runtime: Arc<SharedRuntime>,
    auth: IpcAuth,
    agent_events: AgentEventStore,
) -> anyhow::Result<()> {
    let request = read_http_request(&mut socket).await?;
    match decode_http_request(&request, &auth) {
        DecodedHttpRequest::Immediate(response) => {
            socket.write_all(&response).await?;
            socket.shutdown().await?;
        }
        DecodedHttpRequest::Event(event_request) => {
            let (read_half, mut write_half) = socket.into_split();
            match await_http_event_response(
                runtime.clone(),
                event_request.event,
                if is_blocking_event_name(&event_request.normalized) {
                    Some(Box::pin(wait_for_http_disconnect(read_half)) as DisconnectMonitor)
                } else {
                    None
                },
            )
            .await
            {
                EventExecutionResult::Response(response) => {
                    if is_blocking_event_name(&event_request.normalized) {
                        let snapshot = runtime.snapshot().await;
                        warn!(
                            event_name = %event_request.normalized,
                            pending_permission_count = snapshot.pending_permission_count,
                            pending_question_count = snapshot.pending_question_count,
                            active_session_count = snapshot.active_session_count,
                            "http receiver applied pending event"
                        );
                    }
                    refresh_native_ui_after_event(
                        app_handle.clone(),
                        runtime.clone(),
                        &event_request.normalized,
                    )
                    .await;
                    maybe_refresh_native_ui_for_event(
                        app_handle,
                        runtime,
                        &event_request.normalized,
                    );
                    let encoded = json_http_response(200, &response);
                    write_half.write_all(&encoded).await?;
                    write_half.shutdown().await?;
                }
                EventExecutionResult::Disconnected => {
                    maybe_refresh_native_ui_for_event(app_handle, runtime, "PeerDisconnect");
                }
            }
        }
        DecodedHttpRequest::AgentEvent(agent_request) => {
            let response = apply_agent_event(runtime, agent_events, agent_request.event).await;
            socket
                .write_all(&json_http_response(200, &response))
                .await?;
            socket.shutdown().await?;
        }
    }
    Ok(())
}

#[derive(Debug)]
struct HttpEventRequest {
    event: EventEnvelope,
    normalized: String,
}

#[derive(Debug)]
struct HttpAgentEventRequest {
    event: AgentPluginEvent,
}

enum DecodedHttpRequest {
    Immediate(Vec<u8>),
    Event(HttpEventRequest),
    AgentEvent(HttpAgentEventRequest),
}

fn decode_http_request(request_bytes: &[u8], auth: &IpcAuth) -> DecodedHttpRequest {
    let request = match parse_http_request(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return DecodedHttpRequest::Immediate(json_http_response(
                400,
                &ResponseEnvelope::error(format!("bad_request:{error}")),
            ));
        }
    };

    if request.method == "GET" && request.path == "/health" {
        return DecodedHttpRequest::Immediate(ok_json_bytes(serde_json::json!({
            "ok": true,
            "service": "echoisland-http-receiver",
            "event_path": "/event",
            "agent_event_path": "/agent/events"
        })));
    }

    if request.method != "POST" || (request.path != "/event" && request.path != "/agent/events") {
        return DecodedHttpRequest::Immediate(json_http_response(
            404,
            &ResponseEnvelope::error("not_found"),
        ));
    }

    let provided_token = request_token(&request);

    if provided_token.as_deref() != Some(auth.token()) {
        return DecodedHttpRequest::Immediate(json_http_response(
            401,
            &ResponseEnvelope::error("unauthorized"),
        ));
    }

    if request.path == "/agent/events" {
        let event = match parse_agent_event_body(&request.body) {
            Ok(event) => event,
            Err(response) => {
                return DecodedHttpRequest::Immediate(json_http_response(400, &response));
            }
        };
        return DecodedHttpRequest::AgentEvent(HttpAgentEventRequest { event });
    }

    let event = match parse_event_body(&request.body) {
        Ok(event) => event,
        Err(response) => return DecodedHttpRequest::Immediate(json_http_response(400, &response)),
    };

    let normalized = event.normalized_event_name();
    warn!(
        method = %request.method,
        path = %request.path,
        event_name = %normalized,
        body_len = request.body.len(),
        "http receiver received event"
    );
    DecodedHttpRequest::Event(HttpEventRequest { event, normalized })
}

fn request_token(request: &ParsedHttpRequest) -> Option<String> {
    request
        .headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_string)
        .or_else(|| request.headers.get("x-echoisland-token").cloned())
        .or_else(|| {
            serde_json::from_slice::<serde_json::Value>(&request.body)
                .ok()
                .and_then(|value| {
                    value
                        .get("token")
                        .and_then(|token| token.as_str())
                        .map(str::to_string)
                })
        })
}

enum EventExecutionResult {
    Response(ResponseEnvelope),
    Disconnected,
}

async fn await_http_event_response(
    runtime: Arc<SharedRuntime>,
    event: EventEnvelope,
    disconnect_monitor: Option<DisconnectMonitor>,
) -> EventExecutionResult {
    if let Some(disconnect_monitor) = disconnect_monitor {
        let session_id = event.session_id.clone();
        let mut response_task = tokio::spawn({
            let runtime = runtime.clone();
            async move { runtime.handle_event(event).await }
        });
        tokio::select! {
            response = &mut response_task => EventExecutionResult::Response(
                response.unwrap_or_else(|_| ResponseEnvelope::error("handler_cancelled"))
            ),
            disconnected = disconnect_monitor => {
                if disconnected {
                    runtime.handle_peer_disconnect(&session_id).await;
                    let _ = response_task.await;
                    EventExecutionResult::Disconnected
                } else {
                    EventExecutionResult::Response(
                        response_task
                            .await
                            .unwrap_or_else(|_| ResponseEnvelope::error("handler_cancelled"))
                    )
                }
            }
        }
    } else {
        EventExecutionResult::Response(runtime.handle_event(event).await)
    }
}

fn is_blocking_event_name(normalized: &str) -> bool {
    matches!(normalized, "PermissionRequest" | "AskUserQuestion")
}

fn parse_event_body(body: &[u8]) -> Result<EventEnvelope, ResponseEnvelope> {
    let value = serde_json::from_slice::<serde_json::Value>(body)
        .map_err(|_| ResponseEnvelope::parse_failed())?;
    let event_value = value
        .get("event")
        .filter(|event| event.is_object())
        .cloned()
        .unwrap_or(value);
    let event: EventEnvelope =
        serde_json::from_value(event_value).map_err(|_| ResponseEnvelope::parse_failed())?;
    event
        .validate()
        .map_err(|_| ResponseEnvelope::invalid_payload())?;
    Ok(event)
}

#[derive(Debug, Clone, Deserialize)]
struct AgentPluginEvent {
    #[serde(default)]
    version: u32,
    source_id: String,
    #[serde(default)]
    source_type: Option<String>,
    #[serde(default)]
    ide: Option<String>,
    #[serde(default)]
    workspace_path: Option<String>,
    session_id: String,
    #[serde(default)]
    title: Option<String>,
    status: String,
    #[serde(default)]
    event: Option<String>,
    #[serde(default)]
    updated_at: Option<i64>,
    #[serde(default)]
    metadata: serde_json::Value,
}

fn parse_agent_event_body(body: &[u8]) -> Result<AgentPluginEvent, ResponseEnvelope> {
    let value = serde_json::from_slice::<serde_json::Value>(body)
        .map_err(|_| ResponseEnvelope::parse_failed())?;
    let event_value = value
        .get("event")
        .filter(|event| event.is_object())
        .cloned()
        .unwrap_or(value);
    let event = serde_json::from_value::<AgentPluginEvent>(event_value)
        .map_err(|_| ResponseEnvelope::parse_failed())?;
    if event.version != 1
        || event.source_id.trim().is_empty()
        || event.session_id.trim().is_empty()
        || event.status.trim().is_empty()
    {
        return Err(ResponseEnvelope::invalid_payload());
    }
    Ok(event)
}

async fn apply_agent_event(
    runtime: Arc<SharedRuntime>,
    store: AgentEventStore,
    event: AgentPluginEvent,
) -> ResponseEnvelope {
    let source = event.source_id.trim().to_string();
    let session_id = event.session_id.trim().to_string();
    let event_name = event.event.as_deref().unwrap_or_default();

    let records = {
        let mut store = store.lock().await;
        let source_sessions = store.entry(source.clone()).or_default();
        if event_name == "session_closed" {
            source_sessions.remove(&session_id);
        } else {
            source_sessions.insert(
                session_id.clone(),
                session_record_from_agent_event(&source, &session_id, event),
            );
        }
        source_sessions.values().cloned().collect::<Vec<_>>()
    };

    runtime.sync_source_sessions(&source, records).await;
    ResponseEnvelope::ok()
}

fn session_record_from_agent_event(
    source: &str,
    session_id: &str,
    event: AgentPluginEvent,
) -> SessionRecord {
    let status = agent_event_status(&event);
    let timestamp = event
        .updated_at
        .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single())
        .unwrap_or_else(Utc::now);
    let title = event.title.clone().filter(|value| !value.trim().is_empty());
    let workspace_path = event.workspace_path.clone();
    let project_name = event
        .workspace_path
        .as_deref()
        .and_then(path_name)
        .or_else(|| title.clone());
    let host_app = event
        .ide
        .as_deref()
        .and_then(ide_host_app)
        .or_else(|| {
            event
                .source_type
                .as_deref()
                .filter(|value| !value.is_empty())
        })
        .map(ToOwned::to_owned);

    SessionRecord {
        session_id: session_id.to_string(),
        source: source.to_string(),
        cwd: workspace_path,
        model: event
            .metadata
            .get("model")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        project_name,
        terminal_app: None,
        terminal_bundle: None,
        host_app,
        window_title: title.clone(),
        tty: None,
        terminal_pid: None,
        cli_pid: None,
        iterm_session_id: None,
        kitty_window_id: None,
        tmux_env: None,
        tmux_pane: None,
        tmux_client_tty: None,
        status,
        current_tool: None,
        tool_description: None,
        last_user_prompt: title.clone(),
        last_assistant_message: completion_message(&event),
        tool_history: Vec::new(),
        last_activity: timestamp,
    }
}

fn agent_event_status(event: &AgentPluginEvent) -> AgentStatus {
    match event.event.as_deref().unwrap_or_default() {
        "session_waiting" => return AgentStatus::WaitingQuestion,
        "session_started" | "session_updated" => return AgentStatus::Running,
        "session_completed" | "session_failed" | "session_closed" => return AgentStatus::Idle,
        _ => {}
    }

    match event.status.to_ascii_lowercase().as_str() {
        "waiting" => AgentStatus::WaitingQuestion,
        "running" | "started" | "updated" => AgentStatus::Running,
        "completed" | "complete" | "failed" | "error" | "closed" | "idle" => AgentStatus::Idle,
        _ => AgentStatus::Running,
    }
}

fn completion_message(event: &AgentPluginEvent) -> Option<String> {
    match event.event.as_deref().unwrap_or_default() {
        "session_completed" => return Some(echoisland_i18n::t("completion.done").to_owned()),
        "session_failed" => return Some(echoisland_i18n::t("completion.failed").to_owned()),
        _ => {}
    }

    match event.status.to_ascii_lowercase().as_str() {
        "completed" | "complete" => Some(echoisland_i18n::t("completion.done").to_owned()),
        "failed" | "error" => Some(echoisland_i18n::t("completion.failed").to_owned()),
        _ => None,
    }
}

fn ide_host_app(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "vscode" | "code" => Some("com.microsoft.VSCode"),
        "cursor" => Some("com.todesktop.230313mzl4w4u92"),
        "trae" => Some("com.trae.app"),
        _ => None,
    }
}

fn path_name(value: &str) -> Option<String> {
    value
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
}

struct ParsedHttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn parse_http_request(bytes: &[u8]) -> anyhow::Result<ParsedHttpRequest> {
    let separator = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow::anyhow!("missing header separator"))?;
    let (header_bytes, body_bytes) = bytes.split_at(separator + 4);
    let header_text =
        String::from_utf8(header_bytes[..separator].to_vec()).context("headers are not utf-8")?;
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing method"))?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing path"))?
        .to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let declared_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(body_bytes.len());
    if body_bytes.len() < declared_length {
        anyhow::bail!("incomplete request body");
    }

    Ok(ParsedHttpRequest {
        method,
        path,
        headers,
        body: body_bytes[..declared_length].to_vec(),
    })
}

async fn read_http_request(socket: &mut TcpStream) -> anyhow::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 8192];
    let mut header_end = None;
    let mut expected_total_len = None;

    loop {
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_HTTP_REQUEST_BYTES {
            anyhow::bail!("http request exceeds {MAX_HTTP_REQUEST_BYTES} bytes");
        }

        if header_end.is_none() {
            header_end = buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| index + 4);
            if let Some(header_end_pos) = header_end {
                let header_text = String::from_utf8_lossy(&buffer[..header_end_pos]);
                let content_length = header_text
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(name, value)| {
                            if name.trim().eq_ignore_ascii_case("content-length") {
                                value.trim().parse::<usize>().ok()
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or(0);
                expected_total_len = Some(header_end_pos + content_length);
            }
        }

        if let Some(expected_total_len) = expected_total_len
            && buffer.len() >= expected_total_len
        {
            break;
        }
    }

    Ok(buffer)
}

async fn wait_for_http_disconnect(mut reader: OwnedReadHalf) -> bool {
    let mut probe = [0_u8; 1];
    match reader.read(&mut probe).await {
        Ok(0) => true,
        Ok(_) => true,
        Err(error) => {
            warn!(error = %error, "http disconnect monitor ended with read error");
            true
        }
    }
}

fn ok_json_bytes(value: serde_json::Value) -> Vec<u8> {
    let body = serde_json::to_vec(&value).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
    http_response(200, "application/json", &body)
}

fn json_http_response(status: u16, response: &ResponseEnvelope) -> Vec<u8> {
    let body = serde_json::to_vec(response).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
    http_response(status, "application/json", &body)
}

fn http_response(status: u16, content_type: &str, body: &[u8]) -> Vec<u8> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use echoisland_core::{EventEnvelope, PROTOCOL_VERSION};
    use echoisland_runtime::SharedRuntime;
    use serde_json::json;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::oneshot,
        time::{Duration, sleep},
    };

    use super::{
        AgentEventStore, DecodedHttpRequest, EventExecutionResult, IpcAuth, apply_agent_event,
        await_http_event_response, completion_message, decode_http_request,
        existing_http_receiver_is_healthy, parse_agent_event_body, parse_http_request,
    };

    #[test]
    fn parses_http_request_with_headers() {
        let request = b"POST /event HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\nX-EchoIsland-Token: secret\r\n\r\n{}";
        let parsed = parse_http_request(request).unwrap();
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.path, "/event");
        assert_eq!(
            parsed.headers.get("x-echoisland-token").map(String::as_str),
            Some("secret")
        );
        assert_eq!(parsed.body, b"{}");
    }

    #[test]
    fn generated_agent_completion_messages_use_the_default_locale() {
        let completed = parse_agent_event_body(
            serde_json::to_vec(&json!({
                "version": 1,
                "source_id": "qa-agent",
                "session_id": "qa-completed",
                "status": "completed",
                "event": "session_completed"
            }))
            .unwrap()
            .as_slice(),
        )
        .unwrap();
        let failed = parse_agent_event_body(
            serde_json::to_vec(&json!({
                "version": 1,
                "source_id": "qa-agent",
                "session_id": "qa-failed",
                "status": "failed",
                "event": "session_failed"
            }))
            .unwrap()
            .as_slice(),
        )
        .unwrap();

        assert_eq!(completion_message(&completed).as_deref(), Some("已完成"));
        assert_eq!(completion_message(&failed).as_deref(), Some("执行失败"));
    }

    #[tokio::test]
    async fn health_probe_accepts_split_response_body() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 128];
            let _ = socket.read(&mut request).await.unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 49\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
            sleep(Duration::from_millis(10)).await;
            socket
                .write_all(br#"{"ok":true,"service":"echoisland-http-receiver"}"#)
                .await
                .unwrap();
        });

        assert!(existing_http_receiver_is_healthy(&addr).await);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn accepts_authenticated_http_event() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let storage_path = std::env::temp_dir().join(format!("echoisland-http-test-{suffix}.json"));
        let runtime = Arc::new(SharedRuntime::with_storage_path(storage_path.clone()));
        let auth = IpcAuth::from_token("secret");
        let body = serde_json::to_string(&json!({
            "protocol_version": PROTOCOL_VERSION,
            "hook_event_name": "SessionStart",
            "source": "opencode",
            "session_id": "open-1",
            "timestamp": chrono::Utc::now(),
        }))
        .unwrap();
        let request = format!(
            "POST /event HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer secret\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let decoded = decode_http_request(request.as_bytes(), &auth);
        let event = match decoded {
            DecodedHttpRequest::Event(request) => request.event,
            DecodedHttpRequest::Immediate(_) | DecodedHttpRequest::AgentEvent(_) => {
                panic!("expected event request")
            }
        };
        let response = await_http_event_response(runtime.clone(), event, None).await;
        let response = match response {
            EventExecutionResult::Response(response) => response,
            EventExecutionResult::Disconnected => panic!("request should not disconnect"),
        };
        assert!(response.ok);
        let snapshot = runtime.snapshot().await;
        assert_eq!(snapshot.total_session_count, 1);
        let _ = std::fs::remove_file(storage_path);
    }

    #[tokio::test]
    async fn accepts_authenticated_agent_event() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let storage_path =
            std::env::temp_dir().join(format!("echoisland-agent-event-test-{suffix}.json"));
        let runtime = Arc::new(SharedRuntime::with_storage_path(storage_path.clone()));
        let auth = IpcAuth::from_token("secret");
        let body = serde_json::to_string(&json!({
            "version": 1,
            "source_id": "vscode-codex",
            "source_type": "extension",
            "ide": "vscode",
            "workspace_path": "/tmp/demo",
            "session_id": "agent-1",
            "title": "Refactor pet animation",
            "status": "running",
            "updated_at": chrono::Utc::now().timestamp(),
            "metadata": { "model": "gpt-5" }
        }))
        .unwrap();
        let request = format!(
            "POST /agent/events HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer secret\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );

        let decoded = decode_http_request(request.as_bytes(), &auth);
        let event = match decoded {
            DecodedHttpRequest::AgentEvent(request) => request.event,
            DecodedHttpRequest::Immediate(_) | DecodedHttpRequest::Event(_) => {
                panic!("expected agent event request")
            }
        };
        let store: AgentEventStore = Arc::new(tokio::sync::Mutex::new(Default::default()));
        let response = apply_agent_event(runtime.clone(), store, event).await;

        assert!(response.ok);
        let snapshot = runtime.snapshot().await;
        assert_eq!(snapshot.total_session_count, 1);
        assert_eq!(snapshot.sessions[0].source, "vscode-codex");
        assert_eq!(snapshot.sessions[0].status, "Running");
        assert_eq!(
            snapshot.sessions[0].host_app.as_deref(),
            Some("com.microsoft.VSCode")
        );
        assert_eq!(snapshot.sessions[0].project_name.as_deref(), Some("demo"));
        let _ = std::fs::remove_file(storage_path);
    }

    #[tokio::test]
    async fn blocking_http_request_is_cleared_when_client_disconnects() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let storage_path =
            std::env::temp_dir().join(format!("echoisland-http-disconnect-test-{suffix}.json"));
        let runtime = Arc::new(SharedRuntime::with_storage_path(storage_path.clone()));
        let event = EventEnvelope {
            protocol_version: PROTOCOL_VERSION.to_string(),
            hook_event_name: "PermissionRequest".to_string(),
            session_id: "http-session".to_string(),
            source: "claude".to_string(),
            timestamp: chrono::Utc::now(),
            tool_name: Some("Bash".to_string()),
            tool_input: Some(json!({ "command": "ls" })),
            cwd: Some("/tmp".to_string()),
            model: None,
            message: None,
            agent_id: None,
            metadata: None,
            question: None,
        };
        let (disconnect_tx, disconnect_rx) = oneshot::channel::<()>();
        let response_task = tokio::spawn({
            let runtime = runtime.clone();
            async move {
                await_http_event_response(
                    runtime,
                    event,
                    Some(Box::pin(async move {
                        let _ = disconnect_rx.await;
                        true
                    })),
                )
                .await
            }
        });

        for _ in 0..50 {
            if runtime.snapshot().await.pending_permission_count == 1 {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        assert_eq!(runtime.snapshot().await.pending_permission_count, 1);
        let _ = disconnect_tx.send(());

        let result = response_task.await.unwrap();
        assert!(matches!(result, EventExecutionResult::Disconnected));

        let snapshot = runtime.snapshot().await;
        assert_eq!(snapshot.pending_permission_count, 0);
        assert_eq!(snapshot.pending_question_count, 0);
        assert_eq!(snapshot.sessions[0].status, "Processing");
        let _ = std::fs::remove_file(storage_path);
    }
}
