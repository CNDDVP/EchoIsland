#!/usr/bin/env python3
"""EchoIsland external session watcher (unofficial, local integration).

Watches local session stores of AI tools that have no hook mechanism and
pushes their activity to the EchoIsland HTTP receiver:

  - Kimi CLI        ~/.kimi/sessions/*/*/context.jsonl   (source: kimi)
  - Antigravity     ~/.gemini/antigravity/conversations/*.db (source: antigravity)
  - Codex App       ~/.codex/state_5.sqlite threads       (source: codex)

Codex App threads are scanned by EchoIsland's native Rust adapter. The legacy
watcher is disabled by default; set ECHOISLAND_WATCHER_CODEX_APP=1 only when
using this pack with an older app that has no native thread scanner.

Observation only: events are informational and non-blocking. Failed delivery
is retried on the next poll without committing the deduplication state.
"""
import glob
import json
import os
import re
import sqlite3
import sys
import time
import urllib.request
import urllib.parse
from contextlib import closing
from datetime import datetime, timezone
from pathlib import Path

LOCALAPPDATA = os.environ.get("LOCALAPPDATA", os.path.expanduser("~\\AppData\\Local"))
HOME = os.path.expanduser("~")
RECEIVER_URL = "http://127.0.0.1:37892/event"
RECEIVER_STATUS_PATH = os.path.join(LOCALAPPDATA, "EchoIsland", "http-receiver.json")
TOKEN_PATH = os.path.join(LOCALAPPDATA, "EchoIsland", "ipc-token")
STATE_PATH = os.path.join(HOME, ".echoisland", "bin", "watcher-state.json")
LOG_PATH = os.path.join(HOME, ".echoisland", "bin", "watcher.log")
POLL_SECONDS = 5
MAX_MESSAGE = 400
MAX_JSONL_TAIL_BYTES = 1024 * 1024

# Native Rust scanning owns Codex App threads in this version. Opt in only when
# using this integration pack with an older EchoIsland installation.
ENABLE_CODEX_APP = os.environ.get("ECHOISLAND_WATCHER_CODEX_APP") == "1"
CODEX_DB = os.path.join(HOME, ".codex", "state_5.sqlite")
KIMI_GLOB = os.path.join(HOME, ".kimi", "sessions", "*", "*", "context.jsonl")
ANTIGRAVITY_GLOB = os.path.join(HOME, ".gemini", "antigravity", "conversations", "*.db")

TOOL_SUMMARY_RE = re.compile(r'"toolSummary"\s*:\s*"([^"]{2,200})"')


def translate(key):
    candidates = [Path(__file__).parent / "locales" / "zh-CN.json",
                  Path(__file__).resolve().parent.parent / "crates" / "i18n" / "locales" / "zh-CN.json"]
    for candidate in candidates:
        try:
            with candidate.open(encoding="utf-8") as handle:
                value = json.load(handle).get(key)
            if isinstance(value, str):
                return value
        except (OSError, ValueError):
            pass
    return key


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


HTTP_OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())


def receiver_url():
    candidate = RECEIVER_URL
    try:
        with open(RECEIVER_STATUS_PATH, encoding="utf-8") as handle:
            status = json.load(handle)
    except (OSError, ValueError):
        status = {}
    if not isinstance(status, dict):
        raise ValueError(translate("integration.receiver_loopback"))
    for field in ("event_url", "addr"):
        if field in status and not isinstance(status[field], str):
            raise ValueError(translate("integration.receiver_loopback"))
    if status.get("event_url", "").strip():
        candidate = status["event_url"].strip()
    elif status.get("addr", "").strip():
        candidate = "http://%s/event" % status["addr"].strip()
    if not isinstance(candidate, str):
        raise ValueError(translate("integration.receiver_loopback"))
    parsed = urllib.parse.urlsplit(candidate)
    if (parsed.scheme != "http" or parsed.hostname not in ("127.0.0.1", "::1")
            or parsed.username or parsed.password or parsed.path != "/event"
            or parsed.query or parsed.fragment):
        raise ValueError(translate("integration.receiver_loopback"))
    return candidate


def log(message):
    try:
        with open(LOG_PATH, "a", encoding="utf-8") as handle:
            handle.write("[%s] %s\n" % (datetime.now().isoformat(timespec="seconds"), message))
    except OSError:
        pass


