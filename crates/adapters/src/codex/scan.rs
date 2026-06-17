use std::{
    collections::{HashMap, HashSet},
    fs,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use echoisland_core::{AgentStatus, SessionRecord};
use serde_json::Value;
use tracing::debug;

use crate::platform_support::codex_running_process_limit;

use super::CodexPaths;

const SESSION_SCAN_LIMIT: usize = 16;
const SESSION_HEAD_LINES: usize = 96;
const SESSION_TAIL_BYTES: u64 = 64 * 1024;
const ACTIVE_WINDOW_SECS: i64 = 300;
const ACTIVE_SCAN_INTERVAL_SECS: u64 = 3;
const IDLE_SCAN_INTERVAL_SECS: u64 = 15;
const CODEX_APP_THREAD_SCAN_LIMIT: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
struct HistoryEntry {
    timestamp: DateTime<Utc>,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexAppThread {
    session_id: String,
    rollout_path: Option<PathBuf>,
    cwd: Option<String>,
    model: Option<String>,
    title: Option<String>,
    first_user_message: Option<String>,
    source: Option<String>,
    thread_source: Option<String>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskMarker {
    Started,
    Complete,
    Aborted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskSignal {
    marker: TaskMarker,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TaskActivity {
    latest_signal: Option<TaskSignal>,
    open_task_started_at: Option<DateTime<Utc>>,
    active_task_count: usize,
}

#[derive(Debug, Clone, Default)]
struct TaskActivityTracker {
    active_tasks: HashMap<String, DateTime<Utc>>,
    latest_signal: Option<TaskSignal>,
}

#[derive(Debug, Clone, Default)]
struct TaskActivityScanState {
    offset: u64,
    tracker: TaskActivityTracker,
}

#[derive(Debug, Clone, Default)]
struct HistoryScanState {
    size: u64,
    modified_at: Option<DateTime<Utc>>,
    offset: u64,
    latest_prompt_by_session: HashMap<String, HistoryEntry>,
}

#[derive(Debug, Clone, Default)]
struct CodexAppThreadScanState {
    db_path: Option<PathBuf>,
    modified_at: Option<DateTime<Utc>>,
    threads: Vec<CodexAppThread>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedSessionFile {
    session_id: String,
    originator: Option<String>,
    cwd: Option<String>,
    model: Option<String>,
    last_activity: DateTime<Utc>,
    last_assistant_message: Option<String>,
    task_activity: TaskActivity,
}

#[derive(Debug, Clone)]
struct SessionFileState {
    size: u64,
    modified_at: DateTime<Utc>,
    parsed: Option<ParsedSessionFile>,
    task_activity_state: TaskActivityScanState,
}

#[derive(Debug, Clone)]
pub struct CodexSessionScanner {
    paths: CodexPaths,
    history_state: HistoryScanState,
    app_thread_state: CodexAppThreadScanState,
    session_files: HashMap<PathBuf, SessionFileState>,
    last_sessions: Vec<SessionRecord>,
}

impl CodexSessionScanner {
    pub fn new(paths: CodexPaths) -> Self {
        Self {
            paths,
            history_state: HistoryScanState::default(),
            app_thread_state: CodexAppThreadScanState::default(),
            session_files: HashMap::new(),
            last_sessions: Vec::new(),
        }
    }

    pub fn scan(&mut self) -> Result<Option<Vec<SessionRecord>>> {
        let history_path = self.paths.codex_dir.join("history.jsonl");
        refresh_history_state(&history_path, &mut self.history_state)?;

        let session_root = self.paths.codex_dir.join("sessions");
        let app_threads = refresh_codex_app_thread_state(
            &self.paths.codex_dir,
            CODEX_APP_THREAD_SCAN_LIMIT,
            &mut self.app_thread_state,
        )
        .unwrap_or_else(|error| {
            debug!(
                error = %error,
                "failed to load Codex app thread index; falling back to cached app threads"
            );
            self.app_thread_state.threads.clone()
        });
        let app_thread_by_id = app_threads
            .iter()
            .map(|thread| (thread.session_id.clone(), thread.clone()))
            .collect::<HashMap<_, _>>();
        let mut session_paths = recent_session_files(&session_root, SESSION_SCAN_LIMIT)?;
        for path in app_threads
            .iter()
            .filter_map(|thread| thread.rollout_path.as_ref())
            .filter(|path| path.exists())
        {
            if !session_paths.iter().any(|candidate| candidate == path) {
                session_paths.push(path.clone());
            }
        }
        let interesting_paths = session_paths.iter().cloned().collect::<HashSet<_>>();

        self.session_files
            .retain(|path, _| interesting_paths.contains(path));

        for path in &session_paths {
            let modified_at = file_modified_utc(path)?;
            let size = file_size(path)?;
            let needs_refresh = self
                .session_files
                .get(path)
                .map(|state| state.size != size || state.modified_at != modified_at)
                .unwrap_or(true);

            if needs_refresh {
                let previous_task_activity = self
                    .session_files
                    .get(path)
                    .map(|state| &state.task_activity_state);
                let (parsed, task_activity_state) =
                    parse_session_file_with_task_activity_state(path, previous_task_activity)?;
                self.session_files.insert(
                    path.clone(),
                    SessionFileState {
                        size,
                        modified_at,
                        parsed,
                        task_activity_state,
                    },
                );
            }
        }

        let mut sessions = self
            .session_files
            .values()
            .filter_map(|state| {
                state.parsed.as_ref().map(|parsed| {
                    build_session_record(
                        parsed,
                        &self.history_state.latest_prompt_by_session,
                        app_thread_by_id.get(&parsed.session_id),
                    )
                })
            })
            .collect::<Vec<_>>();
        let parsed_session_ids = sessions
            .iter()
            .map(|session| session.session_id.clone())
            .collect::<HashSet<_>>();
        sessions.extend(
            app_threads
                .iter()
                .filter(|thread| !parsed_session_ids.contains(&thread.session_id))
                .map(build_app_thread_record),
        );

        sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        if let Some(process_count) = codex_running_process_limit(&self.paths.home_dir) {
            sessions.truncate(process_count);
        }

        if sessions == self.last_sessions {
            return Ok(None);
        }

        self.last_sessions = sessions.clone();
        Ok(Some(sessions))
    }

    pub fn recommended_poll_interval(&self) -> Duration {
        let now = Utc::now();
        let recently_active = self.last_sessions.iter().any(|session| {
            session.status != AgentStatus::Idle
                || (now - session.last_activity).num_seconds() <= ACTIVE_WINDOW_SECS
        });

        if recently_active {
            Duration::from_secs(ACTIVE_SCAN_INTERVAL_SECS)
        } else {
            Duration::from_secs(IDLE_SCAN_INTERVAL_SECS)
        }
    }
}

pub fn scan_codex_sessions(paths: &CodexPaths) -> Result<Vec<SessionRecord>> {
    let mut scanner = CodexSessionScanner::new(paths.clone());
    Ok(scanner.scan()?.unwrap_or_default())
}

fn parse_session_file_with_task_activity_state(
    path: &Path,
    previous_task_activity: Option<&TaskActivityScanState>,
) -> Result<(Option<ParsedSessionFile>, TaskActivityScanState)> {
    let head_lines = read_head_lines(path, SESSION_HEAD_LINES)?;
    let tail_lines = read_tail_lines(path, SESSION_TAIL_BYTES)?;
    let task_activity_state = scan_task_activity_incremental(path, previous_task_activity)?;

    let mut session_id = None;
    let mut originator = None;
    let mut cwd = None;
    let mut model = None;
    let mut last_activity = file_modified_utc(path)?;

    for line in &head_lines {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        match value.get("type").and_then(Value::as_str) {
            Some("session_meta") => {
                let payload = value.get("payload").and_then(Value::as_object);
                if let Some(payload) = payload {
                    session_id = payload
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or(session_id);
                    cwd = payload
                        .get("cwd")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or(cwd);
                    originator = payload
                        .get("originator")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or(originator);
                }
                if let Some(timestamp) = parse_timestamp(&value) {
                    last_activity = last_activity.max(timestamp);
                }
            }
            Some("turn_context") => {
                let payload = value.get("payload").and_then(Value::as_object);
                if let Some(payload) = payload {
                    cwd = payload
                        .get("cwd")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or(cwd);
                    model = payload
                        .get("model")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .or(model);
                }
                if let Some(timestamp) = parse_timestamp(&value) {
                    last_activity = last_activity.max(timestamp);
                }
            }
            _ => {
                if let Some(timestamp) = parse_timestamp(&value) {
                    last_activity = last_activity.max(timestamp);
                }
            }
        }
    }

    let Some(session_id) = session_id else {
        return Ok((None, task_activity_state));
    };

    for line in &tail_lines {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        if let Some(timestamp) = parse_timestamp(&value) {
            last_activity = last_activity.max(timestamp);
        }
    }
    let task_activity = task_activity_state.tracker.clone().finish();

    let mut last_assistant_message = None;
    for line in tail_lines.iter().rev() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        if last_assistant_message.is_none() {
            last_assistant_message = extract_task_complete_message(&value)
                .or_else(|| extract_agent_message(&value))
                .or_else(|| extract_assistant_output(&value));
        }
    }

    Ok((
        Some(ParsedSessionFile {
            session_id,
            originator,
            cwd,
            model,
            last_activity,
            last_assistant_message,
            task_activity,
        }),
        task_activity_state,
    ))
}

fn build_session_record(
    parsed: &ParsedSessionFile,
    history: &HashMap<String, HistoryEntry>,
    app_thread: Option<&CodexAppThread>,
) -> SessionRecord {
    let mut last_activity = parsed.last_activity;
    let last_user_prompt = history
        .get(&parsed.session_id)
        .map(|entry| {
            last_activity = last_activity.max(entry.timestamp);
            entry.text.clone()
        })
        .or_else(|| {
            app_thread
                .and_then(|thread| thread.first_user_message.clone())
                .filter(|message| !message.trim().is_empty())
        });
    if let Some(thread) = app_thread {
        last_activity = last_activity.max(thread.updated_at);
    }
    let cwd = parsed
        .cwd
        .clone()
        .or_else(|| app_thread.and_then(|thread| thread.cwd.clone()));
    let model = parsed
        .model
        .clone()
        .or_else(|| app_thread.and_then(|thread| thread.model.clone()));

    let now = Utc::now();
    let status = match parsed.task_activity.open_task_started_at {
        Some(timestamp) if (now - timestamp).num_seconds() <= ACTIVE_WINDOW_SECS => {
            AgentStatus::Processing
        }
        _ => match parsed.task_activity.latest_signal {
            Some(TaskSignal {
                marker: TaskMarker::Started,
                timestamp,
            }) if (now - timestamp).num_seconds() <= ACTIVE_WINDOW_SECS => AgentStatus::Processing,
            Some(TaskSignal {
                marker: TaskMarker::Started,
                ..
            }) => AgentStatus::Idle,
            Some(TaskSignal {
                marker: TaskMarker::Complete,
                ..
            }) => AgentStatus::Idle,
            Some(TaskSignal {
                marker: TaskMarker::Aborted,
                ..
            }) => AgentStatus::Idle,
            None if (now - last_activity).num_seconds() <= ACTIVE_WINDOW_SECS => {
                AgentStatus::Processing
            }
            None => AgentStatus::Idle,
        },
    };
    debug!(
        session_id = %parsed.session_id,
        active_task_count = parsed.task_activity.active_task_count,
        latest_task_signal = ?parsed.task_activity.latest_signal,
        open_task_started_at = ?parsed.task_activity.open_task_started_at,
        last_activity = %last_activity,
        resolved_status = ?status,
        has_valid_last_assistant_message = parsed
            .last_assistant_message
            .as_deref()
            .is_some_and(|message| !message.trim().is_empty()),
        "resolved Codex session status"
    );

    SessionRecord {
        session_id: parsed.session_id.clone(),
        source: "codex".to_string(),
        project_name: cwd.as_ref().and_then(|value| project_name_from_cwd(value)),
        cwd,
        model,
        terminal_app: None,
        terminal_bundle: None,
        host_app: parsed_session_host_app(parsed, app_thread),
        window_title: app_thread.and_then(|thread| thread.title.clone()),
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
        last_user_prompt,
        last_assistant_message: parsed.last_assistant_message.clone(),
        tool_history: Vec::new(),
        last_activity,
    }
}

fn build_app_thread_record(thread: &CodexAppThread) -> SessionRecord {
    let now = Utc::now();
    let status = if (now - thread.updated_at).num_seconds() <= ACTIVE_WINDOW_SECS {
        AgentStatus::Processing
    } else {
        AgentStatus::Idle
    };

    SessionRecord {
        session_id: thread.session_id.clone(),
        source: "codex".to_string(),
        project_name: thread
            .cwd
            .as_ref()
            .and_then(|value| project_name_from_cwd(value)),
        cwd: thread.cwd.clone(),
        model: thread.model.clone(),
        terminal_app: None,
        terminal_bundle: None,
        host_app: codex_app_host_app(thread),
        window_title: thread.title.clone(),
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
        last_user_prompt: thread.first_user_message.clone(),
        last_assistant_message: None,
        tool_history: Vec::new(),
        last_activity: thread.updated_at,
    }
}

fn codex_app_host_app(thread: &CodexAppThread) -> Option<String> {
    let source = thread.source.as_deref().unwrap_or_default();
    let thread_source = thread.thread_source.as_deref().unwrap_or_default();
    let has_no_rollout_file = thread
        .rollout_path
        .as_ref()
        .is_none_or(|path| !path.exists());
    (!thread_source.trim().is_empty() || source.eq_ignore_ascii_case("app") || has_no_rollout_file)
        .then(|| "com.openai.codex".to_string())
}

fn parsed_session_host_app(
    parsed: &ParsedSessionFile,
    app_thread: Option<&CodexAppThread>,
) -> Option<String> {
    if parsed
        .originator
        .as_deref()
        .is_some_and(|originator| originator.eq_ignore_ascii_case("codex-tui"))
    {
        return None;
    }

    if parsed
        .originator
        .as_deref()
        .is_some_and(|originator| originator.eq_ignore_ascii_case("codex-app"))
    {
        return Some("com.openai.codex".to_string());
    }

    app_thread
        .and_then(codex_app_host_app)
        .or_else(|| app_thread.is_some().then(|| "com.openai.codex".to_string()))
}

fn refresh_codex_app_thread_state(
    root: &Path,
    limit: usize,
    state: &mut CodexAppThreadScanState,
) -> Result<Vec<CodexAppThread>> {
    let Some(path) = latest_codex_state_db(root)? else {
        state.db_path = None;
        state.modified_at = None;
        state.threads.clear();
        return Ok(Vec::new());
    };
    let modified_at = file_modified_utc(&path)?;
    if state.db_path.as_ref() == Some(&path) && state.modified_at == Some(modified_at) {
        return Ok(state.threads.clone());
    }

    let threads = load_codex_app_threads_from_db(&path, limit)?;
    state.db_path = Some(path);
    state.modified_at = Some(modified_at);
    state.threads = threads.clone();
    Ok(threads)
}

fn project_name_from_cwd(cwd: &str) -> Option<String> {
    let trimmed = cwd.trim_end_matches(['\\', '/']);
    trimmed
        .rsplit(['\\', '/'])
        .next()
        .map(ToOwned::to_owned)
        .filter(|value| !value.is_empty())
}

fn load_codex_app_threads_from_db(path: &Path, limit: usize) -> Result<Vec<CodexAppThread>> {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .with_context(|| format!("failed to open Codex app state DB {}", path.display()))?;
    let columns = load_threads_table_columns(&connection)?;
    let query = codex_app_threads_query(&columns);
    let mut statement = connection
        .prepare(&query)
        .context("failed to prepare Codex app thread query")?;
    let rows = statement
        .query_map(rusqlite::params![limit as i64], codex_app_thread_from_row)
        .context("failed to query Codex app threads")?;

    let mut threads = Vec::new();
    for row in rows {
        if let Some(thread) = row.context("failed to read Codex app thread row")? {
            threads.push(thread);
        }
    }
    Ok(threads)
}

fn latest_codex_state_db(root: &Path) -> Result<Option<PathBuf>> {
    if !root.exists() {
        return Ok(None);
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", root.display()))?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name.starts_with("state_") && file_name.ends_with(".sqlite") {
            paths.push(path);
        }
    }
    paths.sort_by_key(|path| std::cmp::Reverse(modified_key(path)));
    Ok(paths.into_iter().next())
}

fn load_threads_table_columns(connection: &rusqlite::Connection) -> Result<HashSet<String>> {
    let mut statement = connection
        .prepare("PRAGMA table_info(threads)")
        .context("failed to inspect Codex app threads table")?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>("name"))
        .context("failed to query Codex app threads table columns")?;

    let mut columns = HashSet::new();
    for row in rows {
        columns.insert(row.context("failed to read Codex app threads table column")?);
    }
    Ok(columns)
}

fn codex_app_threads_query(columns: &HashSet<String>) -> String {
    let archived_filter = if columns.contains("archived") {
        "WHERE archived = 0"
    } else {
        ""
    };
    let updated_ms = codex_app_updated_ms_expr(columns);

    format!(
        r#"
        SELECT
          {id},
          {rollout_path},
          {cwd},
          {title},
          {first_user_message},
          {model},
          {source},
          {thread_source},
          {updated_ms} AS updated_ms
        FROM threads
        {archived_filter}
        ORDER BY updated_ms DESC, id DESC
        LIMIT ?1
        "#,
        id = nullable_column_expr(columns, "id"),
        rollout_path = nullable_column_expr(columns, "rollout_path"),
        cwd = nullable_column_expr(columns, "cwd"),
        title = nullable_column_expr(columns, "title"),
        first_user_message = nullable_column_expr(columns, "first_user_message"),
        model = nullable_column_expr(columns, "model"),
        source = nullable_column_expr(columns, "source"),
        thread_source = nullable_column_expr(columns, "thread_source"),
    )
}

fn nullable_column_expr(columns: &HashSet<String>, name: &str) -> String {
    if columns.contains(name) {
        format!("{name} AS {name}")
    } else {
        format!("NULL AS {name}")
    }
}

fn codex_app_updated_ms_expr(columns: &HashSet<String>) -> String {
    let mut parts = Vec::new();
    if columns.contains("updated_at_ms") {
        parts.push("updated_at_ms".to_string());
    }
    if columns.contains("updated_at") {
        parts.push("updated_at * 1000".to_string());
    }
    if columns.contains("created_at_ms") {
        parts.push("created_at_ms".to_string());
    }
    if columns.contains("created_at") {
        parts.push("created_at * 1000".to_string());
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        format!("CAST(COALESCE({}) AS INTEGER)", parts.join(", "))
    }
}

fn codex_app_thread_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Option<CodexAppThread>> {
    let Some(updated_ms) = row.get::<_, Option<i64>>("updated_ms")? else {
        return Ok(None);
    };
    let Some(session_id) = optional_non_empty_string(row.get("id")?) else {
        return Ok(None);
    };

    Ok(Some(CodexAppThread {
        session_id,
        rollout_path: optional_non_empty_string(row.get("rollout_path")?).map(PathBuf::from),
        cwd: optional_non_empty_string(row.get("cwd")?),
        title: optional_non_empty_string(row.get("title")?),
        first_user_message: optional_non_empty_string(row.get("first_user_message")?),
        model: optional_non_empty_string(row.get("model")?),
        source: optional_non_empty_string(row.get("source")?),
        thread_source: optional_non_empty_string(row.get("thread_source")?),
        updated_at: DateTime::<Utc>::from_timestamp_millis(updated_ms).unwrap_or_else(Utc::now),
    }))
}

fn optional_non_empty_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn recent_session_files(root: &Path, limit: usize) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_session_files(root, &mut files)?;
    files.sort_by_key(|path| std::cmp::Reverse(modified_key(path)));
    files.truncate(limit);
    Ok(files)
}

fn collect_session_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to read file type for {}", path.display()))?;

        if file_type.is_dir() {
            collect_session_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
    Ok(())
}

fn modified_key(path: &Path) -> DateTime<Utc> {
    file_modified_utc(path).unwrap_or_else(|_| DateTime::<Utc>::from(std::time::UNIX_EPOCH))
}

fn file_size(path: &Path) -> Result<u64> {
    Ok(fs::metadata(path)
        .with_context(|| format!("failed to stat {}", path.display()))?
        .len())
}

fn load_history(path: &Path) -> Result<std::collections::HashMap<String, HistoryEntry>> {
    if !path.exists() {
        return Ok(std::collections::HashMap::new());
    }

    let reader = BufReader::new(
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?,
    );
    let mut history = std::collections::HashMap::new();

    for line in reader.lines() {
        let line = line.with_context(|| format!("failed to read {}", path.display()))?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        let Some(session_id) = value.get("session_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(text) = value.get("text").and_then(Value::as_str) else {
            continue;
        };
        let Some(timestamp) = value
            .get("ts")
            .and_then(Value::as_i64)
            .and_then(|value| DateTime::<Utc>::from_timestamp(value, 0))
        else {
            continue;
        };

        history.insert(
            session_id.to_string(),
            HistoryEntry {
                timestamp,
                text: text.to_string(),
            },
        );
    }

    Ok(history)
}

fn refresh_history_state(path: &Path, state: &mut HistoryScanState) -> Result<()> {
    if !path.exists() {
        *state = HistoryScanState::default();
        return Ok(());
    }

    let modified_at = Some(file_modified_utc(path)?);
    let size = file_size(path)?;
    let truncated = size < state.offset;
    let unchanged = !truncated && state.size == size && state.modified_at == modified_at;

    if unchanged {
        return Ok(());
    }

    if truncated || state.offset == 0 {
        state.latest_prompt_by_session = load_history(path)?;
        state.offset = size;
        state.size = size;
        state.modified_at = modified_at;
        return Ok(());
    }

    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    file.seek(SeekFrom::Start(state.offset))
        .with_context(|| format!("failed to seek {}", path.display()))?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.with_context(|| format!("failed to read {}", path.display()))?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        let Some(session_id) = value.get("session_id").and_then(Value::as_str) else {
            continue;
        };
        let Some(text) = value.get("text").and_then(Value::as_str) else {
            continue;
        };
        let Some(timestamp) = value
            .get("ts")
            .and_then(Value::as_i64)
            .and_then(|value| DateTime::<Utc>::from_timestamp(value, 0))
        else {
            continue;
        };

        state.latest_prompt_by_session.insert(
            session_id.to_string(),
            HistoryEntry {
                timestamp,
                text: text.to_string(),
            },
        );
    }

    state.offset = size;
    state.size = size;
    state.modified_at = modified_at;
    Ok(())
}

fn read_head_lines(path: &Path, limit: usize) -> Result<Vec<String>> {
    let reader = BufReader::new(
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?,
    );
    reader
        .lines()
        .take(limit)
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to read {}", path.display()))
}

fn read_tail_lines(path: &Path, max_bytes: u64) -> Result<Vec<String>> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let file_len = file
        .metadata()
        .with_context(|| format!("failed to stat {}", path.display()))?
        .len();
    let start = file_len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))
        .with_context(|| format!("failed to seek {}", path.display()))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let text = String::from_utf8_lossy(&buffer);
    let mut lines = text.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    Ok(lines)
}

