use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};

use super::{OpenClawPaths, OpenClawStatus};
use crate::install_support::{load_json_object, write_json_object};
use crate::platform_support::supported_with_note;

pub const DEFAULT_OPENCLAW_RECEIVER_URL: &str = "http://127.0.0.1:37892/event";
const OPENCLAW_HOOK_ID: &str = "echoisland";
const OPENCLAW_PLUGIN_ID: &str = "echoisland";

pub fn install_openclaw_adapter(paths: &OpenClawPaths) -> Result<OpenClawStatus> {
    fs::create_dir_all(&paths.openclaw_dir)
        .with_context(|| format!("failed to create {}", paths.openclaw_dir.display()))?;
    fs::create_dir_all(&paths.hook_dir)
        .with_context(|| format!("failed to create {}", paths.hook_dir.display()))?;
    fs::create_dir_all(&paths.plugin_dir)
        .with_context(|| format!("failed to create {}", paths.plugin_dir.display()))?;

    fs::write(&paths.hook_manifest_path, render_hook_manifest())
        .with_context(|| format!("failed to write {}", paths.hook_manifest_path.display()))?;
    fs::write(&paths.hook_handler_path, render_hook_handler(paths)?)
        .with_context(|| format!("failed to write {}", paths.hook_handler_path.display()))?;
    fs::write(&paths.plugin_package_path, render_plugin_package())
        .with_context(|| format!("failed to write {}", paths.plugin_package_path.display()))?;
    fs::write(&paths.plugin_manifest_path, render_plugin_manifest())
        .with_context(|| format!("failed to write {}", paths.plugin_manifest_path.display()))?;
    fs::write(&paths.plugin_entry_path, render_plugin_entry(paths)?)
        .with_context(|| format!("failed to write {}", paths.plugin_entry_path.display()))?;

    ensure_hook_enabled(paths)?;
    ensure_plugin_enabled(paths)?;
    get_openclaw_status(paths)
}

pub fn get_openclaw_status(paths: &OpenClawPaths) -> Result<OpenClawStatus> {
    let hook_installed =
        paths.hook_manifest_path.exists() && hook_has_echoisland_marker(&paths.hook_handler_path)?;
    let hook_enabled = hook_enabled(paths).unwrap_or(false);
    let plugin_installed = paths.plugin_manifest_path.exists()
        && paths.plugin_package_path.exists()
        && plugin_has_echoisland_marker(&paths.plugin_entry_path)?;
    let plugin_enabled = plugin_enabled(paths).unwrap_or(false);
    let token_exists = paths.token_path.exists();
    let support = supported_with_note(
        "OpenClaw support uses a managed internal hook plus a local EchoIsland plugin. The hook captures command/message/session events; the plugin captures tool calls and EchoIsland approvals when OpenClaw loads plugins.",
    );

    Ok(OpenClawStatus {
        openclaw_dir_exists: paths.openclaw_dir.exists(),
        config_path_exists: paths.config_path.exists(),
        hooks_dir_exists: paths.hooks_dir.exists(),
        hook_installed,
        hook_enabled,
        plugin_installed,
        plugin_enabled,
        token_exists,
        live_capture_supported: support.supported,
        live_capture_ready: hook_installed
            && hook_enabled
            && plugin_installed
            && plugin_enabled
            && token_exists,
        status_note: support.note,
        openclaw_dir: paths.openclaw_dir.display().to_string(),
        config_path: paths.config_path.display().to_string(),
        hooks_dir: paths.hooks_dir.display().to_string(),
        hook_dir: paths.hook_dir.display().to_string(),
        hook_manifest_path: paths.hook_manifest_path.display().to_string(),
        hook_handler_path: paths.hook_handler_path.display().to_string(),
        plugin_dir: paths.plugin_dir.display().to_string(),
        plugin_package_path: paths.plugin_package_path.display().to_string(),
        plugin_manifest_path: paths.plugin_manifest_path.display().to_string(),
        plugin_entry_path: paths.plugin_entry_path.display().to_string(),
        token_path: paths.token_path.display().to_string(),
        receiver_url: paths.receiver_url.clone(),
    })
}

fn hook_has_echoisland_marker(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(raw.contains("echoisland-openclaw-hook"))
}

fn plugin_has_echoisland_marker(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(raw.contains("echoisland-openclaw-plugin"))
}

