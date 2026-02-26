#!/usr/bin/env python3
"""PreToolUse hook — enforce start-of-session checks before work.

This hook auto-runs the startup trio once per session-like transcript key:
1) conductor status --auto-setup
2) maintainer hook verify
3) maintainer maintainer-tests

If checks fail, it hard-blocks tool use with actionable guidance.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

_MAINTAINER_DIR = Path(__file__).resolve().parent.parent
_PROJECT_ROOT = _MAINTAINER_DIR.parent.parent
_ARTIFACT_DIR = _PROJECT_ROOT / "artifacts" / "maintainer"
_STATE_PATH = _ARTIFACT_DIR / "startup_gate_state.json"
_HOOK_HOME = _PROJECT_ROOT / ".maintainer-hook-home"

_ALLOWED_TOOLS = {"Bash", "Write", "Edit", "MultiEdit", "Read", "Grep", "Glob", "Task"}

_STARTUP_PATTERNS = [
    re.compile(r"python3\s+(?:\S+/)?scripts/conductor_tools\.py\s+status\s+--auto-setup"),
    re.compile(r"python3\s+(?:\S+/)?scripts/maintainer/install_hooks\.py\s+--verify"),
    re.compile(r"python3\s+(?:\S+/)?scripts/maintainer/run_tests\.py(?:\s|$)"),
    re.compile(r"python3\s+-m\s+pytest\s+scripts/maintainer/tests\b"),
]

_CHECKS = [
    {
        "name": "conductor-status",
        "cmd": [sys.executable, "scripts/conductor_tools.py", "status", "--auto-setup"],
        "timeout": 20,
    },
    {
        "name": "hooks-verify",
        "cmd": [sys.executable, "scripts/maintainer/install_hooks.py", "--verify"],
        "timeout": 20,
    },
    {
        "name": "maintainer-tests",
        "cmd": [sys.executable, "scripts/maintainer/run_tests.py"],
        "timeout": int(os.environ.get("MAINTAINER_STARTUP_TEST_TIMEOUT", "120")),
    },
]


def _read_state() -> dict[str, Any]:
    if not _STATE_PATH.exists():
        return {"sessions_ok": {}}
    try:
        data = json.loads(_STATE_PATH.read_text())
    except (json.JSONDecodeError, OSError):
        return {"sessions_ok": {}}
    if not isinstance(data, dict):
        return {"sessions_ok": {}}
    sessions = data.get("sessions_ok")
    if not isinstance(sessions, dict):
        sessions = {}
    return {"sessions_ok": sessions}


def _write_state(state: dict[str, Any]) -> None:
    _ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)
    _STATE_PATH.write_text(json.dumps(state, indent=2))


def _session_key(data: dict[str, Any]) -> str:
    """Build a session key with resilient fallback when transcript is unavailable."""
    # Prefer explicit identifiers if present in hook payload.
    for key in ("session_id", "conversation_id", "thread_id", "request_id"):
        raw = data.get(key)
        if raw is not None:
            value = str(raw).strip()
            if value:
                return hashlib.sha1(f"{key}:{value}".encode("utf-8")).hexdigest()

    transcript = data.get("transcript", "")
    if isinstance(transcript, str) and transcript.strip():
        head = transcript[:4000]
        return hashlib.sha1(head.encode("utf-8", errors="ignore")).hexdigest()

    # Last resort when transcript is absent: rotate every 15 minutes.
    now = datetime.now(timezone.utc)
    bucket = now.minute // 15
    fallback = now.strftime("%Y%m%dT%H") + f"-q{bucket}"
    return f"fallback-{fallback}"


def _is_startup_command(command: str) -> bool:
    if not command:
        return False
    return any(rx.search(command) for rx in _STARTUP_PATTERNS)


def _tail(text: str) -> str:
    lines = [ln for ln in (text or "").splitlines() if ln.strip()]
    return lines[-1] if lines else "no output"


def _run_startup_trio() -> tuple[bool, list[dict[str, str]]]:
    results: list[dict[str, str]] = []
    ok = True
    _HOOK_HOME.mkdir(parents=True, exist_ok=True)
    for check in _CHECKS:
        name = check["name"]
        try:
            env = os.environ.copy()
            if name == "maintainer-tests":
                # Keep maintainer tests deterministic in sandboxed contexts where
                # HOME/Downloads may be outside writable roots.
                env["HOME"] = str(_HOOK_HOME)
            proc = subprocess.run(
                check["cmd"],
                capture_output=True,
                text=True,
                timeout=check["timeout"],
                cwd=str(_PROJECT_ROOT),
                env=env,
            )
            passed = proc.returncode == 0
            summary = _tail(proc.stdout if passed else proc.stderr or proc.stdout)
            results.append({
                "name": name,
                "status": "ok" if passed else "fail",
                "summary": summary,
            })
            if not passed:
                ok = False
        except subprocess.TimeoutExpired:
            ok = False
            results.append({
                "name": name,
                "status": "timeout",
                "summary": f"timed out after {check['timeout']}s",
            })
        except Exception as exc:
            ok = False
            results.append({
                "name": name,
                "status": "error",
                "summary": str(exc),
            })
    return ok, results


def main() -> None:
    try:
        data = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)

    tool_name = data.get("tool_name", "")
    if tool_name not in _ALLOWED_TOOLS:
        sys.exit(0)

    tool_input = data.get("tool_input", {}) or {}
    command = tool_input.get("command", "") if isinstance(tool_input, dict) else ""

    # Never block explicit startup commands; allow user/operator to run them manually.
    if tool_name == "Bash" and _is_startup_command(command):
        sys.exit(0)

    key = _session_key(data if isinstance(data, dict) else {})

    state = _read_state()
    sessions_ok = state.get("sessions_ok", {})
    if key in sessions_ok:
        sys.exit(0)

    ok, results = _run_startup_trio()
    now = datetime.now(timezone.utc).isoformat()
    if ok:
        sessions_ok[key] = {
            "checked_at": now,
            "results": results,
        }
        # Keep state file bounded.
        if len(sessions_ok) > 50:
            for old_key in list(sessions_ok.keys())[: len(sessions_ok) - 50]:
                sessions_ok.pop(old_key, None)
        state["sessions_ok"] = sessions_ok
        _write_state(state)
        sys.exit(0)

    detail = "; ".join(f"{r['name']}={r['status']} ({r['summary']})" for r in results)
    msg = (
        "[maintainer:startup-gate] BLOCKED: start-of-session protocol failed. "
        f"{detail}. "
        "Run: "
        "`python3 scripts/conductor_tools.py status --auto-setup` ; "
        "`python3 scripts/maintainer/install_hooks.py --verify` ; "
        "`python3 scripts/maintainer/run_tests.py`."
    )
    sys.stderr.write(msg + "\n")
    print(json.dumps({"blocked": True, "message": msg}))
    sys.exit(2)


if __name__ == "__main__":
    main()
