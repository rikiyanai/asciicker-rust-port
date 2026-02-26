"""Tests for claim_guard_hook — commit message extraction patterns."""
import json
import sys
import tempfile
from pathlib import Path

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.hooks.claim_guard_hook import _extract_commit_message


class TestNonCommitCommands:
    def test_git_push_returns_none(self):
        assert _extract_commit_message("git push origin main") is None

    def test_git_status_returns_none(self):
        assert _extract_commit_message("git status") is None

    def test_git_add_returns_none(self):
        assert _extract_commit_message("git add .") is None

    def test_empty_string_returns_none(self):
        assert _extract_commit_message("") is None

    def test_non_git_command_returns_none(self):
        assert _extract_commit_message("python3 -m pytest tests/") is None

    def test_git_log_returns_none(self):
        assert _extract_commit_message("git log --oneline") is None


class TestBasicDashM:
    def test_double_quoted(self):
        msg = _extract_commit_message('git commit -m "fix: resolved bug"')
        assert msg == "fix: resolved bug"

    def test_single_quoted(self):
        msg = _extract_commit_message("git commit -m 'feat: add feature'")
        assert msg == "feat: add feature"

    def test_with_git_flags_before_m(self):
        msg = _extract_commit_message('git commit -a -m "chore: cleanup"')
        assert msg == "chore: cleanup"

    def test_with_git_flags_after_m(self):
        msg = _extract_commit_message(
            'git commit -m "test: add tests" --no-verify'
        )
        assert msg == "test: add tests"


class TestRepeatedDashM:
    def test_two_m_flags(self):
        msg = _extract_commit_message(
            "git commit -m 'Title line' -m 'Body paragraph'"
        )
        assert msg is not None
        assert "Title line" in msg
        assert "Body paragraph" in msg

    def test_three_m_flags(self):
        msg = _extract_commit_message(
            "git commit -m 'Title' -m 'Body 1' -m 'Body 2'"
        )
        assert msg is not None
        assert "Title" in msg
        assert "Body 1" in msg
        assert "Body 2" in msg


class TestLongFormMessage:
    def test_message_equals(self):
        msg = _extract_commit_message('git commit --message="fix: stuff"')
        assert msg == "fix: stuff"

    def test_message_space(self):
        msg = _extract_commit_message("git commit --message 'fix: stuff'")
        assert msg == "fix: stuff"


class TestHeredoc:
    def test_basic_heredoc(self):
        cmd = (
            "git commit -m \"$(cat <<'EOF'\n"
            "fix(pipeline): resolved everything\n"
            "\n"
            "Co-Authored-By: Someone\n"
            "EOF\n"
            ")\""
        )
        msg = _extract_commit_message(cmd)
        assert msg is not None
        assert "resolved everything" in msg

    def test_heredoc_without_quotes(self):
        cmd = (
            'git commit -m "$(cat <<EOF\n'
            "some message here\n"
            "EOF"
        )
        msg = _extract_commit_message(cmd)
        assert msg is not None
        assert "some message here" in msg


class TestFileFlag:
    def test_dash_f_reads_file(self, tmp_path):
        msg_file = tmp_path / "commit-msg.txt"
        msg_file.write_text("fix: resolved the crash\n\nDetails here.")

        msg = _extract_commit_message(f"git commit -F {msg_file}")
        assert msg is not None
        assert "resolved the crash" in msg

    def test_long_file_flag(self, tmp_path):
        msg_file = tmp_path / "msg.txt"
        msg_file.write_text("feat: new feature")

        msg = _extract_commit_message(f"git commit --file {msg_file}")
        assert msg is not None
        assert "new feature" in msg

    def test_nonexistent_file_returns_none(self):
        msg = _extract_commit_message("git commit -F /nonexistent/path.txt")
        assert msg is None

    def test_quoted_file_path(self, tmp_path):
        msg_file = tmp_path / "msg.txt"
        msg_file.write_text("docs: update readme")

        msg = _extract_commit_message(f'git commit -F "{msg_file}"')
        assert msg is not None
        assert "update readme" in msg


class TestFilePathWithSpaces:
    """Regression: -F with paths containing spaces must work."""

    def test_dash_f_quoted_path_with_spaces(self, tmp_path):
        spaced_dir = tmp_path / "my commit messages"
        spaced_dir.mkdir()
        msg_file = spaced_dir / "msg.txt"
        msg_file.write_text("fix: handle edge case")

        msg = _extract_commit_message(f'git commit -F "{msg_file}"')
        assert msg is not None
        assert "handle edge case" in msg

    def test_file_flag_single_quoted_path_with_spaces(self, tmp_path):
        spaced_dir = tmp_path / "path with spaces"
        spaced_dir.mkdir()
        msg_file = spaced_dir / "commit.txt"
        msg_file.write_text("feat: new widget")

        msg = _extract_commit_message(f"git commit --file '{msg_file}'")
        assert msg is not None
        assert "new widget" in msg


class TestEditorDrivenCommit:
    """Commits without -m or -F open an editor — we can't extract, return None."""

    def test_bare_commit_returns_none(self):
        assert _extract_commit_message("git commit") is None

    def test_commit_amend_no_message_returns_none(self):
        assert _extract_commit_message("git commit --amend") is None

    def test_commit_with_only_a_flag(self):
        assert _extract_commit_message("git commit -a") is None


class TestPriorityOrder:
    """Heredoc should match before -m patterns to avoid partial matches."""

    def test_heredoc_preferred_over_m(self):
        # Heredoc contains the real multi-line message; -m would only get partial
        cmd = (
            "git commit -m \"$(cat <<'EOF'\n"
            "Full multi-line message\n"
            "EOF\n"
            ")\""
        )
        msg = _extract_commit_message(cmd)
        assert "Full multi-line message" in msg
