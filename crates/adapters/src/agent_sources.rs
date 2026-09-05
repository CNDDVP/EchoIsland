use anyhow::{Result, bail};
use echoisland_core::{
    AgentSession, AgentSource, AgentSourceType, SessionRecord, agent_sessions_from_records,
};

use crate::{AdapterCapabilities, SessionScanningAdapter, capabilities_for_source};

pub trait AgentSourceAdapter: Send {
    fn source(&self) -> AgentSource;
    fn detect_sessions(&mut self) -> Result<Vec<AgentSession>>;

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            session_scan: true,
            ..Default::default()
        }
    }

    fn focus_session(&self, session: &AgentSession) -> Result<()> {
        bail!(echoisland_i18n::format(
            "adapter.focus_unavailable",
            &[("source", session.source_id.as_str())]
        ))
    }
}

#[derive(Default)]
pub struct AgentSourceRegistry {
    adapters: Vec<Box<dyn AgentSourceAdapter>>,
}

impl AgentSourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<A>(&mut self, adapter: A)
    where
        A: AgentSourceAdapter + 'static,
    {
        self.adapters.push(Box::new(adapter));
        self.adapters.sort_by(|a, b| {
            b.source()
                .priority
                .cmp(&a.source().priority)
                .then_with(|| a.source().source_id.cmp(&b.source().source_id))
        });
    }

    pub fn sources(&self) -> Vec<AgentSource> {
        self.adapters
            .iter()
            .map(|adapter| adapter.source())
            .collect()
    }

    pub fn capabilities(&self) -> Vec<(String, AdapterCapabilities)> {
        self.adapters
            .iter()
            .map(|adapter| (adapter.source().source_id, adapter.capabilities()))
            .collect()
    }

    pub fn detect_sessions(&mut self) -> Vec<AgentSession> {
        let mut sessions = Vec::new();

        for adapter in &mut self.adapters {
            let source = adapter.source();
            if !source.enabled {
                continue;
            }

            match adapter.detect_sessions() {
                Ok(mut detected) => sessions.append(&mut detected),
                Err(error) => {
                    tracing::warn!(
                        source = source.source_id,
                        error = %error,
                        "failed to detect agent source sessions"
                    );
                }
            }
        }

        sessions.sort_by(|a, b| {
            agent_session_status_rank(a)
                .cmp(&agent_session_status_rank(b))
                .then_with(|| b.last_activity_at.cmp(&a.last_activity_at))
                .then_with(|| a.source_id.cmp(&b.source_id))
                .then_with(|| a.session_id.cmp(&b.session_id))
        });
        sessions
    }
}

pub struct ScanningAgentSourceAdapter<A> {
    source: AgentSource,
    adapter: A,
}

impl<A> ScanningAgentSourceAdapter<A> {
    pub fn new(source: AgentSource, adapter: A) -> Self {
        Self { source, adapter }
    }
}

impl<A> AgentSourceAdapter for ScanningAgentSourceAdapter<A>
where
    A: SessionScanningAdapter + Send,
{
    fn source(&self) -> AgentSource {
        self.source.clone()
    }

    fn detect_sessions(&mut self) -> Result<Vec<AgentSession>> {
        let records = self.adapter.scan_sessions()?;
        Ok(records_to_agent_sessions(records))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        let mut capabilities = capabilities_for_source(self.adapter.adapter_id());
        capabilities.session_scan = true;
        capabilities
    }
}

pub fn records_to_agent_sessions(records: Vec<SessionRecord>) -> Vec<AgentSession> {
    agent_sessions_from_records(records.iter())
}

