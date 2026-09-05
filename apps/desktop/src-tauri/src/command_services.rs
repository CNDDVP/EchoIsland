use std::path::PathBuf;

use echoisland_core::{EventEnvelope, ResponseEnvelope};
use echoisland_runtime::RuntimeSnapshot;
use tokio::fs;

use crate::{app_runtime::AppRuntime, terminal_focus_service::TerminalFocusService};

const SAMPLE_SESSION_IDS: [&str; 8] = [
    "codex-session-001",
    "claude-session-001",
    "claude-session-002",
    "claude-session-003",
    "claude-session-004",
    "claude-session-005",
    "claude-session-006",
    "claude-session-007",
];

pub struct SnapshotCommandService<'a> {
    runtime: &'a AppRuntime,
}

impl<'a> SnapshotCommandService<'a> {
    pub fn new(runtime: &'a AppRuntime) -> Self {
        Self { runtime }
    }

    pub async fn get_snapshot(&self) -> Result<RuntimeSnapshot, String> {
        self.runtime
            .runtime
            .remove_sessions(&SAMPLE_SESSION_IDS)
            .await;
        let snapshot = self.runtime.runtime.snapshot().await;
        if snapshot.active_session_count > 0 {
            TerminalFocusService::new(self.runtime)
                .sync_snapshot_focus_bindings(&snapshot)
                .await?;
        }
        Ok(snapshot)
    }
}

pub struct SampleIngestService<'a> {
    runtime: &'a AppRuntime,
}

impl<'a> SampleIngestService<'a> {
    pub fn new(runtime: &'a AppRuntime) -> Self {
        Self { runtime }
    }

    pub async fn ingest_sample(&self, file_name: String) -> Result<ResponseEnvelope, String> {
        let path = sample_event_fixture_path(&file_name)?;
        let raw = fs::read(&path)
            .await
            .map_err(|error| echoisland_i18n::error("error.read_sample", error))?;
        let event = serde_json::from_slice::<EventEnvelope>(&raw)
            .map_err(|error| echoisland_i18n::error("error.parse_sample", error))?;
        event
            .validate()
            .map_err(|error| echoisland_i18n::error("error.invalid_sample", error))?;
        let normalized = event.normalized_event_name();
        if normalized == "PermissionRequest" || normalized == "AskUserQuestion" {
            let shared_runtime = self.runtime.runtime.clone();
            tauri::async_runtime::spawn(async move {
                let _ = shared_runtime.handle_event(event).await;
            });
            Ok(ResponseEnvelope::ok())
        } else {
            Ok(self.runtime.runtime.handle_event(event).await)
        }
    }
}

fn sample_event_fixture_path(file_name: &str) -> Result<PathBuf, String> {
    #[cfg(not(debug_assertions))]
    {
        let _ = file_name;
        return Err(echoisland_i18n::t("error.sample_debug_only").to_string());
    }

    #[cfg(debug_assertions)]
    {
        let trimmed = file_name.trim();
        if trimmed.is_empty() {
            return Err(echoisland_i18n::t("error.sample_name_empty").to_string());
        }
        let path = std::path::Path::new(trimmed);
        if path.components().count() != 1
            || path.file_name().and_then(|value| value.to_str()) != Some(trimmed)
            || path.extension().and_then(|value| value.to_str()) != Some("json")
        {
            return Err(echoisland_i18n::format(
                "error.sample_name_invalid",
                &[("name", file_name)],
            ));
        }

        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("crates")
            .join("core")
            .join("tests")
            .join("fixtures")
            .join("events")
            .join(trimmed))
    }
}

#[cfg(test)]
mod tests {
    use super::sample_event_fixture_path;

    #[test]
    fn sample_event_fixture_path_rejects_traversal_segments() {
        assert!(sample_event_fixture_path("../secret.json").is_err());
        assert!(sample_event_fixture_path("..\\secret.json").is_err());
        assert!(sample_event_fixture_path("nested/event.json").is_err());
    }

    #[test]
    fn sample_event_fixture_path_points_to_tracked_core_fixture() {
        let path = sample_event_fixture_path("codex_session_start.json").unwrap();

        assert!(
            path.ends_with(
                std::path::Path::new("crates")
                    .join("core")
                    .join("tests")
                    .join("fixtures")
                    .join("events")
                    .join("codex_session_start.json")
            )
        );
        assert!(path.exists());
    }
}
