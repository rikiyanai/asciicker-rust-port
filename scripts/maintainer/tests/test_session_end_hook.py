"""Tests for session_end_hook — claim telemetry collection and writing."""
import json
import sys
from pathlib import Path
from unittest.mock import patch, MagicMock

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.hooks.session_end_hook import (
    _collect_claim_telemetry,
    _write_claim_telemetry,
)


class TestCollectClaimTelemetry:
    def test_returns_none_when_no_session_jsonl(self):
        """No session JSONL found → return None."""
        with patch(
            "scripts.maintainer.hooks.session_end_hook.find_latest_session_jsonl",
            return_value=None,
        ):
            result = _collect_claim_telemetry()
        assert result is None

    def test_calls_find_latest_with_no_args(self):
        """find_latest_session_jsonl must be called with no args to trigger auto-resolution."""
        mock_find = MagicMock(return_value=None)
        with patch(
            "scripts.maintainer.hooks.session_end_hook.find_latest_session_jsonl",
            mock_find,
        ):
            _collect_claim_telemetry()

        mock_find.assert_called_once_with()

    def test_counts_unsupported_claims(self, tmp_path):
        """Assistant messages with forbidden words produce warning counts."""
        from scripts.maintainer.lib.jsonl_parser import SessionRecord, ParsedSession

        session_file = tmp_path / "session.jsonl"
        session_file.write_text("{}")

        mock_records = [
            SessionRecord(record_type="assistant", text="The pipeline is fully resolved and complete."),
            SessionRecord(record_type="assistant", text="Checking git status now."),
        ]
        mock_parsed = ParsedSession(path=str(session_file), records=mock_records)

        with patch(
            "scripts.maintainer.hooks.session_end_hook.find_latest_session_jsonl",
            return_value=session_file,
        ), patch(
            "scripts.maintainer.hooks.session_end_hook.parse_session_jsonl",
            return_value=mock_parsed,
        ), patch(
            "scripts.maintainer.hooks.session_end_hook.get_assistant_messages",
            return_value=mock_records,
        ):
            result = _collect_claim_telemetry()

        assert result is not None
        assert result["unsupported_claim_warnings"] >= 1
        assert result["assistant_messages_checked"] == 2
        assert "triggered_phrases" in result

    def test_clean_messages_zero_warnings(self, tmp_path):
        """Messages without forbidden words produce zero warnings."""
        from scripts.maintainer.lib.jsonl_parser import SessionRecord, ParsedSession

        session_file = tmp_path / "session.jsonl"
        session_file.write_text("{}")

        mock_records = [
            SessionRecord(record_type="assistant", text="Running tests now. Let me check the output."),
            SessionRecord(record_type="assistant", text="The pipeline produced FL-001 results."),
        ]
        mock_parsed = ParsedSession(path=str(session_file), records=mock_records)

        with patch(
            "scripts.maintainer.hooks.session_end_hook.find_latest_session_jsonl",
            return_value=session_file,
        ), patch(
            "scripts.maintainer.hooks.session_end_hook.parse_session_jsonl",
            return_value=mock_parsed,
        ), patch(
            "scripts.maintainer.hooks.session_end_hook.get_assistant_messages",
            return_value=mock_records,
        ):
            result = _collect_claim_telemetry()

        assert result is not None
        assert result["unsupported_claim_warnings"] == 0

    def test_returns_none_when_libs_unavailable(self):
        """Gracefully handles missing claim libs."""
        with patch(
            "scripts.maintainer.hooks.session_end_hook._CLAIM_LIBS_AVAILABLE",
            False,
        ):
            result = _collect_claim_telemetry()
        assert result is None


class TestWriteClaimTelemetry:
    def test_writes_json_file(self, tmp_path):
        telemetry = {
            "timestamp": "2026-02-20T00:00:00+00:00",
            "session_jsonl": "/tmp/session.jsonl",
            "assistant_messages_checked": 10,
            "unsupported_claim_warnings": 3,
            "triggered_phrases": {"resolved": 2, "complete": 1},
        }

        with patch(
            "scripts.maintainer.hooks.session_end_hook._PROJECT_ROOT",
            tmp_path,
        ):
            path = _write_claim_telemetry(telemetry)

        assert path is not None
        assert path.exists()
        data = json.loads(path.read_text())
        assert data["unsupported_claim_warnings"] == 3
        assert data["triggered_phrases"]["resolved"] == 2

    def test_creates_artifact_dir(self, tmp_path):
        """Creates artifacts/maintainer/ if it doesn't exist."""
        telemetry = {"timestamp": "now", "unsupported_claim_warnings": 0}

        with patch(
            "scripts.maintainer.hooks.session_end_hook._PROJECT_ROOT",
            tmp_path,
        ):
            path = _write_claim_telemetry(telemetry)

        assert path is not None
        assert (tmp_path / "artifacts" / "maintainer").is_dir()
