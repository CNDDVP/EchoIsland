import importlib.util
from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import time
import unittest
from unittest.mock import patch, Mock


SCRIPT = Path(__file__).resolve().parents[1] / "ei-session-watcher.py"
spec = importlib.util.spec_from_file_location("watcher", SCRIPT)
watcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(watcher)


class WatcherTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.kimi = self.root / "kimi" / "workspace" / "session" / "context.jsonl"
        self.kimi.parent.mkdir(parents=True)
        self.antigravity = self.root / "antigravity" / "session.db"
        self.antigravity.parent.mkdir()
        self.codex = self.root / "state_5.sqlite"
        self.patches = patch.multiple(
            watcher, KIMI_GLOB=str(self.kimi), ANTIGRAVITY_GLOB=str(self.antigravity),
            SUMMARIES_PB=str(self.root / "missing.pb"), CODEX_DB=str(self.codex),
            STATE_PATH=str(self.root / "state.json"), LOG_PATH=str(self.root / "watcher.log"),
            RECEIVER_STATUS_PATH=str(self.root / "receiver.json"),
        )
        self.patches.start()
        self.addCleanup(self.patches.stop)

    def write_kimi(self, content="first"):
        self.kimi.write_text(json.dumps({"content": content}) + "\n", encoding="utf-8")

    def create_antigravity(self):
        connection = sqlite3.connect(self.antigravity)
        connection.execute("CREATE TABLE steps (idx INTEGER, step_payload BLOB)")
        connection.execute("INSERT INTO steps VALUES (1, ?)", (b'{"toolSummary":"test tool"}',))
        connection.commit()
        return connection

    def create_codex(self):
        with closing(sqlite3.connect(self.codex)) as connection:
            with connection:
                connection.execute("CREATE TABLE threads (id, title, first_user_message, updated_at_ms, cwd, archived)")
                connection.execute("INSERT INTO threads VALUES ('thread', 'title', 'prompt', ?, '/repo', 0)",
                                   (int(time.time() * 1000),))

    def assert_retry_then_dedupe(self, poll):
        state = {}
        with patch.object(watcher, "push_event", side_effect=[False, True]) as push:
            poll(state, "token")
            self.assertEqual(state, {})
            poll(state, "token")
            self.assertTrue(state)
            # The persisted representation must compare equal on the next run.
            state = json.loads(json.dumps(state))
            poll(state, "token")
            self.assertEqual(push.call_count, 2)

    def test_push_failure_retries_then_deduplicates_all_sources(self):
        self.write_kimi()
        self.create_antigravity().close()
        self.create_codex()
        for poll in (watcher.poll_kimi, watcher.poll_antigravity, watcher.poll_codex_app):
            with self.subTest(source=poll.__name__):
                self.assert_retry_then_dedupe(poll)

    def test_partial_jsonl_is_retried_when_completed(self):
        self.kimi.write_text('{"content":"partial', encoding="utf-8")
        state = {}
        with patch.object(watcher, "push_event", return_value=True) as push:
            watcher.poll_kimi(state, "token")
            self.assertEqual(state, {})
            push.assert_not_called()
            with self.kimi.open("a", encoding="utf-8") as handle:
                handle.write(' completed"}\n')
            watcher.poll_kimi(state, "token")
            self.assertEqual(push.call_args.kwargs["message"], "partial completed")

    def test_complete_100kb_record_is_parsed_and_reported(self):
        content = "真实消息" + "x" * 100_000
        self.write_kimi(content)
        records, omitted = watcher.read_recent_jsonl(self.kimi)
        self.assertFalse(omitted)
        self.assertEqual(records[0]["content"], content)
        with patch.object(watcher, "push_event", return_value=True) as push:
            watcher.poll_kimi({}, "token")
            push.assert_called_once()
            self.assertEqual(push.call_args.kwargs["message"], watcher.clamp(content))

    def test_oversized_complete_record_has_bounded_activity_only_fallback(self):
        self.write_kimi("private content " + "x" * (watcher.MAX_JSONL_TAIL_BYTES * 2))
        requests = []
        real_open = open

        class BoundedReader:
            def __init__(self, *args, **kwargs):
                self.handle = real_open(*args, **kwargs)

            def __enter__(self):
                return self

            def __exit__(self, *args):
                self.handle.close()

            def seek(self, *args):
                return self.handle.seek(*args)

            def tell(self):
                return self.handle.tell()

            def read(self, amount=-1):
                requests.append(amount)
                self_outer.assertGreaterEqual(amount, 0)
                return self.handle.read(amount)

        self_outer = self
        with patch.object(watcher, "open", BoundedReader, create=True):
            records, omitted = watcher.read_recent_jsonl(self.kimi)
        self.assertEqual(records, [])
        self.assertTrue(omitted)
        self.assertLessEqual(sum(requests), watcher.MAX_JSONL_TAIL_BYTES + 1)
        state = {}
        with patch.object(watcher, "push_event", return_value=True) as push:
            watcher.poll_kimi(state, "token")
            watcher.poll_kimi(state, "token")
            push.assert_called_once()
            self.assertEqual(push.call_args.kwargs["message"], "Kimi CLI")
            self.assertEqual(push.call_args.args[3], "UserPromptSubmit")

    def test_oversized_partial_record_waits_and_later_normal_message_is_read(self):
        content = json.dumps({"content": "x" * (watcher.MAX_JSONL_TAIL_BYTES + 100)})
        self.kimi.write_text(content, encoding="utf-8")
        state = {}
        with patch.object(watcher, "push_event", return_value=True) as push:
            watcher.poll_kimi(state, "token")
            self.assertEqual(state, {})
            push.assert_not_called()
            with self.kimi.open("a", encoding="utf-8") as handle:
                handle.write("\n")
            watcher.poll_kimi(state, "token")
            self.assertEqual(push.call_args.kwargs["message"], "Kimi CLI")
            with self.kimi.open("a", encoding="utf-8") as handle:
                handle.write('{"content":"new message"}\n')
            watcher.poll_kimi(state, "token")
            self.assertEqual(push.call_args.kwargs["message"], "new message")
            self.assertEqual(push.call_count, 2)

    def test_tail_starting_at_record_boundary_keeps_first_complete_record(self):
        record = b'{"content":"boundary"}\n'
        self.kimi.write_bytes(b'{"content":"old"}\n' + record)
        with patch.object(watcher, "MAX_JSONL_TAIL_BYTES", len(record)):
            records, omitted = watcher.read_recent_jsonl(self.kimi)
        self.assertFalse(omitted)
        self.assertEqual(records, [{"content": "boundary"}])

    def test_truncated_file_is_rescanned_and_empty_file_not_cached(self):
        self.write_kimi("long previous message")
        state = {}
        with patch.object(watcher, "push_event", return_value=True) as push:
            watcher.poll_kimi(state, "token")
            self.kimi.write_bytes(b"")
            watcher.poll_kimi(state, "token")
            self.assertEqual(push.call_count, 1)
            self.write_kimi("new")
            watcher.poll_kimi(state, "token")
            self.assertEqual(push.call_count, 2)
            self.assertEqual(push.call_args.kwargs["message"], "new")

    def test_locked_database_does_not_commit_deduplication(self):
        connection = self.create_antigravity()
        self.addCleanup(connection.close)
        connection.execute("BEGIN EXCLUSIVE")
        state = {}
        with patch.object(watcher, "push_event", return_value=True) as push:
            watcher.poll_antigravity(state, "token")
            self.assertEqual(state, {})
            push.assert_not_called()
            connection.rollback()
            watcher.poll_antigravity(state, "token")
            push.assert_called_once()

    def test_wal_activity_keeps_old_database_active(self):
        connection = self.create_antigravity()
        self.addCleanup(connection.close)
        connection.execute("PRAGMA journal_mode=WAL")
        connection.execute("INSERT INTO steps VALUES (2, ?)", (b'{"toolSummary":"latest WAL tool"}',))
        connection.commit()
        old = time.time() - 10800
        os.utime(self.antigravity, (old, old))
        with patch.object(watcher, "push_event", return_value=True) as push:
            watcher.poll_antigravity({}, "token")
            push.assert_called_once()
            self.assertEqual(push.call_args.kwargs["message"], "latest WAL tool")

    def test_activity_transitions_to_idle_and_history_is_filtered(self):
        self.write_kimi()
        state = {}
        with patch.object(watcher, "push_event", return_value=True) as push:
            watcher.poll_kimi(state, "token")
            self.assertEqual(push.call_args.args[3], "UserPromptSubmit")
            old = time.time() - 121
            os.utime(self.kimi, (old, old))
            watcher.poll_kimi(state, "token")
            self.assertEqual(push.call_args.args[3], "Stop")
            old = time.time() - 7201
            os.utime(self.kimi, (old, old))
            watcher.poll_kimi(state, "token")
            self.assertEqual(push.call_count, 2)

    def test_native_codex_scan_is_default_owner(self):
        with patch.object(watcher, "ENABLE_CODEX_APP", False), \
             patch.object(watcher, "poll_codex_app") as codex, \
             patch.object(watcher, "poll_kimi"), patch.object(watcher, "poll_antigravity"):
            watcher.poll_all({}, "token")
            codex.assert_not_called()

    def test_invalid_receiver_is_rejected_before_token_is_sent(self):
        Path(watcher.RECEIVER_STATUS_PATH).write_text('{"event_url":"https://example.com/event"}')
        with patch.object(watcher.HTTP_OPENER, "open") as opener:
            self.assertFalse(watcher.push_event("secret", "kimi", "s", "Stop"))
            opener.assert_not_called()

    def test_invalid_status_types_do_not_escape_or_block_other_sources(self):
        self.write_kimi()
        invalid = [[], None, "status", 42, {"event_url": 42}, {"event_url": None},
                   {"addr": []}, {"addr": True}, {"event_url": {}}]
        for status in invalid:
            with self.subTest(status=status):
                Path(watcher.RECEIVER_STATUS_PATH).write_text(json.dumps(status))
                with patch.object(watcher.HTTP_OPENER, "open") as opener, \
                     patch.object(watcher, "ENABLE_CODEX_APP", False), \
                     patch.object(watcher, "poll_antigravity") as antigravity:
                    state = {}
                    watcher.poll_all(state, "secret")
                    self.assertEqual(state, {})
                    opener.assert_not_called()
                    antigravity.assert_called_once_with(state, "secret")

    def test_receiver_status_recovers_after_type_error(self):
        status_path = Path(watcher.RECEIVER_STATUS_PATH)
        status_path.write_text('{"event_url":42}')
        with patch.object(watcher.HTTP_OPENER, "open") as opener:
            self.assertFalse(watcher.push_event("secret", "kimi", "s", "Stop"))
            opener.assert_not_called()
        status_path.write_text('{"addr":"127.0.0.1:43210"}')
        response = Mock()
        response.read.return_value = b'{"ok":true}'
        response.__enter__ = Mock(return_value=response)
        response.__exit__ = Mock(return_value=False)
        with patch.object(watcher.HTTP_OPENER, "open", return_value=response) as opener:
            self.assertTrue(watcher.push_event("rotated", "kimi", "s", "Stop"))
            request = opener.call_args.args[0]
            self.assertEqual(request.full_url, "http://127.0.0.1:43210/event")
            self.assertEqual(request.headers["X-echoisland-token"], "rotated")

    def test_http_success_requires_successful_protocol_acknowledgement(self):
        response = Mock()
        response.read.return_value = b'{"ok":false,"error":"unauthorized"}'
        response.__enter__ = Mock(return_value=response)
        response.__exit__ = Mock(return_value=False)
        with patch.object(watcher.HTTP_OPENER, "open", return_value=response):
            self.assertFalse(watcher.push_event("secret", "kimi", "s", "Stop"))
            response.read.return_value = b'{"ok":true}'
            self.assertTrue(watcher.push_event("secret", "kimi", "s", "Stop"))


if __name__ == "__main__":
    unittest.main()
