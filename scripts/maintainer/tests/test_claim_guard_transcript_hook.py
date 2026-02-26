"""Tests for claim_guard_transcript_hook — transcript-based claim blocking."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.hooks.claim_guard_transcript_hook import (
    FL_REF,
    COMMIT_REF,
    META_PATTERNS,
    PHASE_13_COMPLETE_RE,
    PHASE_13_4_MISSING_RE,
    _is_meta_discussion,
    _phase_fact_findings,
    _extract_assistant_text,
    _tail_read_jsonl,
)


class TestEvidencePatterns:
    """FL_REF and COMMIT_REF should match expected patterns."""

    def test_fl_ref_matches_standard(self):
        assert FL_REF.search("See FL-001 for details")
        assert FL_REF.search("FL-999 status update")
        assert FL_REF.search("Reference: FL-0123")

    def test_fl_ref_rejects_invalid(self):
        assert not FL_REF.search("FL-01")  # too short
        assert not FL_REF.search("FL01")   # missing dash

    def test_commit_ref_matches_7char_hex(self):
        assert COMMIT_REF.search("commit abc1234")
        assert COMMIT_REF.search("hash: deadbeef")

    def test_commit_ref_rejects_short_hex(self):
        assert not COMMIT_REF.search("abc12")  # too short (5 chars)


class TestMetaDiscussion:
    """Meta-discussion detection should skip claim guard's own output."""

    def test_two_meta_patterns_triggers(self):
        text = "The claim.guard hook is part of claim.discipline enforcement."
        assert _is_meta_discussion(text) is True

    def test_single_meta_pattern_does_not_trigger(self):
        text = "We need to update the claim guard configuration."
        assert _is_meta_discussion(text) is False

    def test_non_meta_text_does_not_trigger(self):
        text = "The pipeline output is visually broken and needs fixing."
        assert _is_meta_discussion(text) is False


class TestPhaseFactFindings:
    """Phase-specific fact checks for transcript hook."""

    def test_phase_13_complete_regex_matches(self):
        assert PHASE_13_COMPLETE_RE.search("Phase 13 is complete")
        assert PHASE_13_COMPLETE_RE.search("phase 13 has been completed")
        assert not PHASE_13_COMPLETE_RE.search("Phase 14 is complete")

    def test_phase_13_4_missing_regex_matches(self):
        assert PHASE_13_4_MISSING_RE.search("no 13.4 phase exists")
        assert PHASE_13_4_MISSING_RE.search("13.4 does not exist")


class TestExtractAssistantText:
    """Text extraction from assistant records."""

    def test_string_message(self):
        assert _extract_assistant_text({"message": "hello world"}) == "hello world"

    def test_list_message_with_text_blocks(self):
        text = _extract_assistant_text({
            "message": [
                {"type": "text", "text": "first block"},
                {"type": "text", "text": "second block"},
            ]
        })
        assert "first block" in text
        assert "second block" in text

    def test_empty_message_returns_empty(self):
        assert _extract_assistant_text({"message": ""}) == ""
        assert _extract_assistant_text({}) == ""


class TestTailReadJsonl:
    """Efficient tail reading of JSONL files."""

    def test_reads_valid_jsonl(self, tmp_path):
        path = tmp_path / "session.jsonl"
        lines = [
            json.dumps({"type": "assistant", "message": f"msg-{i}"})
            for i in range(5)
        ]
        path.write_text("\n".join(lines) + "\n")

        records = _tail_read_jsonl(path, 65536)
        assert len(records) == 5
        assert records[0]["message"] == "msg-0"

    def test_handles_invalid_json_lines(self, tmp_path):
        path = tmp_path / "session.jsonl"
        path.write_text('{"valid": true}\nnot json\n{"also": "valid"}\n')

        records = _tail_read_jsonl(path, 65536)
        assert len(records) == 2

    def test_tail_limits_bytes_read(self, tmp_path):
        path = tmp_path / "session.jsonl"
        # Write a large file, then read only the tail
        lines = [json.dumps({"idx": i, "pad": "x" * 100}) for i in range(100)]
        path.write_text("\n".join(lines) + "\n")

        # Reading only last 500 bytes should get fewer records
        records = _tail_read_jsonl(path, 500)
        assert len(records) < 100
        assert len(records) > 0