pub fn built_in_source(source_id: &str) -> Option<AgentSource> {
    let (source_type, display_name, icon, priority) = match source_id {
        "codex" | "codex-cli" => (AgentSourceType::Cli, "Codex", Some("codex"), 100),
        "claude" | "claude-code" => (AgentSourceType::Cli, "Claude Code", Some("claude"), 90),
        "openclaw" => (AgentSourceType::Cli, "OpenClaw", Some("openclaw"), 85),
        "gemini" | "gemini-cli" => (AgentSourceType::Cli, "Gemini CLI", Some("gemini"), 80),
        "glm" | "glm-cli" => (AgentSourceType::Cli, "GLM CLI", Some("glm"), 70),
        "vscode" => (AgentSourceType::Ide, "VS Code", Some("vscode"), 50),
        "cursor" => (AgentSourceType::Ide, "Cursor", Some("cursor"), 50),
        "trae" => (AgentSourceType::Ide, "Trae", Some("trae"), 50),
        _ => return None,
    };

    Some(AgentSource {
        source_id: source_id.to_string(),
        source_type,
        display_name: display_name.to_string(),
        icon: icon.map(ToOwned::to_owned),
        priority,
        enabled: true,
    })
}

fn agent_session_status_rank(session: &AgentSession) -> u8 {
    match session.status {
        echoisland_core::AgentSessionStatus::Waiting => 0,
        echoisland_core::AgentSessionStatus::Running => 1,
        echoisland_core::AgentSessionStatus::Completed => 2,
        echoisland_core::AgentSessionStatus::Error => 3,
        echoisland_core::AgentSessionStatus::Idle => 4,
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chrono::Utc;
    use echoisland_core::{AgentStatus, SessionRecord};

    use crate::SessionScanningAdapter;

    use super::{
        AgentSourceRegistry, ScanningAgentSourceAdapter, built_in_source, records_to_agent_sessions,
    };

    #[derive(Clone)]
    struct StaticScanner {
        id: &'static str,
        sessions: Vec<SessionRecord>,
    }

    impl SessionScanningAdapter for StaticScanner {
        fn adapter_id(&self) -> &'static str {
            self.id
        }

        fn scan_sessions(&self) -> Result<Vec<SessionRecord>> {
            Ok(self.sessions.clone())
        }
    }

    fn session(id: &str, source: &str, status: AgentStatus) -> SessionRecord {
        SessionRecord {
            session_id: id.to_string(),
            source: source.to_string(),
            cwd: Some(format!("/tmp/{source}")),
            model: None,
            project_name: None,
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
            status,
            current_tool: None,
            tool_description: None,
            last_user_prompt: None,
            last_assistant_message: None,
            tool_history: Vec::new(),
            last_activity: Utc::now(),
        }
    }

    #[test]
    fn records_convert_to_unified_agent_sessions() {
        let sessions =
            records_to_agent_sessions(vec![session("s1", "codex", AgentStatus::Running)]);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].source_id, "codex");
        assert_eq!(sessions[0].title, "codex");
    }

    #[test]
    fn registry_detects_sessions_from_registered_sources() {
        let mut registry = AgentSourceRegistry::new();
        registry.register(ScanningAgentSourceAdapter::new(
            built_in_source("codex").unwrap(),
            StaticScanner {
                id: "codex",
                sessions: vec![session("codex-1", "codex", AgentStatus::Running)],
            },
        ));
        registry.register(ScanningAgentSourceAdapter::new(
            built_in_source("claude").unwrap(),
            StaticScanner {
                id: "claude",
                sessions: vec![session("claude-1", "claude", AgentStatus::WaitingQuestion)],
            },
        ));

        let sessions = registry.detect_sessions();

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, "claude-1");
        assert_eq!(sessions[1].session_id, "codex-1");
    }

    #[test]
    fn built_in_source_catalog_includes_first_wave_sources() {
        assert_eq!(
            built_in_source("gemini-cli").unwrap().display_name,
            "Gemini CLI"
        );
        assert_eq!(
            built_in_source("openclaw").unwrap().display_name,
            "OpenClaw"
        );
        assert_eq!(built_in_source("trae").unwrap().display_name, "Trae");
        assert!(built_in_source("unknown").is_none());
    }
}