fn hook_enabled(paths: &OpenClawPaths) -> Result<bool> {
    if !paths.config_path.exists() {
        return Ok(false);
    }
    let root = load_json_object(&paths.config_path)?;
    Ok(root
        .get("hooks")
        .and_then(Value::as_object)
        .and_then(|hooks| hooks.get("internal"))
        .and_then(Value::as_object)
        .and_then(|internal| internal.get("entries"))
        .and_then(Value::as_object)
        .and_then(|entries| entries.get(OPENCLAW_HOOK_ID))
        .and_then(Value::as_object)
        .and_then(|entry| entry.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false))
}

fn plugin_enabled(paths: &OpenClawPaths) -> Result<bool> {
    if !paths.config_path.exists() {
        return Ok(false);
    }
    let root = load_json_object(&paths.config_path)?;
    let plugins_enabled = root
        .get("plugins")
        .and_then(Value::as_object)
        .and_then(|plugins| plugins.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let plugin_dir = normalized_path_string(&paths.plugin_dir);
    let load_path_present = root
        .get("plugins")
        .and_then(Value::as_object)
        .and_then(|plugins| plugins.get("load"))
        .and_then(Value::as_object)
        .and_then(|load| load.get("paths"))
        .and_then(Value::as_array)
        .is_some_and(|paths| {
            paths
                .iter()
                .filter_map(Value::as_str)
                .any(|path| path == plugin_dir)
        });
    let entry_enabled = root
        .get("plugins")
        .and_then(Value::as_object)
        .and_then(|plugins| plugins.get("entries"))
        .and_then(Value::as_object)
        .and_then(|entries| entries.get(OPENCLAW_PLUGIN_ID))
        .and_then(Value::as_object)
        .and_then(|entry| entry.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let denied = root
        .get("plugins")
        .and_then(Value::as_object)
        .and_then(|plugins| plugins.get("deny"))
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .any(|id| id == OPENCLAW_PLUGIN_ID)
        });

    Ok(plugins_enabled && load_path_present && entry_enabled && !denied)
}