fn scan_task_activity_incremental(
    path: &Path,
    previous: Option<&TaskActivityScanState>,
) -> Result<TaskActivityScanState> {
    let file_len = file_size(path)?;
    let previous = previous.filter(|state| state.offset <= file_len);
    let mut tracker = previous
        .map(|state| state.tracker.clone())
        .unwrap_or_default();
    let offset = previous.map(|state| state.offset).unwrap_or(0);
    let reader = BufReader::new(
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?,
    );
    let mut reader = reader;
    if offset > 0 {
        reader
            .seek(SeekFrom::Start(offset))
            .with_context(|| format!("failed to seek {}", path.display()))?;
    }

    for line in reader.lines() {
        let line = line.with_context(|| format!("failed to read {}", path.display()))?;
        if !line_may_contain_task_activity(&line) {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        tracker.observe(&value);
    }

    Ok(TaskActivityScanState {
        offset: file_len,
        tracker,
    })
}

fn line_may_contain_task_activity(line: &str) -> bool {
    line.contains("\"turn_id\"")
        || line.contains("\"task_started\"")
        || line.contains("\"task_complete\"")
        || line.contains("\"turn_aborted\"")
}

fn file_modified_utc(path: &Path) -> Result<DateTime<Utc>> {
    let modified = fs::metadata(path)
        .with_context(|| format!("failed to stat {}", path.display()))?
        .modified()
        .with_context(|| format!("failed to read modified time for {}", path.display()))?;
    Ok(DateTime::<Utc>::from(modified))
}

fn parse_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

impl TaskActivityTracker {
    fn observe(&mut self, value: &Value) {
        let timestamp = parse_timestamp(value);
        let turn_id = task_signal_turn_id(value).map(ToOwned::to_owned);

        if let (Some(turn_id), Some(timestamp)) = (turn_id.as_ref(), timestamp) {
            self.active_tasks.insert(turn_id.clone(), timestamp);
        }

        let Some(signal) = extract_task_signal(value) else {
            return;
        };
        self.latest_signal = Some(signal);

        match (signal.marker, turn_id) {
            (TaskMarker::Started, Some(turn_id)) => {
                self.active_tasks.insert(turn_id, signal.timestamp);
            }
            (TaskMarker::Complete | TaskMarker::Aborted, Some(turn_id)) => {
                self.active_tasks.remove(&turn_id);
            }
            (TaskMarker::Aborted, None) => {
                self.active_tasks.clear();
            }
            _ => {}
        }
    }

    fn finish(self) -> TaskActivity {
        TaskActivity {
            latest_signal: self.latest_signal,
            open_task_started_at: self.active_tasks.values().copied().max(),
            active_task_count: self.active_tasks.len(),
        }
    }
}

fn task_signal_payload(value: &Value) -> Option<&Value> {
    value
        .get("payload")
        .filter(|_| matches!(value.get("type").and_then(Value::as_str), Some("event_msg")))
}

fn task_signal_turn_id(value: &Value) -> Option<&str> {
    task_signal_payload(value)
        .and_then(|payload| payload.get("turn_id").and_then(Value::as_str))
        .or_else(|| value.get("turn_id").and_then(Value::as_str))
}

fn extract_agent_message(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if value.get("type").and_then(Value::as_str) != Some("event_msg") {
        return None;
    }
    if payload.get("type").and_then(Value::as_str) != Some("agent_message") {
        return None;
    }
    payload
        .get("message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_task_signal(value: &Value) -> Option<TaskSignal> {
    if value.get("type").and_then(Value::as_str) == Some("turn_aborted") {
        let timestamp = parse_timestamp(value)?;
        return Some(TaskSignal {
            marker: TaskMarker::Aborted,
            timestamp,
        });
    }

    let payload = task_signal_payload(value)?;

    let timestamp = parse_timestamp(value)?;

    match payload.get("type").and_then(Value::as_str) {
        Some("task_started") => Some(TaskSignal {
            marker: TaskMarker::Started,
            timestamp,
        }),
        Some("task_complete") => Some(TaskSignal {
            marker: TaskMarker::Complete,
            timestamp,
        }),
        Some("turn_aborted") => Some(TaskSignal {
            marker: TaskMarker::Aborted,
            timestamp,
        }),
        _ => None,
    }
}

fn extract_task_complete_message(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if value.get("type").and_then(Value::as_str) != Some("event_msg") {
        return None;
    }
    if payload.get("type").and_then(Value::as_str) != Some("task_complete") {
        return None;
    }

    payload
        .get("last_agent_message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_assistant_output(value: &Value) -> Option<String> {
    let payload = value.get("payload")?;
    if value.get("type").and_then(Value::as_str) != Some("response_item") {
        return None;
    }
    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    if payload.get("role").and_then(Value::as_str) != Some("assistant") {
        return None;
    }

    payload
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| {
            content.iter().find_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
}
