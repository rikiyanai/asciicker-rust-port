"""Tests for command_handoff_hook tool coverage."""
from __future__ import annotations

import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.hooks.command_handoff_hook import (
    _ALLOWED_TOOLS,
    _command_payload,
    _session_key,
)


class TestAllowedTools:
    def test_includes_full_required_tool_set(self):
        required = {"Bash", "Write", "Edit", "MultiEdit", "Read", "Grep", "Glob", "Task"}
        assert required.issubset(_ALLOWED_TOOLS)


class TestPayloadExtraction:
    def test_read_payload_contains_preview_and_keys(self):
        payload = _command_payload(
            "Read",
            {"file_path": "docs/INDEX.md", "offset": 10, "limit": 100},
        )
        assert payload["file_path"] == "docs/INDEX.md"
        assert "file_path" in payload["tool_input_keys"]
        assert "preview" in payload

    def test_task_payload_contains_preview(self):
        payload = _command_payload(
            "Task",
            {"description": "Run browser validation flow"},
        )
        assert payload["description"] == "Run browser validation flow"
        assert "preview" in payload


class TestSessionKey:
    def test_uses_explicit_session_id_when_present(self):
        key = _session_key({"session_id": "abc-123"})
        assert len(key) == 12
        assert key.startswith("fallback-") is False

    def test_falls_back_to_rotating_bucket_when_no_ids(self):
        key = _session_key({"tool_name": "Bash"})
        assert key.startswith("fallback-")