def load_state():
    try:
        with open(STATE_PATH, "r", encoding="utf-8") as handle:
            loaded = json.load(handle)
            return loaded if isinstance(loaded, dict) else {}
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
    if not session_id or not token:
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
    try:
        request = urllib.request.Request(
            receiver_url(),
            data=json.dumps({"event": envelope}).encode("utf-8"),
            headers={"Content-Type": "application/json", "x-echoisland-token": token},
            method="POST",
        )
        with HTTP_OPENER.open(request, timeout=3) as response:
            result = json.loads(response.read(65536))
        if not isinstance(result, dict) or result.get("ok") is not True:
            return False
        log("pushed source=%s event=%s session=%s" % (source, event_name, session_id[:40]))
        return True
    except (OSError, ValueError, TypeError) as error:
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


def read_recent_jsonl(path):
    """Return (records, omitted_complete_line) using at most 1 MiB + 1 byte.

    A completed line larger than the window is file activity only: its partial
    bytes must never be presented as an Agent message. Incomplete writes wait.
    """
    with open(path, "rb") as handle:
        handle.seek(0, os.SEEK_END)
        size = handle.tell()
        start = max(0, size - MAX_JSONL_TAIL_BYTES)
        # Include the preceding byte so an exact line boundary is not discarded.
        handle.seek(start - 1 if start else 0)
        raw = handle.read(size - start + (1 if start else 0))
    starts_mid_line = bool(start and raw[:1] != b"\n")
    if start:
        raw = raw[1:]
    if not raw or not raw.endswith(b"\n"):
        return [], False
    omitted_complete_line = starts_mid_line
    if starts_mid_line:
        raw = raw.partition(b"\n")[2]
    records = []
    for line in raw.splitlines()[-10:]:
        try:
            value = json.loads(line)
            if isinstance(value, dict):
                records.append(value)
        except (ValueError, UnicodeError, RecursionError):
            continue
    return records, omitted_complete_line


# --- kimi: ~/.kimi/sessions/<workspace>/<session>/context.jsonl -------------

def poll_kimi(state, token):
    now = time.time()
    for path in glob.glob(KIMI_GLOB):
        try:
            mtime = os.path.getmtime(path)
        except OSError:
            continue
        age = now - mtime
        if age > 7200:
            continue
        session_dir = os.path.basename(os.path.dirname(path))
        stamp = file_stamp(path)
        key = "kimi:%s" % session_dir
        is_active = age <= 120
        event_name = "UserPromptSubmit" if is_active else "Stop"
        if state.get(key) == [stamp, event_name]:
            continue
        try:
            records, omitted_complete_line = read_recent_jsonl(path)
        except OSError:
            continue
        if not records and not omitted_complete_line:
            continue
        last_msg = None
        for record in reversed(records):
            content = record.get("content")
            if content:
                last_msg = content
                break
        cwd = None
        try:
            cwd = os.path.dirname(os.path.dirname(os.path.dirname(path)))
        except OSError:
            pass
        # Only record state after a successful push so failures retry next poll.
        if push_event(token, "kimi", session_dir, event_name,
                      message=clamp(str(last_msg or "Kimi CLI")), cwd=cwd,
                      window_title="Kimi CLI"):
            state[key] = [stamp, event_name]


# --- antigravity: conversations/<uuid>.db (steps protobuf, tool summaries) --

SUMMARIES_PB = os.path.join(HOME, ".gemini", "antigravity", "agyhub_summaries_proto.pb")


def _read_varint(buf, offset):
    res = 0
    shift = 0
    while offset < len(buf):
        b = buf[offset]
        offset += 1
        res |= (b & 0x7f) << shift
        if not (b & 0x80):
            break
        shift += 7
    return res, offset


def get_antigravity_titles():
    titles = {}
    if not os.path.exists(SUMMARIES_PB):
        return titles
    try:
        with open(SUMMARIES_PB, "rb") as f:
            data = f.read()
    except OSError:
        return titles

    uuid_re = re.compile(rb'([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})')
    for m in uuid_re.finditer(data):
        uuid_str = m.group(1).decode("ascii")
        idx = m.end()
        if idx < len(data) and data[idx] == 0x12:
            sub_len, offset = _read_varint(data, idx + 1)
            if offset < len(data) and data[offset] == 0x0a:
                title_len, offset = _read_varint(data, offset + 1)
                title = data[offset:offset+title_len].decode("utf-8", "ignore").strip()
                if title:
                    titles[uuid_str] = title
    return titles


