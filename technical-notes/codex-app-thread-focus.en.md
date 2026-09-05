# Codex App Click-To-Focus Plan

Last updated: `2026-05-20`

Status: Codex App's private thread deeplink format is confirmed and EchoIsland now uses it as the first best-effort thread focus path. End-to-end validation is still needed in a packaged build.

## Goal

When EchoIsland shows a Codex App session, clicking that message should:

- Baseline: bring Codex App to the foreground.
- Enhanced behavior: try to return Codex App to the matching thread.
- Fallback: if thread-level resume fails, still activate Codex App.

## Confirmed Details

Codex App's macOS bundle id is:

```text
com.openai.codex
```

EchoIsland's current macOS focus path can already activate native apps by bundle id:

- Panel click emits `FocusSession`.
- Runtime resolves `session_id` into `SessionFocusTarget`.
- The macOS focus backend resolves `host_app` / `terminal_bundle`.
- `com.openai.codex` is activated through the native app activation path.

Relevant local code:

- `apps/desktop/src-tauri/src/native_panel_renderer/descriptors.rs`
- `apps/desktop/src-tauri/src/terminal_focus_service.rs`
- `apps/desktop/src-tauri/src/terminal_focus/macos.rs`
- `apps/desktop/src-tauri/src/terminal_focus/macos/native_apps.rs`
- `apps/desktop/src-tauri/src/terminal_focus/macos/target.rs`
- `crates/adapters/src/codex/scan.rs`

## Codex App Findings

The public OpenAI Codex repository exposes URL schemes such as:

```text
codex://threads/new
codex://plugins/<plugin-name>?marketplacePath=<path>
```

`codex://threads/new` opens a new thread, so it is not suitable for returning to an existing thread.

The installed `Codex.app` desktop shell registers `codex://`, and its "Copy deeplink" action copies this format:

```text
codex://threads/<conversation-id>
```

So the most direct current thread-level focus path is opening that deeplink.

The public repository also exposes `codex app-server`:

- Protocol: JSON-RPC 2.0, with `"jsonrpc": "2.0"` omitted on the wire.
- Default unix socket: `$CODEX_HOME/app-server-control/app-server-control.sock`.
- The unix socket carries WebSocket Upgrade followed by one JSON-RPC message per WebSocket text frame.
- Relevant method: `thread/resume`.

Example request:

```json
{
  "method": "thread/resume",
  "id": 11,
  "params": {
    "threadId": "thr_123",
    "excludeTurns": true
  }
}
```

The local app-server can still be used as a fallback. However, the installed Codex App currently runs its child server as `codex app-server --listen stdio://`, so external apps cannot connect to that stdio channel directly; the default unix socket may not exist.

## Recommended Implementation

### 1. Preserve Codex App Ownership During Scanning

When reading thread metadata from the Codex App state sqlite database:

- Use the Codex thread id as `session_id`.
- Keep `source` as `codex`.
- Set `host_app` to `com.openai.codex`.
- Use `cwd`, `model`, `title`, and `first_user_message` for display.
- Keep `rollout_path` as a future enhancement field, while preferring `threadId` for the first implementation.

This makes basic app activation work even without thread-level resume.

### 2. Try Codex Deeplink On Click

When the focus target matches:

- `target.source == "codex"`
- `target.host_app == Some("com.openai.codex")`
- `target.session_id` is non-empty

Then:

1. Open:

   ```text
   codex://threads/<session_id>
   ```

2. If system `open` succeeds, treat the jump request as handed off to Codex App.
3. If the deeplink fails, resolve Codex Home, defaulting to `~/.codex`, while respecting `CODEX_HOME`.
4. Build the socket path:

   ```text
   $CODEX_HOME/app-server-control/app-server-control.sock
   ```

5. If the socket exists, connect to it.
6. Perform WebSocket Upgrade.
7. Send `initialize`.
8. Send the `initialized` notification.
9. Send `thread/resume`:

   ```json
   {
     "method": "thread/resume",
     "id": 2,
     "params": {
       "threadId": "<session_id>",
       "excludeTurns": true
     }
   }
   ```

10. Read the response and log diagnostics.
11. Activate `com.openai.codex`.

### 3. Always Keep App Activation As Fallback

Deeplink or `thread/resume` failure should never block click behavior.

The app should still be activated when:

- Codex App is not running.
- The deeplink protocol is not registered or is rejected by the system.
- The socket does not exist.
- WebSocket Upgrade fails.
- `initialize` fails.
- `thread/resume` returns an error.
- The protocol changes.

The user-visible flow should always be:

```text
click message -> try Codex thread deeplink -> optionally try app-server resume -> activate Codex App
```

## Risks

### 1. The Deeplink Is A Private Desktop Protocol

`codex://threads/<id>` comes from the currently installed Codex App desktop shell. It is not a stable public contract in the open repository. Future Codex App versions may change the format, so fallback behavior must remain.

### 2. App-Server Resume May Not Switch Desktop UI

`thread/resume` can resume a thread inside app-server, but it is not guaranteed that the Codex App desktop UI switches its selected thread when another local client sends that request.

If the desktop UI does not observe that state, the fallback may only:

- activate Codex App
- make app-server load the thread context

without guaranteeing UI selection.

### 3. The Desktop Shell Source May Not Be In The Public Repo

The public repository mostly contains CLI, TUI, app-server, SDK, protocol, and login assets. It does not appear to include the complete macOS app shell, such as `Info.plist` or an Xcode project.

EchoIsland therefore relies on the behavior exposed by the installed desktop shell for `codex://threads/<id>`.

### 4. The Protocol Can Evolve

The app-server README marks WebSocket transport as experimental / unsupported. The unix socket control plane is the local entry point, but implementation details may still change across Codex versions.

Implementation should use:

- short timeouts
- clear diagnostics
- strict fallback behavior
- no user-visible error for resume failure

## Verification Plan

On a macOS machine with Codex App installed:

1. Open Codex App and create at least two threads.
2. Start EchoIsland.
3. Confirm Codex App threads have `host_app = com.openai.codex`.
4. Click a Codex App session in EchoIsland.
5. Confirm Codex App is brought to the foreground.
6. Confirm whether the matching thread is selected.
7. Check EchoIsland diagnostics:

   - whether the deeplink was handed to the system successfully
   - whether the socket exists
   - if the app-server fallback runs, whether initialize / `thread/resume` succeeds
   - whether app activation succeeds

## Recommended Phases

Phase 1: stable app activation only.

- Low cost.
- Low risk.
- The current code already supports most of it.

Phase 2: add `codex://threads/<id>` deeplink opening.

- Matches Codex App's own "Copy deeplink" format.
- Should let Codex App handle thread switching itself.
- Failure does not affect activation.

Phase 3: keep app-server `thread/resume` as an observable fallback.

Options:

- activate after observing Codex App state sqlite changes
- align with a future public Codex App protocol

## Conclusion

The current implementation path is `click message -> open codex://threads/<session_id> -> fallback activate Codex App`.

`codex://threads/<id>` is confirmed from the installed Codex App's "Copy deeplink" logic. App-server `thread/resume` remains a fallback and observation point.
