//! Feishu chat polling sidecar.
//!
//! EchoIsland must not use `lark-cli event +subscribe` on the same Feishu App
//! that a remote bot backend such as OpenClaw consumes. Feishu's WebSocket
//! event stream is single-consumer per App, so two subscribers compete for
//! user messages and the remote bot may stop receiving them.
//!
//! This sidecar only polls message history through `lark-cli im
//! +chat-messages-list`. That is a REST read and does not consume the bot
//! event stream, so the remote OpenClaw bot remains untouched.

use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use echoisland_adapters::{envelope_from_chat_reply, parse_chat_messages_response};
use echoisland_runtime::{RuntimeSnapshot, SharedRuntime};
use serde_json::Value;
use tauri::AppHandle;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{debug, info, warn};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use crate::native_panel_renderer::facade::runtime::{
    NativePanelRuntimeBackend, current_native_panel_runtime_backend,
};

#[cfg(windows)]
const LARK_CLI_BINARY: &str = "lark-cli.cmd";
#[cfg(not(windows))]
const LARK_CLI_BINARY: &str = "lark-cli";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const CHAT_IDS_ENV: &str = "ECHOISLAND_FEISHU_CHAT_IDS";
const POLL_INTERVAL: Duration = Duration::from_secs(4);
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(30);
const HEARTBEAT_INTERVAL_POLLS: u64 = 8;
const POLL_PAGE_SIZE: &str = "20";
const MAX_SEEN_MESSAGE_IDS: usize = 512;

pub fn spawn_feishu_sidecar<R: tauri::Runtime + 'static>(
    app_handle: AppHandle<R>,
    runtime: Arc<SharedRuntime>,
) {
    tauri::async_runtime::spawn(async move {
        run_feishu_poll_supervisor(app_handle, runtime).await;
    });
}

async fn run_feishu_poll_supervisor<R: tauri::Runtime + 'static>(
    app_handle: AppHandle<R>,
    runtime: Arc<SharedRuntime>,
) {
    let env_chat_ids = read_watched_chat_ids_from_env();
    let mut started_chat_ids = HashSet::new();
    let mut discovered_user_chat_ids = Vec::new();
    let mut auto_discovery_attempted = false;
    info!(
        env_chat_ids = ?env_chat_ids,
        "feishu poll supervisor started (REST only, no WebSocket subscription)"
    );

    loop {
        if !auto_discovery_attempted {
            auto_discovery_attempted = true;
            match discover_feishu_p2p_chat_id_from_cli().await {
                Ok(Some(chat_id)) => {
                    info!(
                        chat_id = %chat_id,
                        "discovered feishu p2p chat id from lark-cli user authorization"
                    );
                    discovered_user_chat_ids.push(chat_id);
                }
                Ok(None) => {
                    info!(
                        "could not auto-discover feishu p2p chat id; set {CHAT_IDS_ENV} or run lark-cli auth login"
                    );
                }
                Err(error) => {
                    warn!(%error, "failed to auto-discover feishu p2p chat id");
                }
            }
        }

        let snapshot = runtime.snapshot().await;
        let chat_ids = collect_feishu_chat_ids_from_snapshot(
            &snapshot,
            &env_chat_ids,
            &discovered_user_chat_ids,
        );
        if chat_ids.is_empty() {
            info!("{CHAT_IDS_ENV} empty and no restored feishu sessions; feishu polling is idle");
        }

        for chat_id in chat_ids {
            if !started_chat_ids.insert(chat_id.clone()) {
                continue;
            }

            info!(
                chat_id = %chat_id,
                "starting feishu chat polling sidecar (REST only, no WebSocket subscription)"
            );
            let app_handle = app_handle.clone();
            let runtime = runtime.clone();
            tauri::async_runtime::spawn(async move {
                run_chat_poll_loop(chat_id, app_handle, runtime).await;
            });
        }

        tokio::time::sleep(DISCOVERY_INTERVAL).await;
    }
}

fn read_watched_chat_ids_from_env() -> Vec<String> {
    std::env::var(CHAT_IDS_ENV)
        .map(|value| feishu_chat_ids_from_env_value(&value))
        .unwrap_or_default()
}

fn feishu_chat_ids_from_env_value(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(normalize_feishu_chat_id)
        .collect()
}

fn collect_feishu_chat_ids_from_snapshot(
    snapshot: &RuntimeSnapshot,
    env_chat_ids: &[String],
    discovered_chat_ids: &[String],
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut chat_ids = Vec::new();

    for chat_id in snapshot
        .sessions
        .iter()
        .filter(|session| session.source.eq_ignore_ascii_case("feishu"))
        .filter_map(|session| chat_id_from_feishu_session_id(&session.session_id))
        .chain(env_chat_ids.iter().map(String::as_str))
        .chain(discovered_chat_ids.iter().map(String::as_str))
    {
        if seen.insert(chat_id.to_string()) {
            chat_ids.push(chat_id.to_string());
        }
    }

    chat_ids
}

