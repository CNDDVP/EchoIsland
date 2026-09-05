use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use echoisland_adapters::{
    ClaudeAdapter, CodexAdapter, InstallableAdapter, OpenClawAdapter, SessionScanningAdapter,
};
use echoisland_core::{EventEnvelope, ResponseEnvelope};
use echoisland_ipc::{DEFAULT_ADDR, EventHandler, send_raw, serve_tcp};
use echoisland_paths::bridge_binary_name;
use echoisland_runtime::SharedRuntime;
use std::process::ExitCode;
use tokio::fs;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug)]
struct HostRuntime {
    runtime: SharedRuntime,
}

impl Default for HostRuntime {
    fn default() -> Self {
        Self {
            runtime: SharedRuntime::new(),
        }
    }
}

#[async_trait]
impl EventHandler for HostRuntime {
    async fn handle_event(&self, event: EventEnvelope) -> ResponseEnvelope {
        self.runtime.handle_event(event).await
    }
}

#[derive(Debug)]
struct UserFacingError(String);

impl std::fmt::Display for UserFacingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
impl std::error::Error for UserFacingError {}

fn user_error(message: String) -> anyhow::Error {
    anyhow::Error::new(UserFacingError(message))
}

#[tokio::main]
async fn main() -> ExitCode {
    setup_tracing();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = if let Some(message) = error.downcast_ref::<UserFacingError>() {
                message.to_string()
            } else {
                echoisland_i18n::error("cli.operation_failed", error)
            };
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("serve") => {
            let addr = args.next().unwrap_or_else(|| DEFAULT_ADDR.to_string());
            serve_tcp(&addr, Arc::new(HostRuntime::default())).await
        }
        Some("send") => {
            let mut addr = DEFAULT_ADDR.to_string();
            let mut file: Option<PathBuf> = None;
            let mut from_stdin = false;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--addr" => {
                        addr = args.next().ok_or_else(|| {
                            user_error(echoisland_i18n::format(
                                "cli.missing_value",
                                &[("argument", "--addr")],
                            ))
                        })?;
                    }
                    "--file" => {
                        file = Some(PathBuf::from(args.next().ok_or_else(|| {
                            user_error(echoisland_i18n::format(
                                "cli.missing_value",
                                &[("argument", "--file")],
                            ))
                        })?));
                    }
                    "--stdin" => from_stdin = true,
                    other => {
                        return Err(user_error(echoisland_i18n::format(
                            "cli.unknown_argument",
                            &[("argument", other)],
                        )));
                    }
                }
            }

            let payload = if let Some(path) = file {
                fs::read(path).await?
            } else if from_stdin {
                read_stdin().await?
            } else {
                return Err(user_error(
                    echoisland_i18n::t("cli.input_required").to_string(),
                ));
            };

            let response = send_raw(&addr, &payload).await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }
        Some("snapshot") => {
            let runtime = SharedRuntime::new();
            let snapshot = runtime.snapshot().await;
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
            Ok(())
        }
        Some("codex-status") => {
            let status = CodexAdapter::with_default_paths().status()?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        Some("claude-status") => {
            let status = ClaudeAdapter::with_default_paths().status()?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        Some("openclaw-status") => {
            let status = OpenClawAdapter::with_default_paths().status()?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        Some("codex-scan") => {
            let sessions = CodexAdapter::with_default_paths().scan_sessions()?;
            println!("{}", serde_json::to_string_pretty(&sessions)?);
            Ok(())
        }
        Some("claude-scan") => {
            let sessions = ClaudeAdapter::with_default_paths().scan_sessions()?;
            println!("{}", serde_json::to_string_pretty(&sessions)?);
            Ok(())
        }
        Some("install-codex") => {
            let mut bridge_path: Option<PathBuf> = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--bridge" => {
                        bridge_path = Some(PathBuf::from(args.next().ok_or_else(|| {
                            user_error(echoisland_i18n::format(
                                "cli.missing_value",
                                &[("argument", "--bridge")],
                            ))
                        })?));
                    }
                    other => {
                        return Err(user_error(echoisland_i18n::format(
                            "cli.unknown_argument",
                            &[("argument", other)],
                        )));
                    }
                }
            }

            let bridge_path = bridge_path.unwrap_or_else(default_bridge_path);
            if !bridge_path.exists() {
                return Err(user_error(echoisland_i18n::format(
                    "cli.bridge_missing",
                    &[("path", &bridge_path.display().to_string())],
                )));
            }

            let status = CodexAdapter::with_default_paths().install(&bridge_path)?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        Some("install-claude") => {
            let mut bridge_path: Option<PathBuf> = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--bridge" => {
                        bridge_path = Some(PathBuf::from(args.next().ok_or_else(|| {
                            user_error(echoisland_i18n::format(
                                "cli.missing_value",
                                &[("argument", "--bridge")],
                            ))
                        })?));
                    }
                    other => {
                        return Err(user_error(echoisland_i18n::format(
                            "cli.unknown_argument",
                            &[("argument", other)],
                        )));
                    }
                }
            }

            let bridge_path = bridge_path.unwrap_or_else(default_bridge_path);
            if !bridge_path.exists() {
                return Err(user_error(echoisland_i18n::format(
                    "cli.bridge_missing",
                    &[("path", &bridge_path.display().to_string())],
                )));
            }

            let status = ClaudeAdapter::with_default_paths().install(&bridge_path)?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        Some("install-openclaw") => {
            let status =
                OpenClawAdapter::with_default_paths().install(std::path::Path::new("."))?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        Some(other) => Err(user_error(echoisland_i18n::format(
            "cli.unknown_command",
            &[("command", other)],
        ))),
    }
}

fn setup_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).with_target(false).try_init();
}

async fn read_stdin() -> anyhow::Result<Vec<u8>> {
    use tokio::io::{self, AsyncReadExt};

    let mut stdin = io::stdin();
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer).await?;
    Ok(buffer)
}

fn default_bridge_path() -> PathBuf {
    if let Some(target_dir) = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from) {
        return target_dir.join("debug").join(bridge_binary_name());
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    root.join("target").join("debug").join(bridge_binary_name())
}
