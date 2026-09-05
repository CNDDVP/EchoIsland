use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use echoisland_core::SessionRecord;
use echoisland_paths::current_platform_paths;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
struct PersistedState {
    sessions: Vec<SessionRecord>,
}

pub fn default_state_path() -> PathBuf {
    current_platform_paths().state_path
}

pub fn save_sessions(path: &Path, sessions: &[SessionRecord]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let payload = PersistedState {
        sessions: sessions.to_vec(),
    };
    let encoded = serde_json::to_vec_pretty(&payload).context("failed to encode state")?;
    write_atomic(path, &encoded)?;
    Ok(())
}

pub fn load_sessions(path: &Path) -> Result<Vec<SessionRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let decoded: PersistedState =
        serde_json::from_slice(&raw).context("failed to decode persisted state")?;
    Ok(decoded.sessions)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("state.json");
    let temp_path = path.with_file_name(format!("{file_name}.tmp"));

    let mut temp_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp_path)
        .with_context(|| format!("failed to open temp file {}", temp_path.display()))?;
    temp_file
        .write_all(bytes)
        .with_context(|| format!("failed to write temp file {}", temp_path.display()))?;
    temp_file
        .sync_all()
        .with_context(|| format!("failed to flush temp file {}", temp_path.display()))?;
    drop(temp_file);

    if let Err(error) = replace_temp_file(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error).with_context(|| {
            format!(
                "failed to atomically move {} to {}",
                temp_path.display(),
                path.display()
            )
        });
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_temp_file(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(temp_path, path)
}

#[cfg(windows)]
fn replace_temp_file(temp_path: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use chrono::Utc;

    use echoisland_core::{AgentStatus, SessionRecord};

    use super::{load_sessions, save_sessions};

    fn temp_file() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("echoisland-state-{suffix}.json"))
    }

    #[test]
    fn saves_and_loads_sessions() {
        let path = temp_file();
        let sessions = vec![SessionRecord {
            session_id: "s1".into(),
            source: "codex".into(),
            cwd: Some("D:/repo".into()),
            model: Some("gpt-5.4".into()),
            project_name: Some("repo".into()),
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
            status: AgentStatus::Processing,
            current_tool: None,
            tool_description: None,
            last_user_prompt: Some("hello".into()),
            last_assistant_message: None,
            tool_history: Vec::new(),
            last_activity: Utc::now(),
        }];

        save_sessions(&path, &sessions).unwrap();
        let loaded = load_sessions(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].session_id, "s1");

        let _ = std::fs::remove_file(path);
    }

    #[cfg(windows)]
    #[test]
    fn failed_windows_replace_keeps_the_previous_state_readable() {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

        let path = temp_file();
        let first = SessionRecord {
            session_id: "previous".into(),
            source: "codex".into(),
            cwd: None,
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
            status: AgentStatus::Idle,
            current_tool: None,
            tool_description: None,
            last_user_prompt: None,
            last_assistant_message: None,
            tool_history: Vec::new(),
            last_activity: Utc::now(),
        };
        save_sessions(&path, std::slice::from_ref(&first)).unwrap();
        let lock = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&path)
            .unwrap();
        let mut replacement = first.clone();
        replacement.session_id = "replacement".into();

        assert!(save_sessions(&path, &[replacement]).is_err());
        assert_eq!(load_sessions(&path).unwrap()[0].session_id, "previous");

        drop(lock);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("json.tmp"));
    }
}
