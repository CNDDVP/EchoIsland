use std::sync::{Arc, Mutex as StdMutex};

use echoisland_adapters::{
    ProcessSourceScanner, ProcessSourceSpec, cursor_process_spec, gemini_cli_process_spec,
    glm_cli_process_spec, trae_process_spec, vscode_process_spec,
};
use echoisland_runtime::SharedRuntime;
use tokio::time::Duration;

use crate::session_scan_runner::{SessionScanner, create_session_watcher, spawn_session_scan_loop};

impl SessionScanner for ProcessSourceScanner {
    fn scan(&mut self) -> anyhow::Result<Option<Vec<echoisland_core::SessionRecord>>> {
        ProcessSourceScanner::scan(self)
    }

    fn recommended_poll_interval(&self) -> Duration {
        ProcessSourceScanner::recommended_poll_interval(self)
    }
}

pub fn spawn_process_source_scan_loops(runtime: Arc<SharedRuntime>) {
    for spec in [
        gemini_cli_process_spec(),
        glm_cli_process_spec(),
        vscode_process_spec(),
        cursor_process_spec(),
        trae_process_spec(),
    ] {
        spawn_process_source_scan_loop(runtime.clone(), spec);
    }
}

fn spawn_process_source_scan_loop(runtime: Arc<SharedRuntime>, spec: ProcessSourceSpec) {
    let source = spec.source;
    let scanner = Arc::new(StdMutex::new(ProcessSourceScanner::new(spec)));
    spawn_session_scan_loop(
        runtime,
        source,
        scanner,
        move |watch_tx| create_session_watcher(source, watch_tx, Vec::new()),
        "failed to scan process source sessions",
    );
}
