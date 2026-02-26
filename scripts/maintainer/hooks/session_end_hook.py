#!/usr/bin/env python3
"""Stop hook — run all session-end maintainer tools.

Runs janitor (full), audit (1 session), goal sanity, and claim telemetry
at session end. Warn-only: always exit 0 regardless of findings.

Hook interface:
  stdin: JSON context (ignored for Stop hooks)
  exit 0: always (warn-only)
"""
from __future__ import annotations

import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

_MAINTAINER_DIR = Path(__file__).resolve().parent.parent
_PROJECT_ROOT = _MAINTAINER_DIR.parent.parent

# Import maintainer libs — guarded so hook still works if libs missing
try:
    sys.path.insert(0, str(_PROJECT_ROOT))
    from scripts.maintainer.lib.jsonl_parser import (
        find_latest_session_jsonl, get_assistant_messages, parse_session_jsonl,
    )
    from scripts.maintainer.claim_guard import check_message
    _CLAIM_LIBS_AVAILABLE = True
except ImportError:
    _CLAIM_LIBS_AVAILABLE = False

# Tools to run at session end, in order
_TOOLS = [
    [sys.executable, str(_MAINTAINER_DIR / "janitor_run.py"), "--mode", "full"],
    [sys.executable, str(_MAINTAINER_DIR / "audit_run.py"), "--sessions", "1"],
    [sys.executable, str(_MAINTAINER_DIR / "goal_sanity.py")],
]


def _collect_claim_telemetry() -> dict | None:
    """Run claim guard on recent session JSONL to collect warning telemetry."""
    if not _CLAIM_LIBS_AVAILABLE:
        return None
    try:
        # Call with no args — lets _resolve_project_dir() auto-resolve
        # to ~/.claude/projects/... where session JSONLs actually live.
        # Passing _PROJECT_ROOT would search the repo root (no JSONLs there).
        session_path = find_latest_session_jsonl()
        if not session_path:
            return None

        parsed = parse_session_jsonl(session_path)
        assistant_msgs = get_assistant_messages(parsed)

        warning_count = 0
        triggered_phrases: dict[str, int] = {}

        for msg in assistant_msgs:
            text = msg.text if hasattr(msg, "text") else str(msg)
            if not text or len(text) < 10:
                continue
            report = check_message(text, mode="warn")
            for finding in report.findings:
                if finding.category == "unsupported_claim":
                    warning_count += 1
                    # Extract the forbidden words from the summary
                    if "[" in finding.summary and "]" in finding.summary:
                        words = finding.summary.split("[")[1].split("]")[0]
                        for word in words.split(", "):
                            word = word.strip()
                            triggered_phrases[word] = triggered_phrases.get(word, 0) + 1

        return {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "session_jsonl": str(session_path),
            "assistant_messages_checked": len(assistant_msgs),
            "unsupported_claim_warnings": warning_count,
            "triggered_phrases": triggered_phrases,
        }
    except Exception:
        return None


def _write_claim_telemetry(telemetry: dict) -> Path | None:
    """Write claim guard telemetry to artifact file."""
    try:
        artifact_dir = _PROJECT_ROOT / "artifacts" / "maintainer"
        artifact_dir.mkdir(parents=True, exist_ok=True)
        ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
        path = artifact_dir / f"claim_telemetry_{ts}.json"
        path.write_text(json.dumps(telemetry, indent=2))
        return path
    except Exception:
        return None


def main():
    for cmd in _TOOLS:
        tool_name = Path(cmd[1]).stem
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if result.stdout:
                sys.stderr.write(f"[maintainer:{tool_name}] {result.stdout.strip()}\n")
            if result.returncode != 0 and result.stderr:
                sys.stderr.write(
                    f"[maintainer:{tool_name}] WARN: {result.stderr.strip()}\n"
                )
        except subprocess.TimeoutExpired:
            sys.stderr.write(f"[maintainer:{tool_name}] WARN: timed out after 30s\n")
        except Exception as e:
            sys.stderr.write(f"[maintainer:{tool_name}] ERROR: {e}\n")

    # Collect and write claim guard telemetry
    telemetry = _collect_claim_telemetry()
    if telemetry:
        path = _write_claim_telemetry(telemetry)
        if path:
            warn_count = telemetry["unsupported_claim_warnings"]
            sys.stderr.write(
                f"[maintainer:claim_telemetry] {warn_count} unsupported claim warnings. "
                f"Telemetry: {path}\n"
            )

    # Always exit 0 — warn-only
    sys.exit(0)


if __name__ == "__main__":
    main()
