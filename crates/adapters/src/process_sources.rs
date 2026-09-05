use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use echoisland_core::{AgentStatus, SessionRecord};

const ACTIVE_SCAN_INTERVAL_SECS: u64 = 5;
const IDLE_SCAN_INTERVAL_SECS: u64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSourceKind {
    Cli,
    Ide,
}

#[derive(Debug, Clone)]
pub struct ProcessSourceSpec {
    pub source: &'static str,
    pub display_name: &'static str,
    pub kind: ProcessSourceKind,
    pub exact_process_names: &'static [&'static str],
    pub command_substrings: &'static [&'static str],
    pub host_app: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ProcessSourceScanner {
    spec: ProcessSourceSpec,
    first_seen_at: HashMap<u32, DateTime<Utc>>,
    last_sessions: Vec<SessionRecord>,
    has_scanned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessInfo {
    pid: u32,
    executable: String,
    command: String,
    cwd: Option<String>,
}

impl ProcessSourceScanner {
    pub fn new(spec: ProcessSourceSpec) -> Self {
        Self {
            spec,
            first_seen_at: HashMap::new(),
            last_sessions: Vec::new(),
            has_scanned: false,
        }
    }

    pub fn spec(&self) -> &ProcessSourceSpec {
        &self.spec
    }

    pub fn scan(&mut self) -> Result<Option<Vec<SessionRecord>>> {
        let now = Utc::now();
        let processes = running_processes();
        let mut seen_pids = HashSet::new();
        let mut sessions = Vec::new();

        for mut process in processes
            .into_iter()
            .filter(|process| process_matches_spec(process, &self.spec))
        {
            attach_process_cwd(&mut process);
            seen_pids.insert(process.pid);
            let first_seen = *self.first_seen_at.entry(process.pid).or_insert(now);
            sessions.push(record_from_process(&self.spec, process, first_seen));
        }

        self.first_seen_at.retain(|pid, _| seen_pids.contains(pid));
        sessions.sort_by(|a, b| a.session_id.cmp(&b.session_id));

        if self.has_scanned && sessions == self.last_sessions {
            return Ok(None);
        }

        self.has_scanned = true;
        self.last_sessions = sessions.clone();
        Ok(Some(sessions))
    }

    pub fn recommended_poll_interval(&self) -> Duration {
        if self.last_sessions.is_empty() {
            Duration::from_secs(IDLE_SCAN_INTERVAL_SECS)
        } else {
            Duration::from_secs(ACTIVE_SCAN_INTERVAL_SECS)
        }
    }
}

pub fn gemini_cli_process_spec() -> ProcessSourceSpec {
    ProcessSourceSpec {
        source: "gemini",
        display_name: "Gemini CLI",
        kind: ProcessSourceKind::Cli,
        exact_process_names: &["gemini", "gemini.exe", "gemini.cmd"],
        command_substrings: &[
            "@google/gemini-cli",
            "google-gemini/gemini-cli",
            "/gemini-cli/",
            "\\gemini-cli\\",
        ],
        host_app: Some("cli"),
    }
}

pub fn glm_cli_process_spec() -> ProcessSourceSpec {
    ProcessSourceSpec {
        source: "glm",
        display_name: "GLM CLI",
        kind: ProcessSourceKind::Cli,
        exact_process_names: &["glm", "glm.exe", "glm-cli", "glm-cli.exe", "zai", "zai-cli"],
        command_substrings: &["glm-cli", "zai-glm-cli", "zai-cli", "@zai/", "@z-ai/"],
        host_app: Some("cli"),
    }
}

pub fn vscode_process_spec() -> ProcessSourceSpec {
    ProcessSourceSpec {
        source: "vscode",
        display_name: "VS Code",
        kind: ProcessSourceKind::Ide,
        exact_process_names: &["Code", "Code.exe"],
        command_substrings: &[
            "Visual Studio Code.app/Contents/MacOS",
            "com.microsoft.VSCode",
        ],
        host_app: Some("com.microsoft.VSCode"),
    }
}

pub fn cursor_process_spec() -> ProcessSourceSpec {
    ProcessSourceSpec {
        source: "cursor",
        display_name: "Cursor",
        kind: ProcessSourceKind::Ide,
        exact_process_names: &["Cursor", "Cursor.exe"],
        command_substrings: &["Cursor.app/Contents/MacOS", "com.todesktop.230313mzl4w4u92"],
        host_app: Some("com.todesktop.230313mzl4w4u92"),
    }
}

pub fn trae_process_spec() -> ProcessSourceSpec {
    ProcessSourceSpec {
        source: "trae",
        display_name: "Trae",
        kind: ProcessSourceKind::Ide,
        exact_process_names: &["Trae", "Trae.exe"],
        command_substrings: &["Trae.app/Contents/MacOS", "com.trae.app"],
        host_app: Some("com.trae.app"),
    }
}

fn record_from_process(
    spec: &ProcessSourceSpec,
    process: ProcessInfo,
    first_seen: DateTime<Utc>,
) -> SessionRecord {
    let project_name = process.cwd.as_deref().and_then(path_name);
    let status = match spec.kind {
        ProcessSourceKind::Cli => AgentStatus::Running,
        ProcessSourceKind::Ide => AgentStatus::Idle,
    };

    SessionRecord {
        session_id: format!("{}:{}", spec.source, process.pid),
        source: spec.source.to_string(),
        cwd: process.cwd,
        model: None,
        project_name,
        terminal_app: None,
        terminal_bundle: None,
        host_app: spec.host_app.map(ToOwned::to_owned),
        window_title: Some(spec.display_name.to_string()),
        tty: None,
        terminal_pid: None,
        cli_pid: (spec.kind == ProcessSourceKind::Cli).then_some(process.pid),
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
        last_activity: first_seen,
    }
}

fn process_matches_spec(process: &ProcessInfo, spec: &ProcessSourceSpec) -> bool {
    let name = process_name(&process.executable).to_ascii_lowercase();
    if spec
        .exact_process_names
        .iter()
        .any(|candidate| name == candidate.to_ascii_lowercase())
    {
        return true;
    }

    let command = process.command.to_ascii_lowercase();
    spec.command_substrings
        .iter()
        .any(|needle| command.contains(&needle.to_ascii_lowercase()))
}

fn process_name(value: &str) -> String {
    value
        .trim_matches('"')
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .to_string()
}

fn path_name(value: &str) -> Option<String> {
    value
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
}

fn running_processes() -> Vec<ProcessInfo> {
    #[cfg(windows)]
    {
        windows_processes()
    }

    #[cfg(unix)]
    {
        unix_processes()
    }

    #[cfg(all(not(windows), not(unix)))]
    {
        Vec::new()
    }
}

#[cfg(unix)]
fn unix_processes() -> Vec<ProcessInfo> {
    let Ok(output) = std::process::Command::new("/bin/ps")
        .args(["-axo", "pid=,comm=,args="])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_unix_ps_line)
        .collect()
}