fn ensure_hook_enabled(paths: &OpenClawPaths) -> Result<()> {
    let mut root = if paths.config_path.exists() {
        load_json_object(&paths.config_path)?
    } else {
        Map::new()
    };
    let hooks = root
        .entry("hooks".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("openclaw hooks config must be an object"))?;
    let internal = hooks_obj
        .entry("internal".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let internal_obj = internal
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("openclaw hooks.internal config must be an object"))?;
    let entries = internal_obj
        .entry("entries".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let entries_obj = entries.as_object_mut().ok_or_else(|| {
        anyhow::anyhow!("openclaw hooks.internal.entries config must be an object")
    })?;

    entries_obj.insert(OPENCLAW_HOOK_ID.to_string(), json!({ "enabled": true }));

    write_json_object(&paths.config_path, &root)
}

fn ensure_plugin_enabled(paths: &OpenClawPaths) -> Result<()> {
    let mut root = if paths.config_path.exists() {
        load_json_object(&paths.config_path)?
    } else {
        Map::new()
    };
    let plugins = root
        .entry("plugins".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let plugins_obj = plugins
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("openclaw plugins config must be an object"))?;

    plugins_obj.insert("enabled".to_string(), Value::Bool(true));
    ensure_plugin_load_path(plugins_obj, &normalized_path_string(&paths.plugin_dir))?;
    ensure_plugin_entry_enabled(plugins_obj)?;
    ensure_plugin_allowed(plugins_obj)?;
    remove_plugin_denied(plugins_obj);

    write_json_object(&paths.config_path, &root)
}

fn ensure_plugin_load_path(plugins_obj: &mut Map<String, Value>, plugin_dir: &str) -> Result<()> {
    let load = plugins_obj
        .entry("load".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let load_obj = load
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("openclaw plugins.load config must be an object"))?;
    let paths = load_obj
        .entry("paths".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let paths = paths
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("openclaw plugins.load.paths config must be an array"))?;
    if !paths.iter().any(|value| value.as_str() == Some(plugin_dir)) {
        paths.push(Value::String(plugin_dir.to_string()));
    }
    Ok(())
}

fn ensure_plugin_entry_enabled(plugins_obj: &mut Map<String, Value>) -> Result<()> {
    let entries = plugins_obj
        .entry("entries".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let entries_obj = entries
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("openclaw plugins.entries config must be an object"))?;
    let entry = entries_obj
        .entry(OPENCLAW_PLUGIN_ID.to_string())
        .or_insert_with(|| json!({}));
    let entry_obj = entry.as_object_mut().ok_or_else(|| {
        anyhow::anyhow!("openclaw plugins.entries.echoisland config must be an object")
    })?;
    entry_obj.insert("enabled".to_string(), Value::Bool(true));
    entry_obj
        .entry("config".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    Ok(())
}

fn ensure_plugin_allowed(plugins_obj: &mut Map<String, Value>) -> Result<()> {
    let Some(allow) = plugins_obj.get_mut("allow") else {
        return Ok(());
    };
    let allow = allow
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("openclaw plugins.allow config must be an array"))?;
    if !allow
        .iter()
        .filter_map(Value::as_str)
        .any(|id| id == OPENCLAW_PLUGIN_ID)
    {
        allow.push(Value::String(OPENCLAW_PLUGIN_ID.to_string()));
    }
    Ok(())
}

fn remove_plugin_denied(plugins_obj: &mut Map<String, Value>) {
    if let Some(deny) = plugins_obj.get_mut("deny").and_then(Value::as_array_mut) {
        deny.retain(|value| value.as_str() != Some(OPENCLAW_PLUGIN_ID));
    }
}

fn normalized_path_string(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn render_hook_manifest() -> &'static str {
    r#"---
metadata:
  openclaw:
    name: echoisland
    description: Forward OpenClaw session activity to EchoIsland.
    events:
      - command:new
      - command:reset
      - command:stop
      - message:received
      - message:sent
      - session:patch
---

# EchoIsland OpenClaw Hook

Managed by EchoIsland. Forwards OpenClaw session/message events to the local EchoIsland receiver.
"#
}

fn render_hook_handler(paths: &OpenClawPaths) -> Result<String> {
    let receiver_url = serde_json::to_string(&paths.receiver_url)?;
    let receiver_status_path =
        serde_json::to_string(&paths.receiver_status_path.display().to_string())?;
    let token_path = serde_json::to_string(&paths.token_path.display().to_string())?;

    Ok(format!(
        r#"// echoisland-openclaw-hook
import {{ readFile }} from "node:fs/promises";

const RECEIVER_URL = {receiver_url};
const RECEIVER_STATUS_PATH = {receiver_status_path};
const TOKEN_PATH = {token_path};

let cachedToken = null;

function getPath(root, path) {{
  let current = root;
  for (const key of path) {{
    if (current == null) return undefined;
    current = current[key];
  }}
  return current;
}}

function firstDefined(values) {{
  for (const value of values) {{
    if (value == null) continue;
    if (typeof value === "string" && value.trim() === "") continue;
    return value;
  }}
  return undefined;
}}

function textValue(value) {{
  if (value == null) return undefined;
  if (typeof value === "string") {{
    const trimmed = value.trim();
    return trimmed || undefined;
  }}
  if (Array.isArray(value)) {{
    return firstDefined(value.map(textValue));
  }}
  if (typeof value === "object") {{
    return firstDefined([
      textValue(value.content),
      textValue(value.text),
      textValue(value.message),
      textValue(value.title),
    ]);
  }}
  return undefined;
}}

function objectValue(value) {{
  return value && typeof value === "object" && !Array.isArray(value) ? value : undefined;
}}

function statusValue(input) {{
  return String(
    firstDefined([
      input?.status,
      getPath(input, ["context", "status"]),
      getPath(input, ["context", "sessionEntry", "status"]),
      getPath(input, ["context", "patch", "status"]),
      getPath(input, ["context", "patch", "state"]),
    ]) ?? "",
  ).toLowerCase();
}}

function eventNameFromStatus(status) {{
  if (!status) return undefined;
  if (["waiting", "waiting_question", "question", "input_required"].includes(status)) {{
    return "AskUserQuestion";
  }}
  if (["approval_required", "permission_required", "requires_approval"].includes(status)) {{
    return "PermissionRequest";
  }}
  if (["completed", "complete", "done", "failed", "error", "idle", "stopped"].includes(status)) {{
    return "SessionEnd";
  }}
  if (["running", "processing", "started"].includes(status)) {{
    return "SessionStart";
  }}
  return undefined;
}}

function eventNameFor(action, input) {{
  const status = statusValue(input);
  switch (String(action ?? "").toLowerCase()) {{
    case "command:new":
      return "SessionStart";
    case "command:reset":
      return "SessionEnd";
    case "session:patch":
      return eventNameFromStatus(status) ?? "SessionStart";
    case "message:received":
      return "UserPromptSubmit";
    case "message:sent":
      return "AfterAgentResponse";
    case "command:stop":
      return "Stop";
    default:
      return null;
  }}
}}

function sessionId(input) {{
  return textValue(
    firstDefined([
      input?.sessionKey,
      input?.sessionId,
      input?.id,
      getPath(input, ["context", "sessionKey"]),
      getPath(input, ["context", "sessionId"]),
      getPath(input, ["context", "sessionEntry", "sessionKey"]),
      getPath(input, ["context", "sessionEntry", "sessionId"]),
      getPath(input, ["context", "sessionEntry", "id"]),
      getPath(input, ["context", "patch", "sessionKey"]),
      getPath(input, ["context", "patch", "sessionId"]),
      getPath(input, ["context", "message", "sessionId"]),
    ]),
  );
}}

function workspaceDir(input) {{
  return textValue(
    firstDefined([
      input?.workspaceDir,
      input?.cwd,
      getPath(input, ["context", "workspaceDir"]),
      getPath(input, ["context", "cwd"]),
      getPath(input, ["context", "sessionEntry", "workspaceDir"]),
      getPath(input, ["context", "sessionEntry", "cwd"]),
      getPath(input, ["context", "patch", "workspaceDir"]),
      getPath(input, ["context", "patch", "cwd"]),
    ]),
  );
}}

function modelName(input) {{
  return textValue(
    firstDefined([
      input?.model,
      getPath(input, ["context", "model"]),
      getPath(input, ["context", "sessionEntry", "model"]),
      getPath(input, ["context", "patch", "model"]),
    ]),
  );
}}

function messageText(input) {{
  return textValue(
    firstDefined([
      input?.message,
      input?.content,
      input?.text,
      getPath(input, ["context", "content"]),
      getPath(input, ["context", "message"]),
      getPath(input, ["context", "message", "content"]),
      getPath(input, ["context", "message", "text"]),
      getPath(input, ["context", "patch", "message"]),
      getPath(input, ["context", "patch", "title"]),
      getPath(input, ["context", "patch", "summary"]),
    ]),
  );
}}

function toolName(input) {{
  return textValue(
    firstDefined([
      input?.toolName,
      input?.tool_name,
      getPath(input, ["context", "toolName"]),
      getPath(input, ["context", "tool_name"]),
      getPath(input, ["context", "tool", "name"]),
      getPath(input, ["context", "patch", "toolName"]),
      getPath(input, ["context", "patch", "currentTool"]),
    ]),
  );
}}

function toolInput(input) {{
  return objectValue(
    firstDefined([
      input?.toolInput,
      input?.tool_input,
      getPath(input, ["context", "toolInput"]),
      getPath(input, ["context", "tool_input"]),
      getPath(input, ["context", "tool", "input"]),
      getPath(input, ["context", "tool", "arguments"]),
      getPath(input, ["context", "patch", "toolInput"]),
    ]),
  );
}}

function questionPayload(input) {{
  const raw = firstDefined([
    input?.question,
    getPath(input, ["context", "question"]),
    getPath(input, ["context", "patch", "question"]),
  ]);
  const question = objectValue(raw);
  const text = textValue(firstDefined([question?.text, question?.message, raw, messageText(input)]));
  if (!text) return undefined;

  const options = Array.isArray(question?.options)
    ? question.options
        .map((option) => {{
          const label = textValue(option?.label ?? option?.text ?? option);
          if (!label) return undefined;
          return {{
            label,
            description: textValue(option?.description),
          }};
        }})
        .filter(Boolean)
    : [];

  return {{
    header: textValue(question?.header ?? question?.title),
    text,
    options,
  }};
}}

function isBlockingEvent(hookEventName) {{
  return hookEventName === "PermissionRequest" || hookEventName === "AskUserQuestion";
}}

function buildEnvelope(input) {{
  const action = String(input?.action ?? input?.type ?? "").toLowerCase();
  const hookEventName = eventNameFor(action, input);
  if (!hookEventName) return null;

  const id = sessionId(input);
  if (!id) return null;

  const cwd = workspaceDir(input);
  const tool = toolName(input);
  const toolPayload = toolInput(input);
  const question = questionPayload(input);

  return {{
    protocol_version: "1",
    hook_event_name: hookEventName,
    source: "openclaw",
    session_id: id,
    timestamp: new Date().toISOString(),
    cwd,
    model: modelName(input),
    message: messageText(input),
    tool_name: tool,
    tool_input: toolPayload,
    question,
    metadata: {{
      terminal_app: "openclaw",
      host_app: "cli",
      window_title: "OpenClaw",
      workspace_roots: cwd ? [cwd] : undefined,
    }},
  }};
}}

async function token() {{
  if (cachedToken) return cachedToken;
  cachedToken = (await readFile(TOKEN_PATH, "utf8")).trim();
  return cachedToken;
}}

async function receiverUrl() {{
  try {{
    const status = JSON.parse(await readFile(RECEIVER_STATUS_PATH, "utf8"));
    if (typeof status?.event_url === "string" && status.event_url.trim()) {{
      return status.event_url;
    }}
    if (typeof status?.addr === "string" && status.addr.trim()) {{
      return `http://${{status.addr}}/event`;
    }}
  }} catch (_error) {{
  }}
  return RECEIVER_URL;
}}

export default async function handler(input) {{
  const envelope = buildEnvelope(input);
  if (!envelope) return;

  try {{
    const authToken = await token();
    if (!authToken) return;

    const response = await fetch(await receiverUrl(), {{
      method: "POST",
      headers: {{
        "content-type": "application/json",
        "x-echoisland-token": authToken,
      }},
      body: JSON.stringify({{ event: envelope }}),
    }});

    if (isBlockingEvent(envelope.hook_event_name) && response.ok) {{
      return await response.json();
    }}
  }} catch (_error) {{
  }}
}}
"#
    ))
}

fn render_plugin_package() -> &'static str {
    r#"{
  "name": "echoisland-openclaw-plugin",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "openclaw": {
    "extensions": [
      "./index.ts"
    ]
  }
}
"#
}

