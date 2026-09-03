#!/usr/bin/env python3
"""EchoIsland external session watcher (unofficial, local integration).

Watches local session stores of AI tools that have no hook mechanism and
pushes their activity to the EchoIsland HTTP receiver:

  - Kimi CLI        ~/.kimi/sessions/*/*/context.jsonl   (source: kimi)
  - Antigravity     ~/.gemini/antigravity/conversations/*.db (source: antigravity)
  - Codex App       ~/.codex/state_5.sqlite threads       (source: codex)

EchoIsland v0.6.1 only scans Codex App threads in newer unreleased versions;
the codex-app section here covers that gap until an upgrade (then set
ENABLE_CODEX_APP = False to avoid duplicate cards).

Observation only: events are informational, always non-blocking, and failures
are silently dropped so the watched tools are never disturbed.
"""
import glob
import json
import os
import re
import sqlite3
import sys
import time
import urllib.request
from datetime import datetime, timezone

LOCALAPPDATA = os.environ.get("LOCALAPPDATA", os.path.expanduser("~\\AppData\\Local"))
HOME = os.path.expanduser("~")
RECEIVER_URL = "http://127.0.0.1:37892/event"
TOKEN_PATH = os.path.join(LOCALAPPDATA, "EchoIsland", "ipc-token")
STATE_PATH = os.path.join(HOME, ".echoisland", "bin", "watcher-state.json")
LOG_PATH = os.path.join(HOME, ".echoisland", "bin", "watcher.log")
POLL_SECONDS = 5
MAX_MESSAGE = 400

ENABLE_CODEX_APP = True
CODEX_DB = os.path.join(HOME, ".codex", "state_5.sqlite")
KIMI_GLOB = os.path.join(HOME, ".kimi", "sessions", "*", "*", "context.jsonl")
ANTIGRAVITY_GLOB = os.path.join(HOME, ".gemini", "antigravity", "conversations", "*.db")

TOOL_SUMMARY_RE = re.compile(r'"toolSummary"\s*:\s*"([^"]{2,200})"')


def log(message):
    try:
        with open(LOG_PATH, "a", encoding="utf-8") as handle:
            handle.write("[%s] %s\n" % (datetime.now().isoformat(timespec="seconds"), message))
    except OSError:
        pass


def load_state():
    try:
        with open(STATE_PATH, "r", encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, ValueError):
        return {}


def save_state(state):
    tmp = STATE_PATH + ".tmp"
    try:
        with open(tmp, "w", encoding="utf-8") as handle:
            json.dump(state, handle)
        os.replace(tmp, STATE_PATH)
    except OSError:
        pass


def read_token():
    try:
        with open(TOKEN_PATH, "r", encoding="utf-8") as handle:
            token = handle.read().strip()
        return token or None
    except OSError:
        return None


def push_event(token, source, session_id, event_name, message=None, cwd=None,
               window_title=None):
    if not session_id:
        return False
    envelope = {
        "protocol_version": "1",
        "hook_event_name": event_name,
        "source": source,
        "session_id": session_id,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "cwd": cwd,
        "message": (message or "")[:MAX_MESSAGE] or None,
        "metadata": {
            "terminal_app": source,
            "host_app": "cli",
            "window_title": window_title or source,
        },
    }
    request = urllib.request.Request(
        RECEIVER_URL,
        data=json.dumps({"event": envelope}).encode("utf-8"),
        headers={"Content-Type": "application/json", "x-echoisland-token": token or ""},
        method="POST",
    )
    try:
        urllib.request.urlopen(request, timeout=3).read()
        log("pushed source=%s event=%s session=%s" % (source, event_name, session_id[:40]))
        return True
    except OSError as error:
        log("push failed source=%s session=%s: %s" % (source, session_id[:40], error))
        return False


def file_stamp(path):
    try:
        stat = os.stat(path)
        # lists (not tuples) so state survives the JSON round-trip unchanged
        return [stat.st_mtime_ns, stat.st_size]
    except OSError:
        return None


def clamp(text, limit=MAX_MESSAGE):
    text = (text or "").strip()
    if len(text) > limit:
        text = text[:limit] + "…"
    return text


# --- kimi: ~/.kimi/sessions/<workspace>/<session>/context.jsonl -------------

