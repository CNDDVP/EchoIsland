// EchoIsland adapter for ZCode (unofficial, local integration).
// Reads a ZCode hook event from stdin, normalizes fields to the EchoIsland
// hook-bridge protocol (source=zcode), forwards it, and always exits 0.
// Non-blocking events only — ZCode PermissionRequest stays on the StackChan hook.
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import os from "node:os";
import path from "node:path";

const exe =
  process.platform === "win32" ? "echoisland-hook-bridge.exe" : "echoisland-hook-bridge";
const bridge = path.join(os.homedir(), ".echoisland", "bin", exe);

const chunks = [];
let bytes = 0;
for await (const chunk of process.stdin) {
  bytes += chunk.length;
  if (bytes > 1_048_576) process.exit(0);
  chunks.push(chunk);
}
let payload = {};
try {
  payload = JSON.parse(Buffer.concat(chunks).toString("utf8"));
} catch {
  payload = {};
}

if (payload && typeof payload === "object") {
  if (Array.isArray(payload)) process.exit(0);
  const event = String(payload.hook_event_name ?? "").replace(/[_-]/g, "").toLowerCase();
  if (["permissionrequest", "askuserquestion", "elicitation"].includes(event)) process.exit(0);
  if (payload.message == null) {
    const msg =
      payload.tool_input?.description ??
      payload.tool_input?.command ??
      payload.prompt ??
      payload.last_assistant_message;
    if (msg != null) payload.message = String(msg);
  }
  if (payload.cwd == null) payload.cwd = process.cwd();
}

if (existsSync(bridge)) {
  const child = spawn(bridge, ["--source", "zcode"], {
    stdio: ["pipe", "ignore", "ignore"],
    windowsHide: true,
  });
  child.on("error", () => process.exit(0));
  child.on("close", () => process.exit(0));
  child.stdin.on("error", () => process.exit(0));
  child.stdin.end(JSON.stringify(payload));
  setTimeout(() => { child.kill(); process.exit(0); }, 5_000).unref();
} else {
  process.exit(0);
}
