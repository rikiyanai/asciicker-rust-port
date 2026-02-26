"""Tests for claim_guard_content_hook — file content claim blocking."""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.hooks.claim_guard_content_hook import (
    WATCHED_PATTERNS,
    SKIP_PATTERNS,
    main,
)


class TestWatchedPatterns:
    """Content hook should watch key file types."""

    @pytest.mark.parametrize("ext", [".md", ".py", ".ts", ".js", ".json", ".rs", ".toml"])
    def test_watched_extensions_covered(self, ext):
        import re
        matches = any(re.search(p, f"foo{ext}") for p in WATCHED_PATTERNS)
        assert matches, f"{ext} should be watched"

    @pytest.mark.parametrize("ext", [".png", ".xp", ".bin", ".wasm"])
    def test_binary_extensions_not_watched(self, ext):
        import re
        matches = any(re.search(p, f"foo{ext}") for p in WATCHED_PATTERNS)
        assert not matches, f"{ext} should NOT be watched"


class TestSkipPatterns:
    """Content hook should skip maintainer infrastructure files."""

    @pytest.mark.parametrize("path", [
        "scripts/maintainer/claim_guard.py",
        "scripts/maintainer/tests/test_claim_guard.py",
        "scripts/maintainer/POLICY.md",
        "docs/research/ascii/verification/FAILURE_LOG.md",
    ])
    def test_maintainer_files_skipped(self, path):
        import re
        matches = any(re.search(p, path) for p in SKIP_PATTERNS)
        assert matches, f"{path} should be skipped"


class TestContentHookAcceptance:
    """Integration tests for content hook main() via subprocess stdin."""

    def test_allows_write_without_forbidden_words(self, monkeypatch, capsys):
        payload = json.dumps({
            "tool_name": "Write",
            "tool_input": {
                "file_path": "docs/test.md",
                "content": "This is a normal document with no claims.",
            },
        })
        monkeypatch.setattr("sys.stdin", __import__("io").StringIO(payload))
        with pytest.raises(SystemExit) as exc_info:
            main()
        assert exc_info.value.code == 0

    def test_allows_edit_to_non_watched_file(self, monkeypatch, capsys):
        payload = json.dumps({
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "assets/sprite.xp",
                "new_string": "resolved everything completely",
            },
        })
        monkeypatch.setattr("sys.stdin", __import__("io").StringIO(payload))
        with pytest.raises(SystemExit) as exc_info:
            main()
        assert exc_info.value.code == 0

    def test_allows_write_to_skipped_path(self, monkeypatch, capsys):
        payload = json.dumps({
            "tool_name": "Write",
            "tool_input": {
                "file_path": "scripts/maintainer/POLICY.md",
                "content": "The pipeline is fixed and resolved and complete.",
            },
        })
        monkeypatch.setattr("sys.stdin", __import__("io").StringIO(payload))
        with pytest.raises(SystemExit) as exc_info:
            main()
        assert exc_info.value.code == 0

    def test_allows_non_write_tool(self, monkeypatch, capsys):
        payload = json.dumps({
            "tool_name": "Bash",
            "tool_input": {
                "command": "echo resolved",
            },
        })
        monkeypatch.setattr("sys.stdin", __import__("io").StringIO(payload))
        with pytest.raises(SystemExit) as exc_info:
            main()
        assert exc_info.value.code == 0

    def test_allows_short_content(self, monkeypatch, capsys):
        payload = json.dumps({
            "tool_name": "Write",
            "tool_input": {
                "file_path": "docs/test.md",
                "content": "done",
            },
        })
        monkeypatch.setattr("sys.stdin", __import__("io").StringIO(payload))
        with pytest.raises(SystemExit) as exc_info:
            main()
        assert exc_info.value.code == 0
