// echoisland-openclaw-plugin
import { readFile } from "node:fs/promises";
import { definePluginEntry } from "openclaw/plugin-sdk/plugin-entry";

const RECEIVER_URL = "http://127.0.0.1:37892/event";
const RECEIVER_STATUS_PATH = "C:\\Users\\chukaixin\\AppData\\Local\\EchoIsland\\http-receiver.json";
const TOKEN_PATH = "C:\\Users\\chukaixin\\AppData\\Local\\EchoIsland\\ipc-token";

let cachedToken = null;

function textValue(value) {
  if (value == null) return undefined;
  if (typeof value === "string") {
    const trimmed = value.trim();
    return trimmed || undefined;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      const text = textValue(item);
      if (text) return text;
    }
    return undefined;
  }
  if (typeof value === "object") {
    return (
      textValue(value.content) ??
      textValue(value.text) ??
      textValue(value.message) ??
      textValue(value.title) ??
      textValue(value.name)
    );
  }
  return String(value);
}

function objectValue(value) {
  return value && typeof value === "object" && !Array.isArray(value) ? value : undefined;
}

function sessionId(event, ctx) {
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
}

function workspaceDir(event, ctx) {
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
}

function modelName(event, ctx) {
  return textValue(event?.model ?? event?.modelId ?? event?.providerModel ?? ctx?.model ?? ctx?.modelId);
}

function messageText(event) {
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
}

function toolInput(event) {
  return objectValue(event?.params ?? event?.toolInput ?? event?.input ?? event?.arguments);
}

function toolDescription(event) {
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
}

function envelope(hookEventName, event, ctx, extra = {}) {
  const id = sessionId(event, ctx);
  if (!id) return null;
  const cwd = workspaceDir(event, ctx);

  return {
    protocol_version: "1",
    hook_event_name: hookEventName,
    source: "openclaw",
    session_id: id,
    timestamp: new Date().toISOString(),
    cwd,
    model: modelName(event, ctx),
    message: messageText(event),
    metadata: {
      terminal_app: "openclaw",
      host_app: "cli",
      window_title: "OpenClaw",
      workspace_roots: cwd ? [cwd] : undefined,
    },
    ...extra,
  };
}

async function token() {
  if (cachedToken) return cachedToken;
  cachedToken = (await readFile(TOKEN_PATH, "utf8")).trim();
  return cachedToken;
}

async function receiverUrl() {
  try {
    const status = JSON.parse(await readFile(RECEIVER_STATUS_PATH, "utf8"));
    if (typeof status?.event_url === "string" && status.event_url.trim()) {
      return status.event_url;
    }
    if (typeof status?.addr === "string" && status.addr.trim()) {
      return `http://${status.addr}/event`;
    }
  } catch (_error) {
  }
  return RECEIVER_URL;
}

async function postEvent(event) {
  if (!event) return undefined;
  const authToken = await token();
  if (!authToken) return undefined;
  const response = await fetch(await receiverUrl(), {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-echoisland-token": authToken,
    },
    body: JSON.stringify({ event }),
  });
  if (!response.ok) return undefined;
  return await response.json();
}

async function tryPost(event) {
  try {
    return await postEvent(event);
  } catch (_error) {
    return undefined;
  }
}

function deniedByEchoIsland(response) {
  return response?.decision?.behavior === "deny";
}

export default definePluginEntry({
  id: "echoisland",
  name: "EchoIsland",
  description: "Forward OpenClaw runtime events to EchoIsland.",
  register(api) {
    api.on("session_start", async (event, ctx) => {
      await tryPost(envelope("SessionStart", event, ctx));
    });

    api.on("session_end", async (event, ctx) => {
      await tryPost(envelope("SessionEnd", event, ctx));
    });

    api.on("message_received", async (event, ctx) => {
      await tryPost(envelope("UserPromptSubmit", event, ctx));
    });

    api.on("message_sent", async (event, ctx) => {
      await tryPost(envelope("AfterAgentResponse", event, ctx));
    });

    api.on("before_tool_call", async (event, ctx) => {
      const toolName = textValue(event?.toolName ?? event?.name ?? event?.tool?.name);
      const approval = envelope("PermissionRequest", event, ctx, {
        tool_name: toolName,
        tool_input: {
          ...(toolInput(event) ?? {}),
          description: toolDescription(event),
        },
      });
      const response = await tryPost(approval);
      if (deniedByEchoIsland(response)) {
        return {
          block: true,
          blockReason: "Denied by EchoIsland",
        };
      }
      return undefined;
    });

    api.on("after_tool_call", async (event, ctx) => {
      const toolName = textValue(event?.toolName ?? event?.name ?? event?.tool?.name);
      await tryPost(
        envelope("PostToolUse", event, ctx, {
          tool_name: toolName,
          tool_input: {
            ...(toolInput(event) ?? {}),
            description: toolDescription(event),
          },
        }),
      );
    });

    api.on("agent_end", async (event, ctx) => {
      await tryPost(envelope("SessionEnd", event, ctx));
    });
  },
});
