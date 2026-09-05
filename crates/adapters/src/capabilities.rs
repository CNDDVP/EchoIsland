use serde::Serialize;

/// Supported observations/actions, independent of installation or health.
/// Process discovery alone must never advertise task or approval integration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct AdapterCapabilities {
    pub process_detection: bool,
    pub session_scan: bool,
    pub realtime_hook: bool,
    pub approval: bool,
    pub question: bool,
    pub completion: bool,
    pub terminal_focus: bool,
    pub history: bool,
    pub sub_agent: bool,
    pub tool_call: bool,
}

pub fn capabilities_for_source(source_id: &str) -> AdapterCapabilities {
    match source_id {
        "codex" | "codex-cli" => AdapterCapabilities {
            session_scan: true,
            realtime_hook: true,
            completion: true,
            history: true,
            tool_call: true,
            ..Default::default()
        },
        "claude" | "claude-code" => AdapterCapabilities {
            session_scan: true,
            realtime_hook: true,
            approval: true,
            question: true,
            completion: true,
            history: true,
            tool_call: true,
            ..Default::default()
        },
        "openclaw" => AdapterCapabilities {
            realtime_hook: true,
            approval: true,
            question: true,
            completion: true,
            tool_call: true,
            ..Default::default()
        },
        "gemini" | "gemini-cli" | "glm" | "glm-cli" | "vscode" | "cursor" | "trae" => {
            AdapterCapabilities {
                process_detection: true,
                ..Default::default()
            }
        }
        _ => AdapterCapabilities::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_does_not_claim_hooks_or_task_state() {
        for source in ["gemini", "glm", "vscode", "cursor", "trae"] {
            let capability = capabilities_for_source(source);
            assert!(capability.process_detection);
            assert!(!capability.session_scan);
            assert!(!capability.realtime_hook);
            assert!(!capability.approval);
            assert!(!capability.completion);
        }
        assert_eq!(
            capabilities_for_source("unknown"),
            AdapterCapabilities::default()
        );
        assert_eq!(
            capabilities_for_source("codex"),
            capabilities_for_source("codex-cli")
        );
        // Codex hooks observe events; approvals still happen in Codex itself.
        assert!(!capabilities_for_source("codex").approval);
        assert!(capabilities_for_source("claude").question);
    }
}
