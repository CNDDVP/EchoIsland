// Shared by the managed OpenClaw adapter and the standalone integration pack.
import { readFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

let locale;
export function t(key) {
  if (!locale) {
    for (const relative of ["./locales/zh-CN.json", "../../crates/i18n/locales/zh-CN.json"]) {
      try {
        locale = JSON.parse(readFileSync(new URL(relative, import.meta.url), "utf8"));
        break;
      } catch { /* The standalone pack and installed plugin use different roots. */ }
    }
  }
  return locale?.[key] ?? key;
}

export function runtimePaths(env = process.env, platform = process.platform, home = os.homedir()) {
  const root = platform === "win32"
    ? (env.LOCALAPPDATA || env.APPDATA || home)
    : platform === "darwin"
      ? path.join(home, "Library", "Application Support")
      : (env.XDG_STATE_HOME || path.join(home, ".local", "state"));
  return {
    tokenPath: path.join(root, "EchoIsland", "ipc-token"),
    statusPath: path.join(root, "EchoIsland", "http-receiver.json"),
  };
}

export function validateReceiverUrl(value) {
  const url = new URL(value);
  if (url.protocol !== "http:" || !["127.0.0.1", "[::1]"].includes(url.hostname)
      || url.username || url.password || url.pathname !== "/event" || url.search || url.hash) {
    throw new Error(t("integration.receiver_loopback"));
  }
  return url.href;
}

export function createTransport({
  tokenPath = runtimePaths().tokenPath,
  statusPath = runtimePaths().statusPath,
  receiverUrl = "http://127.0.0.1:37892/event",
  readFileImpl = readFile,
  fetchImpl = globalThis.fetch,
} = {}) {
  async function currentReceiverUrl() {
    let status;
    try {
      status = JSON.parse(await readFileImpl(statusPath, "utf8"));
    } catch {
      return validateReceiverUrl(receiverUrl);
    }
    // Invalid published addresses fail closed; never send the token elsewhere.
    if (typeof status?.event_url === "string" && status.event_url.trim()) {
      return validateReceiverUrl(status.event_url);
    }
    if (typeof status?.addr === "string" && status.addr.trim()) {
      return validateReceiverUrl(`http://${status.addr}/event`);
    }
    return validateReceiverUrl(receiverUrl);
  }

  return async function postEvent(event, { timeoutMs } = {}) {
    if (!event) return undefined;
    const url = await currentReceiverUrl();
    // The receiver may rotate its token while the agent/plugin stays running.
    const token = (await readFileImpl(tokenPath, "utf8")).trim();
    if (!token) return undefined;
    const blocking = ["PermissionRequest", "AskUserQuestion"].includes(event.hook_event_name);
    const response = await fetchImpl(url, {
      method: "POST",
      redirect: "error",
      signal: AbortSignal.timeout(timeoutMs ?? (blocking ? 300_000 : 3_000)),
      headers: { "content-type": "application/json", "x-echoisland-token": token },
      body: JSON.stringify({ event }),
    });
    if (!response.ok) return undefined;
    return await response.json();
  };
}
