import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";
import http from "node:http";
import { createTransport, runtimePaths, validateReceiverUrl } from "../openclaw-plugin/echoisland-http.mjs";

test("only literal loopback HTTP event endpoints are accepted", () => {
  for (const value of ["http://127.0.0.1:37892/event", "http://[::1]:37892/event"]) {
    assert.equal(validateReceiverUrl(value), value);
  }
  for (const value of ["https://example.com/event", "http://localhost/event", "http://127.0.0.1.example.com/event",
    "http://127.0.0.1/other", "http://secret@127.0.0.1/event", "http://127.0.0.1/event?token=x", "file:///event"]) {
    assert.throws(() => validateReceiverUrl(value));
  }
});

test("runtime paths follow the current user and platform", () => {
  assert.match(runtimePaths({ LOCALAPPDATA: "D:/新用户/Local" }, "win32", "/home/user").tokenPath,
    /新用户.*EchoIsland.*ipc-token/);
  assert.match(runtimePaths({}, "darwin", "/Users/new").statusPath, /Application Support.*EchoIsland/);
  assert.match(runtimePaths({ XDG_STATE_HOME: "/custom/state" }, "linux", "/home/new").tokenPath, /custom.*state/);
});

test("transport follows published port, rereads rotated token and forbids redirects", async () => {
  let token = "first";
  let port = 40001;
  const requests = [];
  const post = createTransport({
    tokenPath: "token", statusPath: "status",
    readFileImpl: async (file) => file === "token" ? token : JSON.stringify({ addr: `127.0.0.1:${port}` }),
    fetchImpl: async (url, options) => {
      requests.push({ url, options });
      return { ok: true, json: async () => ({ ok: true }) };
    },
  });
  await post({ hook_event_name: "Stop" });
  token = "second"; port = 40002;
  await post({ hook_event_name: "Stop" });
  assert.equal(requests[0].options.headers["x-echoisland-token"], "first");
  assert.equal(requests[1].options.headers["x-echoisland-token"], "second");
  assert.equal(requests[1].url, "http://127.0.0.1:40002/event");
  assert.equal(requests[1].options.redirect, "error");
  assert.ok(requests[1].options.signal instanceof AbortSignal);
});

test("untrusted status never receives token or a request", async () => {
  const reads = [];
  const post = createTransport({
    tokenPath: "token", statusPath: "status",
    readFileImpl: async (file) => { reads.push(file); return '{"event_url":"http://example.com/event"}'; },
    fetchImpl: async () => assert.fail("must not contact an external host"),
  });
  await assert.rejects(post({ hook_event_name: "Stop" }));
  assert.deepEqual(reads, ["status"]);
});

test("stalled request is aborted", async () => {
  const post = createTransport({
    tokenPath: "token", statusPath: "status",
    readFileImpl: async (file) => file === "token" ? "token" : "{}",
    fetchImpl: async (_url, { signal }) => new Promise((_resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("request was not aborted")), 1000);
      signal.addEventListener("abort", () => { clearTimeout(timer); reject(signal.reason); });
    }),
  });
  await assert.rejects(post({ hook_event_name: "Stop" }, { timeoutMs: 5 }), { name: "TimeoutError" });
});

test("real HTTP redirect cannot forward the token to another endpoint", async () => {
  let redirectedRequests = 0;
  const server = http.createServer((request, response) => {
    if (request.url === "/event") {
      response.writeHead(307, { location: "/redirected" });
      response.end();
    } else {
      redirectedRequests += 1;
      response.end('{"ok":true}');
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  try {
    const receiverUrl = `http://127.0.0.1:${server.address().port}/event`;
    const post = createTransport({
      tokenPath: "token", statusPath: "status", receiverUrl,
      readFileImpl: async (file) => file === "token" ? "secret" : "{}",
    });
    await assert.rejects(post({ hook_event_name: "Stop" }));
    assert.equal(redirectedRequests, 0);
  } finally {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
});

test("plugin normalizes tool calls and preserves explicit approval decisions", async () => {
  const source = await readFile(new URL("../openclaw-plugin/index.ts", import.meta.url), "utf8");
  const events = [];
  const handlers = {};
  let response = { decision: { behavior: "deny" } };
  const context = {
    createTransport: () => async (event) => { events.push(event); return response; },
    definePluginEntry: (entry) => entry,
    t: (key) => key,
  };
  vm.runInNewContext(source.replace(/^import .*;\r?\n/gm, "").replace("export default", "globalThis.plugin ="), context);
  context.plugin.register({ on: (name, handler) => { handlers[name] = handler; } });
  const input = { sessionId: "session", toolName: "Bash", params: { command: "cargo test" } };
  const denied = await handlers.before_tool_call(input, { cwd: "/repo" });
  assert.equal(denied.block, true);
  assert.equal(events[0].hook_event_name, "PermissionRequest");
  assert.equal(events[0].tool_input.command, "cargo test");
  assert.equal(events[0].session_id, "session");
  response = { decision: { behavior: "allow" } };
  assert.equal(await handlers.before_tool_call(input, {}), undefined);
  await handlers.after_tool_call(input, {});
  assert.equal(events.at(-1).hook_event_name, "PostToolUse");
});