def poll_kimi(state, token):
    for path in glob.glob(KIMI_GLOB):
        session_dir = os.path.basename(os.path.dirname(path))
        stamp = file_stamp(path)
        key = "kimi:%s" % session_dir
        previous = state.get(key)
        prev_stamp = previous[:2] if isinstance(previous, list) and len(previous) >= 2 else None
        if stamp is None or prev_stamp == stamp:
            continue
        lines_seen_prev = previous[2] if isinstance(previous, list) and len(previous) > 2 else 0
        try:
            with open(path, "r", encoding="utf-8") as handle:
                lines = handle.readlines()
        except OSError:
            continue
        records = []
        for line in lines:
            try:
                records.append(json.loads(line))
            except ValueError:
                continue
        state[key] = [stamp[0], stamp[1], len(lines)]
        if not records:
            continue
        last_user = None
        last_assistant = None
        new_user_after_prev = False
        for index, record in enumerate(records):
            role = record.get("role")
            if role == "user":
                last_user = record.get("content")
                if index >= lines_seen_prev:
                    new_user_after_prev = True
            elif role == "assistant":
                last_assistant = record.get("content")
        cwd = None
        try:
            cwd = os.path.dirname(os.path.dirname(os.path.dirname(path)))
        except OSError:
            pass
        if new_user_after_prev and last_user:
            push_event(token, "kimi", session_dir, "UserPromptSubmit",
                       message=clamp(str(last_user)), cwd=cwd, window_title="Kimi CLI")
        elif last_assistant:
            push_event(token, "kimi", session_dir, "Stop",
                       message=clamp(str(last_assistant)), cwd=cwd, window_title="Kimi CLI")


# --- antigravity: conversations/<uuid>.db (steps protobuf, tool summaries) --

def poll_antigravity(state, token):
    for path in glob.glob(ANTIGRAVITY_GLOB):
        session_id = os.path.splitext(os.path.basename(path))[0]
        stamp = [file_stamp(path), file_stamp(path + "-wal")]
        key = "antigravity:%s" % session_id
        if state.get(key) == stamp:
            continue
        is_new = key not in state
        state[key] = stamp
        summary = None
        try:
            connection = sqlite3.connect("file:%s?mode=ro" % path.replace("\\", "/"), uri=True)
            rows = connection.execute(
                "SELECT step_payload FROM steps ORDER BY idx DESC LIMIT 30"
            ).fetchall()
            connection.close()
            for (payload,) in rows:
                if payload is None:
                    continue
                text = payload.decode("utf-8", "replace")
                match = TOOL_SUMMARY_RE.search(text)
                if match:
                    summary = match.group(1)
                    break
        except sqlite3.Error:
            continue
        push_event(token, "antigravity", session_id,
                   "SessionStart" if is_new else "PostToolUse",
                   message=clamp(summary or "Antigravity conversation activity"),
                   cwd=None, window_title="Antigravity")


# --- codex app: ~/.codex/state_5.sqlite threads -----------------------------

def poll_codex_app(state, token):
    stamp = [file_stamp(CODEX_DB), file_stamp(CODEX_DB + "-wal")]
    key = "codexapp:threads"
    if state.get(key) == stamp:
        return
    try:
        connection = sqlite3.connect("file:%s?mode=ro" % CODEX_DB.replace("\\", "/"), uri=True)
        rows = connection.execute(
            "SELECT id, title, first_user_message, updated_at_ms, cwd "
            "FROM threads ORDER BY updated_at_ms DESC LIMIT 20"
        ).fetchall()
        connection.close()
    except sqlite3.Error:
        return
    state[key] = stamp
    for thread_id, title, first_message, updated_ms, cwd in rows:
        thread_key = "codexapp:thread:%s" % thread_id
        if state.get(thread_key) == updated_ms:
            continue
        state[thread_key] = updated_ms
        message = first_message or title or ""
        push_event(token, "codex", thread_id, "UserPromptSubmit",
                   message=clamp(message), cwd=cwd,
                   window_title="Codex App")


def poll_all(state, token):
    if ENABLE_CODEX_APP:
        poll_codex_app(state, token)
    poll_kimi(state, token)
    poll_antigravity(state, token)


def main():
    once = "--once" in sys.argv
    state = load_state()
    token = read_token()
    if token is None:
        log("no ipc token at %s; exiting" % TOKEN_PATH)
        return
    log("watcher started (pid=%s, once=%s)" % (os.getpid(), once))
    while True:
        try:
            poll_all(state, token)
            save_state(state)
        except Exception as error:  # keep the watcher alive no matter what
            log("poll error: %r" % error)
        if once:
            break
        time.sleep(POLL_SECONDS)


if __name__ == "__main__":
    main()
