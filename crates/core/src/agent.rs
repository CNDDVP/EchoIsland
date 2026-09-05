use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::state::{AgentStatus, SessionRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSourceType {
    Cli,
    Ide,
    Extension,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSource {
    pub source_id: String,
    pub source_type: AgentSourceType,
    pub display_name: String,
    pub icon: Option<String>,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionStatus {
    Idle,
    Running,
    Waiting,
    Completed,
    Error,
}

impl From<AgentStatus> for AgentSessionStatus {
    fn from(value: AgentStatus) -> Self {
        match value {
            AgentStatus::Idle => Self::Idle,
            AgentStatus::Processing | AgentStatus::Running => Self::Running,
            AgentStatus::WaitingApproval | AgentStatus::WaitingQuestion => Self::Waiting,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSession {
    pub source_id: String,
    pub session_id: String,
    pub workspace_path: Option<String>,
    pub title: String,
    pub status: AgentSessionStatus,
    pub pid: Option<u32>,
    pub window_id: Option<String>,
    pub terminal_tab: Option<String>,
    pub ide_bundle_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub metadata: Value,
}

impl AgentSession {
    pub fn from_record(record: &SessionRecord) -> Self {
        let terminal_tab = record
            .iterm_session_id
            .clone()
            .or_else(|| record.kitty_window_id.clone())
            .or_else(|| record.tmux_pane.clone())
            .or_else(|| record.tty.clone());

        Self {
            source_id: record.source.clone(),
            session_id: record.session_id.clone(),
            workspace_path: record.cwd.clone(),
            title: session_title(record),
            status: record.status.into(),
            pid: record.cli_pid.or(record.terminal_pid),
            window_id: record.window_title.clone(),
            terminal_tab,
            ide_bundle_id: record.host_app.clone(),
            started_at: None,
            last_activity_at: Some(record.last_activity),
            metadata: json!({
                "project_name": record.project_name.clone(),
                "cwd": record.cwd.clone(),
                "model": record.model.clone(),
                "terminal_app": record.terminal_app.clone(),
                "terminal_bundle": record.terminal_bundle.clone(),
                "host_app": record.host_app.clone(),
                "window_title": record.window_title.clone(),
                "tty": record.tty.clone(),
                "terminal_pid": record.terminal_pid,
                "cli_pid": record.cli_pid,
                "current_tool": record.current_tool.clone(),
                "tool_description": record.tool_description.clone(),
                "last_user_prompt": record.last_user_prompt.clone(),
                "last_assistant_message": record.last_assistant_message.clone(),
            }),
        }
    }
}

pub fn agent_sessions_from_records<'a>(
    records: impl IntoIterator<Item = &'a SessionRecord>,
) -> Vec<AgentSession> {
    let mut sessions = records
        .into_iter()
        .map(AgentSession::from_record)
        .collect::<Vec<_>>();
    sessions.sort_by(|a, b| {
        agent_session_sort_key(a)
            .cmp(&agent_session_sort_key(b))
            .then_with(|| b.last_activity_at.cmp(&a.last_activity_at))
            .then_with(|| a.source_id.cmp(&b.source_id))
            .then_with(|| a.session_id.cmp(&b.session_id))
    });
    sessions
}

fn agent_session_sort_key(session: &AgentSession) -> u8 {
    match session.status {
        AgentSessionStatus::Waiting => 0,
        AgentSessionStatus::Running => 1,
        AgentSessionStatus::Completed => 2,
        AgentSessionStatus::Error => 3,
        AgentSessionStatus::Idle => 4,
    }
}

fn session_title(record: &SessionRecord) -> String {
    record
        .project_name
        .clone()
        .or_else(|| {
            record
                .last_user_prompt
                .as_ref()
                .map(|prompt| compact_title(prompt))
        })
        .or_else(|| record.cwd.as_ref().and_then(|cwd| path_name(cwd)))
        .unwrap_or_else(|| record.source.clone())
}

fn compact_title(value: &str) -> String {
    let trimmed = value.trim();
    let mut title = echoisland_i18n::text::truncate_graphemes(trimmed, 80).to_string();
    if title.is_empty() {
        title = echoisland_i18n::t("session.untitled").to_string();
    }
    title
}

fn path_name(value: &str) -> Option<String> {
    value
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{AgentSessionStatus, AgentStatus, SessionRecord};

    use super::{AgentSession, agent_sessions_from_records};

    fn session(id: &str, source: &str, status: AgentStatus) -> SessionRecord {
        SessionRecord {
            session_id: id.to_string(),
            source: source.to_string(),
            cwd: Some("/tmp/demo".to_string()),
            model: Some("gpt-5".to_string()),
            project_name: Some("demo".to_string()),
            terminal_app: Some("iTerm2".to_string()),
            terminal_bundle: Some("com.googlecode.iterm2".to_string()),
            host_app: None,
            window_title: Some("demo - codex".to_string()),
            tty: Some("/dev/ttys001".to_string()),
            terminal_pid: Some(100),
            cli_pid: Some(200),
            iterm_session_id: Some("iterm-1".to_string()),
            kitty_window_id: None,
            tmux_env: None,
            tmux_pane: None,
            tmux_client_tty: None,
            status,
            current_tool: None,
            tool_description: None,
            last_user_prompt: Some("implement feature".to_string()),
            last_assistant_message: None,
            tool_history: Vec::new(),
            last_activity: Utc::now(),
        }
    }

    #[test]
    fn agent_session_maps_existing_session_record() {
        let record = session("s1", "codex", AgentStatus::WaitingApproval);
        let session = AgentSession::from_record(&record);

        assert_eq!(session.source_id, "codex");
        assert_eq!(session.workspace_path.as_deref(), Some("/tmp/demo"));
        assert_eq!(session.title, "demo");
        assert_eq!(session.status, AgentSessionStatus::Waiting);
        assert_eq!(session.pid, Some(200));
        assert_eq!(session.terminal_tab.as_deref(), Some("iterm-1"));
        assert_eq!(session.metadata["model"], "gpt-5");
    }

    #[test]
    fn agent_sessions_sort_waiting_before_running_before_idle() {
        let idle = session("idle", "codex", AgentStatus::Idle);
        let running = session("running", "gemini", AgentStatus::Running);
        let waiting = session("waiting", "claude", AgentStatus::WaitingQuestion);

        let sessions = agent_sessions_from_records([&idle, &running, &waiting]);

        assert_eq!(sessions[0].session_id, "waiting");
        assert_eq!(sessions[1].session_id, "running");
        assert_eq!(sessions[2].session_id, "idle");
    }

    #[test]
    fn unicode_grapheme_prompt_titles_keep_the_80_character_budget_and_original_message() {
        for cluster in ["中", "A", "👩‍💻", "👍🏽", "🇨🇳", "e\u{301}"] {
            let prefix = "a".repeat(79);
            let prompt = format!("  {prefix}{cluster}Z  ");
            let mut record = session("unicode-title", "codex", AgentStatus::Processing);
            record.project_name = None;
            record.last_user_prompt = Some(prompt.clone());

            let projected = AgentSession::from_record(&record);

            // Agent API titles retain their existing no-ellipsis style, unlike native previews.
            assert_eq!(projected.title, format!("{prefix}{cluster}"));
            assert_eq!(projected.metadata["last_user_prompt"], prompt);
        }
    }

    #[test]
    fn unicode_grapheme_empty_prompt_title_keeps_the_localized_fallback() {
        let mut record = session("empty-title", "codex", AgentStatus::Idle);
        record.project_name = None;
        record.last_user_prompt = Some(" \r\n\t ".to_string());

        assert_eq!(AgentSession::from_record(&record).title, "未命名会话");
    }
}
