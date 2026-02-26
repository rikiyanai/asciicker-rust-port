"""Tests for jsonl_parser — mixed record types, unknown types, malformed tolerance."""
import json
import sys
import tempfile
from pathlib import Path

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.jsonl_parser import (
    parse_session_jsonl, get_user_messages, get_assistant_messages,
    SessionRecord, ParsedSession, _extract_text_from_message,
)

FIXTURES_DIR = Path(__file__).parent / "fixtures"


class TestExtractText:
    def test_plain_string(self):
        assert _extract_text_from_message("hello") == "hello"

    def test_content_blocks(self):
        blocks = [
            {"type": "text", "text": "Hello"},
            {"type": "tool_use", "id": "x", "name": "Read"},
            {"type": "text", "text": "World"},
        ]
        result = _extract_text_from_message(blocks)
        assert "Hello" in result
        assert "World" in result

    def test_tool_result_excluded(self):
        blocks = [
            {"type": "text", "text": "checking"},
            {"type": "tool_result", "content": "big blob of data"},
        ]
        result = _extract_text_from_message(blocks)
        assert "checking" in result
        assert "big blob" not in result

    def test_dict_with_content(self):
        msg = {"content": "inner text"}
        assert _extract_text_from_message(msg) == "inner text"

    def test_empty_returns_empty(self):
        assert _extract_text_from_message(None) == ""
        assert _extract_text_from_message(42) == ""


class TestParseSessionJsonl:
    def test_sample_fixture(self):
        session = parse_session_jsonl(FIXTURES_DIR / "sample_session.jsonl")
        assert session.total_lines > 0
        assert len(session.records) > 0
        assert session.parse_errors == []

    def test_record_types_present(self):
        session = parse_session_jsonl(FIXTURES_DIR / "sample_session.jsonl")
        types = {r.record_type for r in session.records}
        assert "user" in types
        assert "assistant" in types
        assert "progress" in types

    def test_unknown_types_preserved(self):
        """Unknown record types become record_type='telemetry' (not crash)."""
        session = parse_session_jsonl(FIXTURES_DIR / "sample_session.jsonl")
        types = {r.record_type for r in session.records}
        # The sample has a "telemetry" type which is not in the known set
        assert "telemetry" in types

    def test_file_not_found(self):
        session = parse_session_jsonl("/nonexistent/path.jsonl")
        assert len(session.parse_errors) == 1
        assert "not found" in session.parse_errors[0].lower()

    def test_malformed_json_tolerance(self):
        """Malformed lines are logged as errors, not raised."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".jsonl", delete=False
        ) as f:
            f.write('{"type":"user","message":"good line"}\n')
            f.write('THIS IS NOT JSON\n')
            f.write('{"type":"assistant","message":"also good"}\n')
            f.write('\n')  # empty line
            tmp_path = f.name

        try:
            session = parse_session_jsonl(tmp_path)
            assert session.total_lines == 4
            assert len(session.records) == 2  # 2 valid records
            assert session.skipped_lines == 2  # 1 malformed + 1 empty
            assert len(session.parse_errors) == 1  # 1 JSON error
        finally:
            Path(tmp_path).unlink()

    def test_non_dict_lines_skipped(self):
        """Lines that parse as non-dict JSON are skipped."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".jsonl", delete=False
        ) as f:
            f.write('"just a string"\n')
            f.write('[1, 2, 3]\n')
            f.write('{"type":"user","message":"valid"}\n')
            tmp_path = f.name

        try:
            session = parse_session_jsonl(tmp_path)
            assert len(session.records) == 1
            assert session.skipped_lines == 2
        finally:
            Path(tmp_path).unlink()


class TestGetMessages:
    def test_user_messages_from_fixture(self):
        session = parse_session_jsonl(FIXTURES_DIR / "sample_session.jsonl")
        user_msgs = get_user_messages(session)
        assert len(user_msgs) >= 2
        # Check that tool results are excluded
        texts = [m.text for m in user_msgs]
        assert any("pipeline" in t.lower() or "fix" in t.lower() for t in texts)

    def test_assistant_messages_from_fixture(self):
        session = parse_session_jsonl(FIXTURES_DIR / "sample_session.jsonl")
        asst_msgs = get_assistant_messages(session)
        assert len(asst_msgs) >= 2

    def test_empty_text_excluded(self):
        """Records with empty text are filtered out."""
        session = ParsedSession(
            path="test",
            records=[
                SessionRecord(record_type="user", text="", line_number=1),
                SessionRecord(record_type="user", text="hello", line_number=2),
            ],
        )
        msgs = get_user_messages(session)
        assert len(msgs) == 1
        assert msgs[0].text == "hello"

    def test_content_blocks_in_assistant(self):
        """Assistant messages with content blocks extract text correctly."""
        session = parse_session_jsonl(FIXTURES_DIR / "sample_session.jsonl")
        asst_msgs = get_assistant_messages(session)
        # Line 4 has mixed content blocks
        texts = [m.text for m in asst_msgs]
        assert any("root cause" in t.lower() or "1536" in t for t in texts)