def poll_antigravity(state, token):
    titles = get_antigravity_titles()
    now = time.time()
    for path in glob.glob(ANTIGRAVITY_GLOB):
        session_id = os.path.splitext(os.path.basename(path))[0]
        try:
            mtime = max(os.path.getmtime(path),
                        os.path.getmtime(path + "-wal") if os.path.exists(path + "-wal") else 0)
        except OSError:
            continue
        age = now - mtime
        # Only inspect conversations from the last 2 hours
        if age > 7200:
            continue

        stamp = [file_stamp(path), file_stamp(path + "-wal")]
        key = "antigravity:%s" % session_id
        title = titles.get(session_id)
        is_active = age <= 120  # Active if touched in last 2 minutes
        event_name = "PostToolUse" if is_active else "Stop"

        if state.get(key) == [stamp, title, event_name]:
            continue

        summary = None
        if is_active:
            try:
                with closing(sqlite3.connect(Path(path).resolve().as_uri() + "?mode=ro",
                                             uri=True, timeout=0.1)) as connection:
                    rows = connection.execute(
                        "SELECT step_payload FROM steps ORDER BY idx DESC LIMIT 30"
                    ).fetchall()
                for (payload,) in rows:
                    if payload is None:
                        continue
                    text = payload.decode("utf-8", "replace")
                    match = TOOL_SUMMARY_RE.search(text)
                    if match:
                        summary = match.group(1)
                        break
            except sqlite3.Error:
                # A locked/partially replaced database must be retried unchanged.
                continue

        # Only record state after a successful push so failures retry next poll.
        if push_event(token, "antigravity", session_id,
                      event_name,
                      message=clamp(summary or translate("integration.antigravity_activity" if is_active else "status.idle")),
                      cwd=title, window_title=title or "Antigravity"):
            state[key] = [stamp, title, event_name]


# --- codex app: ~/.codex/state_5.sqlite threads -----------------------------

def poll_codex_app(state, token):
    stamp = [file_stamp(CODEX_DB), file_stamp(CODEX_DB + "-wal")]
    key = "codexapp:threads"
    try:
        with closing(sqlite3.connect(Path(CODEX_DB).resolve().as_uri() + "?mode=ro",
                                     uri=True, timeout=0.1)) as connection:
            rows = connection.execute(
                "SELECT id, title, first_user_message, updated_at_ms, cwd "
                "FROM threads WHERE archived = 0 ORDER BY updated_at_ms DESC LIMIT 20"
            ).fetchall()
    except sqlite3.Error:
        return
    now_ms = time.time() * 1000
    for thread_id, title, first_message, updated_ms, cwd in rows:
        age_ms = now_ms - (updated_ms or 0)
        # Only inspect threads updated in the last 2 hours
        if age_ms > 7200 * 1000:
            continue
        thread_key = "codexapp:thread:%s" % thread_id
        is_active = age_ms <= 120_000  # Active if touched in last 2 minutes
        event_name = "UserPromptSubmit" if is_active else "Stop"
        if state.get(thread_key) == [updated_ms, event_name]:
            continue
        message = first_message or title or ""
        # Only record state after a successful push so failures retry next poll.
        if push_event(token, "codex", thread_id, event_name,
                      message=clamp(message), cwd=cwd,
                      window_title="Codex App"):
            state[thread_key] = [updated_ms, event_name]


def poll_all(state, token):
    if ENABLE_CODEX_APP:
        poll_codex_app(state, token)
    poll_kimi(state, token)
    poll_antigravity(state, token)


def main():
    once = "--once" in sys.argv
    state = load_state()
    log("watcher started (pid=%s, once=%s)" % (os.getpid(), once))
    while True:
        try:
            token = read_token()
            if token:
                poll_all(state, token)
                save_state(state)
        except Exception as error:  # keep the watcher alive no matter what
            log("poll error: %r" % error)
        if once:
            break
        time.sleep(POLL_SECONDS)


if __name__ == "__main__":
    main()