fn render_plugin_manifest() -> &'static str {
    r#"{
  "id": "echoisland",
  "name": "EchoIsland",
  "description": "Forwards OpenClaw runtime events to EchoIsland.",
  "configSchema": {
    "type": "object",
    "additionalProperties": false
  }
}
"#
}

fn render_plugin_entry(paths: &OpenClawPaths) -> Result<String> {
    let receiver_url = serde_json::to_string(&paths.receiver_url)?;
    let receiver_status_path =
        serde_json::to_string(&paths.receiver_status_path.display().to_string())?;
    let token_path = serde_json::to_string(&paths.token_path.display().to_string())?;

    Ok(format!(
        r#"// echoisland-openclaw-plugin
import {{ readFile }} from "node:fs/promises";
import {{ definePluginEntry }} from "openclaw/plugin-sdk/plugin-entry";

const RECEIVER_URL = {receiver_url};
const RECEIVER_STATUS_PATH = {receiver_status_path};
const TOKEN_PATH = {token_path};

let cachedToken = null;

function textValue(value) {{
  if (value == null) return undefined;
  if (typeof value === "string") {{
    const trimmed = value.trim();
    return trimmed || undefined;
  }}
  if (Array.isArray(value)) {{
    for (const item of value) {{
      const text = textValue(item);
      if (text) return text;
    }}
    return undefined;
  }}
  if (typeof value === "object") {{
    return (
      textValue(value.content) ??
      textValue(value.text) ??
      textValue(value.message) ??
      textValue(value.title) ??
      textValue(value.name)
    );
  }}
  return String(value);
}}

function objectValue(value) {{
  return value && typeof value === "object" && !Array.isArray(value) ? value : undefined;
}}

function sessionId(event, ctx) {{
  return textValue(
    event?.sessionKey ??
      event?.sessionId ??
      event?.session_id ??
      event?.conversationId ??
      event?.conversation?.id ??
      ctx?.sessionKey ??
      ctx?.sessionId ??
      ctx?.conversationId ??
      ctx?.runId,
  );
}}

function workspaceDir(event, ctx) {{
  return textValue(
    event?.workspaceDir ??
      event?.cwd ??
      event?.workspace?.dir ??
      event?.sessionEntry?.workspaceDir ??
      event?.session?.workspaceDir ??
      ctx?.workspaceDir ??
      ctx?.cwd ??
      ctx?.workspace?.dir,
  );
}}

function modelName(event, ctx) {{
  return textValue(event?.model ?? event?.modelId ?? event?.providerModel ?? ctx?.model ?? ctx?.modelId);
}}

function messageText(event) {{
  return textValue(
    event?.content ??
      event?.message ??
      event?.text ??
      event?.bodyForAgent ??
      event?.reply ??
      event?.result?.content ??
      event?.toolResult?.content ??
      event?.error?.message,
  );
}}

function toolInput(event) {{
  return objectValue(event?.params ?? event?.toolInput ?? event?.input ?? event?.arguments);
}}

function toolDescription(event) {{
  const params = toolInput(event);
  return textValue(
    event?.description ??
      params?.description ??
      params?.command ??
      params?.file_path ??
      params?.path ??
      params?.pattern ??
      params?.prompt,
  );
}}

function envelope(hookEventName, event, ctx, extra = {{}}) {{
  const id = sessionId(event, ctx);
  if (!id) return null;
  const cwd = workspaceDir(event, ctx);

  return {{
    protocol_version: "1",
    hook_event_name: hookEventName,
    source: "openclaw",
    session_id: id,
    timestamp: new Date().toISOString(),
    cwd,
    model: modelName(event, ctx),
    message: messageText(event),
    metadata: {{
      terminal_app: "openclaw",
      host_app: "cli",
      window_title: "OpenClaw",
      workspace_roots: cwd ? [cwd] : undefined,
    }},
    ...extra,
  }};
}}

async function token() {{
  if (cachedToken) return cachedToken;
  cachedToken = (await readFile(TOKEN_PATH, "utf8")).trim();
  return cachedToken;
}}

async function receiverUrl() {{
  try {{
    const status = JSON.parse(await readFile(RECEIVER_STATUS_PATH, "utf8"));
    if (typeof status?.event_url === "string" && status.event_url.trim()) {{
      return status.event_url;
    }}
    if (typeof status?.addr === "string" && status.addr.trim()) {{
      return `http://${{status.addr}}/event`;
    }}
  }} catch (_error) {{
  }}
  return RECEIVER_URL;
}}

async function postEvent(event) {{
  if (!event) return undefined;
  const authToken = await token();
  if (!authToken) return undefined;
  const response = await fetch(await receiverUrl(), {{
    method: "POST",
    headers: {{
      "content-type": "application/json",
      "x-echoisland-token": authToken,
    }},
    body: JSON.stringify({{ event }}),
  }});
  if (!response.ok) return undefined;
  return await response.json();
}}

async function tryPost(event) {{
  try {{
    return await postEvent(event);
  }} catch (_error) {{
    return undefined;
  }}
}}

function deniedByEchoIsland(response) {{
  return response?.decision?.behavior === "deny";
}}

export default definePluginEntry({{
  id: "echoisland",
  name: "EchoIsland",
  description: "Forward OpenClaw runtime events to EchoIsland.",
  register(api) {{
    api.on("session_start", async (event, ctx) => {{
      await tryPost(envelope("SessionStart", event, ctx));
    }});

    api.on("session_end", async (event, ctx) => {{
      await tryPost(envelope("SessionEnd", event, ctx));
    }});

    api.on("message_received", async (event, ctx) => {{
      await tryPost(envelope("UserPromptSubmit", event, ctx));
    }});

    api.on("message_sent", async (event, ctx) => {{
      await tryPost(envelope("AfterAgentResponse", event, ctx));
    }});

    api.on("before_tool_call", async (event, ctx) => {{
      const toolName = textValue(event?.toolName ?? event?.name ?? event?.tool?.name);
      const approval = envelope("PermissionRequest", event, ctx, {{
        tool_name: toolName,
        tool_input: {{
          ...(toolInput(event) ?? {{}}),
          description: toolDescription(event),
        }},
      }});
      const response = await tryPost(approval);
      if (deniedByEchoIsland(response)) {{
        return {{
          block: true,
          blockReason: "Denied by EchoIsland",
        }};
      }}
      return undefined;
    }});

    api.on("after_tool_call", async (event, ctx) => {{
      const toolName = textValue(event?.toolName ?? event?.name ?? event?.tool?.name);
      await tryPost(
        envelope("PostToolUse", event, ctx, {{
          tool_name: toolName,
          tool_input: {{
            ...(toolInput(event) ?? {{}}),
            description: toolDescription(event),
          }},
        }}),
      );
    }});

    api.on("agent_end", async (event, ctx) => {{
      await tryPost(envelope("SessionEnd", event, ctx));
    }});
  }},
}});
"#
    ))
}