#[cfg(unix)]
fn parse_unix_ps_line(line: &str) -> Option<ProcessInfo> {
    let mut parts = line.trim_start().splitn(3, char::is_whitespace);
    let pid = parts.next()?.trim().parse::<u32>().ok()?;
    let executable = parts.next()?.trim().to_string();
    let command = parts.next().unwrap_or_default().trim().to_string();
    Some(ProcessInfo {
        pid,
        executable,
        command,
        cwd: None,
    })
}

#[cfg(unix)]
fn attach_process_cwd(process: &mut ProcessInfo) {
    process.cwd = process_cwd(process.pid);
}

#[cfg(not(unix))]
fn attach_process_cwd(_process: &mut ProcessInfo) {}

#[cfg(unix)]
fn process_cwd(pid: u32) -> Option<String> {
    let output = std::process::Command::new("/usr/sbin/lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n').map(str::trim))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(windows)]
fn windows_processes() -> Vec<ProcessInfo> {
    use std::mem::size_of;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }

        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snapshot, &mut entry) == 0 {
            CloseHandle(snapshot);
            return Vec::new();
        }

        let mut processes = Vec::new();
        loop {
            let executable = process_entry_name(&entry);
            processes.push(ProcessInfo {
                pid: entry.th32ProcessID,
                executable: executable.clone(),
                command: executable,
                cwd: None,
            });

            if Process32NextW(snapshot, &mut entry) == 0 {
                break;
            }
        }

        CloseHandle(snapshot);
        processes
    }
}

#[cfg(windows)]
fn process_entry_name(
    entry: &windows_sys::Win32::System::Diagnostics::ToolHelp::PROCESSENTRY32W,
) -> String {
    let len = entry
        .szExeFile
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(entry.szExeFile.len());
    String::from_utf16_lossy(&entry.szExeFile[..len])
}

#[cfg(test)]
mod tests {
    use super::{
        ProcessInfo, cursor_process_spec, gemini_cli_process_spec, glm_cli_process_spec, path_name,
        process_matches_spec, process_name, trae_process_spec, vscode_process_spec,
    };

    fn process(executable: &str, command: &str) -> ProcessInfo {
        ProcessInfo {
            pid: 42,
            executable: executable.to_string(),
            command: command.to_string(),
            cwd: Some("/tmp/demo".to_string()),
        }
    }

    #[test]
    fn matches_gemini_cli_binary_and_npm_package() {
        assert!(process_matches_spec(
            &process("/usr/local/bin/gemini", "gemini"),
            &gemini_cli_process_spec()
        ));
        assert!(process_matches_spec(
            &process(
                "/usr/local/bin/node",
                "node /x/@google/gemini-cli/dist/index.js"
            ),
            &gemini_cli_process_spec()
        ));
    }

    #[test]
    fn matches_glm_common_cli_names() {
        assert!(process_matches_spec(
            &process("/usr/local/bin/glm", "glm"),
            &glm_cli_process_spec()
        ));
        assert!(process_matches_spec(
            &process("/usr/local/bin/node", "node /x/zai-glm-cli/bin.js"),
            &glm_cli_process_spec()
        ));
    }

    #[test]
    fn matches_first_wave_ide_process_names() {
        assert!(process_matches_spec(
            &process(
                "/Applications/Visual Studio Code.app/Contents/MacOS/Code",
                "Code"
            ),
            &vscode_process_spec()
        ));
        assert!(process_matches_spec(
            &process("Cursor.exe", "Cursor.exe"),
            &cursor_process_spec()
        ));
        assert!(process_matches_spec(
            &process("Trae", "Trae"),
            &trae_process_spec()
        ));
    }

    #[test]
    fn extracts_leaf_names() {
        assert_eq!(process_name(r#"C:\Tools\gemini.exe"#), "gemini.exe");
        assert_eq!(path_name("/tmp/demo").as_deref(), Some("demo"));
    }
}
