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
for await (const chunk of process.stdin) chunks.push(chunk);
let payload = {};
try {
  payload = JSON.parse(Buffer.concat(chunks).toString("utf8"));
} catch {
  payload = {};
}

if (payload && typeof payload === "object") {
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
  });
  child.stdin.end(JSON.stringify(payload));
  child.on("error", () => process.exit(0));
  child.on("close", () => process.exit(0));
} else {
  process.exit(0);
}