async fn discover_feishu_p2p_chat_id_from_cli() -> Result<Option<String>, String> {
    let Some(bot_open_id) = current_bot_open_id().await? else {
        return Ok(None);
    };
    resolve_p2p_chat_id_from_user_api(&bot_open_id).await
}

async fn current_bot_open_id() -> Result<Option<String>, String> {
    let mut command = lark_cli_command();
    let output = command
        .args(["api", "GET", "/open-apis/bot/v3/info", "--as", "bot"])
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| format!("spawn lark-cli bot info failed: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "lark-cli bot info failed with status {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: Value = serde_json::from_str(&stdout)
        .map_err(|error| format!("failed to parse lark-cli bot info response: {error}"))?;
    Ok(value
        .get("bot")
        .and_then(|bot| bot.get("open_id"))
        .and_then(Value::as_str)
        .map(str::to_string))
}

async fn resolve_p2p_chat_id_from_user_api(bot_open_id: &str) -> Result<Option<String>, String> {
    let mut command = lark_cli_command();
    let mut child = command
        .args([
            "api",
            "POST",
            "/open-apis/im/v1/chat_p2p/batch_query",
            "--as",
            "user",
            "--data",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("spawn lark-cli p2p chat query failed: {error}"))?;

    let payload = p2p_chat_query_payload(bot_open_id);
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "lark-cli p2p chat query stdin missing".to_string())?;
    stdin
        .write_all(payload.as_bytes())
        .await
        .map_err(|error| format!("write lark-cli p2p chat query stdin failed: {error}"))?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .map_err(|error| format!("wait lark-cli p2p chat query failed: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("need_user_authorization") {
            return Ok(None);
        }
        return Err(format!(
            "lark-cli p2p chat query failed with status {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(chat_id_from_p2p_query_response(&stdout))
}

fn chat_id_from_p2p_query_response(output: &str) -> Option<String> {
    let value: Value = serde_json::from_str(output).ok()?;
    find_first_chat_id(&value)
}

fn p2p_chat_query_payload(bot_open_id: &str) -> String {
    format!(r#"{{"chatter_ids":["{bot_open_id}"]}}"#)
}

fn chat_id_from_feishu_session_id(session_id: &str) -> Option<&str> {
    session_id.strip_prefix("feishu:").and_then(|chat_id| {
        let chat_id = chat_id.trim();
        (!chat_id.is_empty()).then_some(chat_id)
    })
}

fn normalize_feishu_chat_id(value: &str) -> Option<String> {
    let value = value.trim();
    let value = value.strip_prefix("feishu:").unwrap_or(value).trim();
    (!value.is_empty()).then(|| value.to_string())
}

async fn run_chat_poll_loop<R: tauri::Runtime + 'static>(
    chat_id: String,
    app_handle: AppHandle<R>,
    runtime: Arc<SharedRuntime>,
) {
    let mut seen = SeenMessages::new();
    let mut poll_count = 0_u64;

    match poll_chat_messages(&chat_id).await {
        Ok(messages) => {
            for message in &messages {
                if let Some(id) = message.message_id.as_deref() {
                    seen.insert(id.to_string(), message_fingerprint(message));
                }
            }
            debug!(
                chat_id = %chat_id,
                primed = seen.len(),
                "feishu poll primed seen-set"
            );
        }
        Err(error) => {
            warn!(%error, chat_id = %chat_id, "feishu initial poll failed; will retry");
        }
    }

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        match poll_chat_messages(&chat_id).await {
            Ok(messages) => {
                poll_count += 1;
                if poll_count.is_multiple_of(HEARTBEAT_INTERVAL_POLLS) {
                    info!(
                        chat_id = %chat_id,
                        seen_count = seen.len(),
                        latest_message_count = messages.len(),
                        "feishu poll heartbeat"
                    );
                }
                for message in messages.iter().rev() {
                    let Some(id) = message.message_id.clone() else {
                        continue;
                    };
                    let fingerprint = message_fingerprint(message);
                    if seen.contains_same(&id, &fingerprint) {
                        continue;
                    }
                    seen.insert(id.clone(), fingerprint);

                    let Some(envelope) = envelope_from_chat_reply(&chat_id, message) else {
                        continue;
                    };
                    let preview = envelope.message.clone().unwrap_or_default();
                    let _ = runtime.handle_event(envelope).await;
                    info!(
                        chat_id = %chat_id,
                        message_id = %id,
                        message_preview = %preview.chars().take(80).collect::<String>(),
                        "accepted feishu bot reply from poll"
                    );
                    refresh_native_ui_for_feishu_message(app_handle.clone(), runtime.clone());
                }
            }
            Err(error) => {
                warn!(%error, chat_id = %chat_id, "feishu poll failed; will retry");
            }
        }
    }
}

async fn poll_chat_messages(
    chat_id: &str,
) -> Result<Vec<echoisland_adapters::FeishuChatMessage>, String> {
    let mut command = lark_cli_command();
    let output = command
        .args([
            "im",
            "+chat-messages-list",
            "--as",
            "bot",
            "--chat-id",
            chat_id,
            "--page-size",
            POLL_PAGE_SIZE,
            "--format",
            "json",
        ])
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| format!("spawn lark-cli failed: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "lark-cli chat polling failed with status {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_chat_messages_response(&stdout).map_err(|error| error.to_string())
}

fn lark_cli_command() -> Command {
    let mut command = Command::new(LARK_CLI_BINARY);
    hide_child_console_window(&mut command);
    command
}

#[cfg(windows)]
fn hide_child_console_window(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_child_console_window(_: &mut Command) {}

fn message_fingerprint(message: &echoisland_adapters::FeishuChatMessage) -> String {
    format!(
        "{}\n{}\n{}\n{}",
        message.sender_id.as_deref().unwrap_or_default(),
        message.sender_type.as_deref().unwrap_or_default(),
        message.msg_type.as_deref().unwrap_or_default(),
        message.content.as_deref().unwrap_or_default()
    )
}

fn find_first_chat_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => value
            .trim()
            .starts_with("oc_")
            .then(|| value.trim().to_string()),
        Value::Array(items) => items.iter().find_map(find_first_chat_id),
        Value::Object(map) => {
            for key in ["chat_id", "chatId", "open_chat_id", "container_id"] {
                if let Some(chat_id) = map.get(key).and_then(find_first_chat_id) {
                    return Some(chat_id);
                }
            }
            map.values().find_map(find_first_chat_id)
        }
        _ => None,
    }
}

struct SeenMessages {
    order: VecDeque<String>,
    fingerprints: HashMap<String, String>,
}

impl SeenMessages {
    fn new() -> Self {
        Self {
            order: VecDeque::with_capacity(MAX_SEEN_MESSAGE_IDS),
            fingerprints: HashMap::with_capacity(MAX_SEEN_MESSAGE_IDS),
        }
    }

    fn contains_same(&self, id: &str, fingerprint: &str) -> bool {
        self.fingerprints
            .get(id)
            .is_some_and(|current| current == fingerprint)
    }

    fn insert(&mut self, id: String, fingerprint: String) {
        let already_seen = self.fingerprints.contains_key(&id);
        self.fingerprints.insert(id.clone(), fingerprint);
        if already_seen {
            return;
        }
        self.order.push_back(id);
        while self.order.len() > MAX_SEEN_MESSAGE_IDS {
            if let Some(dropped) = self.order.pop_front() {
                self.fingerprints.remove(&dropped);
            }
        }
    }

    fn len(&self) -> usize {
        self.fingerprints.len()
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn refresh_native_ui_for_feishu_message<R: tauri::Runtime + 'static>(
    app_handle: AppHandle<R>,
    runtime: Arc<SharedRuntime>,
) {
    let native_panel_backend = current_native_panel_runtime_backend();
    if !native_panel_backend.native_ui_enabled() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        for delay_ms in [0, 100, 320] {
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            let snapshot = runtime.snapshot().await;
            if let Err(error) = native_panel_backend.update_snapshot(&app_handle, &snapshot) {
                warn!(error = %error, "failed to refresh native island after feishu message");
            }
        }
    });
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn refresh_native_ui_for_feishu_message<R: tauri::Runtime + 'static>(
    _app_handle: AppHandle<R>,
    _runtime: Arc<SharedRuntime>,
) {
}

#[cfg(test)]
mod tests {
    use super::{
        SeenMessages, chat_id_from_feishu_session_id, collect_feishu_chat_ids_from_snapshot,
        feishu_chat_ids_from_env_value, message_fingerprint,
    };
    use echoisland_adapters::FeishuChatMessage;
    use echoisland_runtime::{RuntimeSnapshot, SessionSnapshotView};

    #[test]
    fn chat_ids_are_collected_from_restored_sessions_and_env() {
        let mut snapshot = empty_snapshot();
        snapshot.sessions.push(feishu_session("feishu:oc_1"));

        let chat_ids = collect_feishu_chat_ids_from_snapshot(
            &snapshot,
            &feishu_chat_ids_from_env_value("oc_2, feishu:oc_3, , oc_2"),
            &["oc_4".to_string(), "oc_2".to_string()],
        );

        assert_eq!(chat_ids, vec!["oc_1", "oc_2", "oc_3", "oc_4"]);
    }

    #[test]
    fn p2p_query_parser_ignores_unresolved_placeholder() {
        let output = r#"{"data":{"items":[{"chat_id":"<resolved_chat_id>","user_id":"ou_1"}]}}"#;

        assert_eq!(super::chat_id_from_p2p_query_response(output), None);
    }

    #[test]
    fn p2p_query_parser_extracts_resolved_chat_id() {
        let output = r#"{"data":{"items":[{"chat_id":"oc_1","user_id":"ou_1"}]}}"#;

        assert_eq!(
            super::chat_id_from_p2p_query_response(output).as_deref(),
            Some("oc_1")
        );
    }

    #[test]
    fn p2p_query_payload_uses_feishu_chatter_ids_field() {
        assert_eq!(
            super::p2p_chat_query_payload("ou_1"),
            r#"{"chatter_ids":["ou_1"]}"#
        );
    }

    #[test]
    fn chat_id_parser_rejects_non_feishu_sessions() {
        assert_eq!(
            chat_id_from_feishu_session_id("feishu:oc_cebc89e62febb18a24248a547307684c"),
            Some("oc_cebc89e62febb18a24248a547307684c")
        );
        assert_eq!(chat_id_from_feishu_session_id("claude:abc"), None);
        assert_eq!(chat_id_from_feishu_session_id("feishu:"), None);
    }

    #[test]
    fn seen_set_evicts_oldest_when_full() {
        let mut seen = SeenMessages::new();
        for i in 0..(super::MAX_SEEN_MESSAGE_IDS + 5) {
            seen.insert(format!("id_{i}"), format!("fingerprint_{i}"));
        }
        assert_eq!(seen.len(), super::MAX_SEEN_MESSAGE_IDS);
        assert!(!seen.contains_same("id_0", "fingerprint_0"));
        assert!(seen.contains_same(
            &format!("id_{}", super::MAX_SEEN_MESSAGE_IDS + 4),
            &format!("fingerprint_{}", super::MAX_SEEN_MESSAGE_IDS + 4)
        ));
    }

    #[test]
    fn seen_set_dedupes_repeat_inserts_but_allows_message_updates() {
        let mut seen = SeenMessages::new();
        seen.insert("a".to_string(), "old".to_string());
        seen.insert("a".to_string(), "old".to_string());
        assert_eq!(seen.len(), 1);
        assert!(seen.contains_same("a", "old"));

        seen.insert("a".to_string(), "new".to_string());
        assert_eq!(seen.len(), 1);
        assert!(!seen.contains_same("a", "old"));
        assert!(seen.contains_same("a", "new"));
    }

    #[test]
    fn message_fingerprint_changes_when_content_is_updated() {
        let mut message = chat_message("om_1", "typing...");
        let before = message_fingerprint(&message);
        message.content = Some("final answer".to_string());

        assert_ne!(before, message_fingerprint(&message));
    }

    fn empty_snapshot() -> RuntimeSnapshot {
        RuntimeSnapshot {
            status: "空闲".to_string(),
            primary_source: "claude".to_string(),
            active_session_count: 0,
            total_session_count: 0,
            pending_permission_count: 0,
            pending_question_count: 0,
            pending_permission: None,
            pending_question: None,
            pending_permissions: Vec::new(),
            pending_questions: Vec::new(),
            sessions: Vec::new(),
        }
    }

    fn feishu_session(session_id: &str) -> SessionSnapshotView {
        SessionSnapshotView {
            session_id: session_id.to_string(),
            source: "feishu".to_string(),
            project_name: None,
            cwd: None,
            model: None,
            terminal_app: None,
            terminal_bundle: None,
            host_app: None,
            window_title: None,
            tty: None,
            terminal_pid: None,
            cli_pid: None,
            iterm_session_id: None,
            kitty_window_id: None,
            tmux_env: None,
            tmux_pane: None,
            tmux_client_tty: None,
            status: "空闲".to_string(),
            current_tool: None,
            tool_description: None,
            last_user_prompt: None,
            last_assistant_message: None,
            tool_history_count: 0,
            tool_history: Vec::new(),
            last_activity: chrono::Utc::now(),
        }
    }

    fn chat_message(message_id: &str, content: &str) -> FeishuChatMessage {
        FeishuChatMessage {
            message_id: Some(message_id.to_string()),
            msg_type: Some("post".to_string()),
            content: Some(content.to_string()),
            sender_id: Some("cli_bot".to_string()),
            sender_type: Some("app".to_string()),
            create_time: None,
        }
    }
}
