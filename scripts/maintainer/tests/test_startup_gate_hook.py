"""Tests for startup_gate_hook session-key derivation."""
from __future__ import annotations

import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.hooks.startup_gate_hook import _session_key


class TestSessionKey:
    def test_prefers_explicit_session_id(self):
        key = _session_key({"session_id": "session-123"})
        assert len(key) == 40  # full sha1 digest
        assert key.startswith("fallback-") is False

    def test_uses_transcript_when_available(self):
        key = _session_key({"transcript": "hello world"})
        assert len(key) == 40
        assert key.startswith("fallback-") is False

    def test_uses_fallback_bucket_when_no_identifiers(self):
        key = _session_key({})
        assert key.startswith("fallback-")
