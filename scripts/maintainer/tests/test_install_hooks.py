"""Tests for maintainer hook installation coverage and verification."""
from __future__ import annotations

import json
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.install_hooks import HOOKS, apply_hooks, verify_hooks


_REQUIRED_TOOLS = {"Bash", "Write", "Edit", "MultiEdit", "Read", "Grep", "Glob", "Task"}


def _matchers_for(prefix: str) -> set[str]:
    return {
        str(hook.get("matcher"))
        for name, hook in HOOKS.items()
        if name.startswith(prefix)
    }


def _build_settings_payload(drop_hook_names: set[str] | None = None) -> dict:
    drop_hook_names = drop_hook_names or set()
    hooks_by_type: dict[str, dict[str, dict]] = {}
    for name, hook in HOOKS.items():
        if name in drop_hook_names:
            continue
        hook_type = str(hook["hook_type"])
        matcher = hook.get("matcher")
        matcher_key = "__none__" if matcher is None else str(matcher)
        type_groups = hooks_by_type.setdefault(hook_type, {})
        group = type_groups.setdefault(
            matcher_key,
            {"matcher": matcher, "hooks": []},
        )
        group["hooks"].append(
            {
                "type": "command",
                "command": f"python3 {hook['script']}",
                "statusMessage": hook["status_message"],
            }
        )

    payload = {"hooks": {}}
    for hook_type, groups in hooks_by_type.items():
        payload["hooks"][hook_type] = list(groups.values())
    return payload


class TestHookCoverage:
    def test_command_handoff_covers_required_matchers(self):
        assert _REQUIRED_TOOLS.issubset(_matchers_for("command_handoff_"))

    def test_startup_gate_covers_required_matchers(self):
        assert _REQUIRED_TOOLS.issubset(_matchers_for("startup_gate_"))

    def test_claim_guard_transcript_covers_required_matchers(self):
        assert _REQUIRED_TOOLS.issubset(_matchers_for("claim_guard_transcript_"))

    def test_claim_guard_content_covers_write_edit_multiedit(self):
        matchers = _matchers_for("claim_guard_content_")
        assert {"Write", "Edit", "MultiEdit"}.issubset(matchers)

    def test_all_hook_scripts_exist_on_disk(self):
        for name, hook in HOOKS.items():
            script = Path(hook["script"])
            assert script.exists(), f"Hook script missing: {name} -> {script}"

    def test_all_hooks_have_required_keys(self):
        required_keys = {"hook_type", "script", "status_message", "description", "verify_fragment"}
        for name, hook in HOOKS.items():
            for key in required_keys:
                assert key in hook, f"Hook {name} missing key: {key}"

    def test_total_hook_count_meets_minimum(self):
        # WS-1 requires >= 31 hooks to cover full tool set
        assert len(HOOKS) >= 31, f"Expected >=31 hooks, got {len(HOOKS)}"


class TestVerifyHooks:
    def test_verify_hooks_passes_when_all_declared_hooks_exist(self, tmp_path, monkeypatch):
        settings_path = tmp_path / ".claude" / "settings.json"
        settings_path.parent.mkdir(parents=True, exist_ok=True)
        settings_path.write_text(
            json.dumps(_build_settings_payload(), indent=2),
            encoding="utf-8",
        )
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        assert verify_hooks() is True

    def test_verify_hooks_fails_when_required_hook_missing(self, tmp_path, monkeypatch):
        settings_path = tmp_path / ".claude" / "settings.json"
        settings_path.parent.mkdir(parents=True, exist_ok=True)
        settings_path.write_text(
            json.dumps(
                _build_settings_payload(drop_hook_names={"command_handoff_read"}),
                indent=2,
            ),
            encoding="utf-8",
        )
        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        assert verify_hooks() is False

    def test_apply_hooks_installs_missing_entries(self, tmp_path, monkeypatch):
        settings_path = tmp_path / ".claude" / "settings.json"
        settings_path.parent.mkdir(parents=True, exist_ok=True)
        settings_path.write_text(json.dumps({"hooks": {}}, indent=2), encoding="utf-8")

        monkeypatch.setattr(Path, "home", lambda: tmp_path)
        assert apply_hooks() is True
        assert verify_hooks() is True
