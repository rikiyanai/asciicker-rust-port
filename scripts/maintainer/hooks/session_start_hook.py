#!/usr/bin/env python3
"""SessionStart hook — run maintainer start-of-session protocol.

Runs the required startup trio:
1) conductor status --auto-setup
2) maintainer hook verification
3) maintainer test suite

Warn-only: prints results to stderr, always exits 0.

Hook interface:
  stdin: JSON context (ignored for SessionStart)
  stdout: JSON result with statusMessage
  exit 0: always (warn-only)
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

_MAINTAINER_DIR = Path(__file__).resolve().parent.parent
_PROJECT_ROOT = _MAINTAINER_DIR.parent.parent
_HOOK_HOME = _PROJECT_ROOT / ".maintainer-hook-home"

_CHECKS = [
    {
        "name": "conductor-status",
        "cmd": [
            sys.executable,
            str(_PROJECT_ROOT / "scripts" / "conductor_tools.py"),
            "status",
            "--auto-setup",
        ],
        "timeout": 15,
    },
    {
        "name": "hooks-verify",
        "cmd": [sys.executable, str(_MAINTAINER_DIR / "install_hooks.py"), "--verify"],
        "timeout": 10,
    },
    {
        "name": "maintainer-tests",
        "cmd": [sys.executable, str(_MAINTAINER_DIR / "run_tests.py")],
        "timeout": 90,
    },
]


def main():
    results = []
    _HOOK_HOME.mkdir(parents=True, exist_ok=True)

    for check in _CHECKS:
        name = check["name"]
        try:
            env = None
            if name == "maintainer-tests":
                env = dict(**os.environ)
                env["HOME"] = str(_HOOK_HOME)
            result = subprocess.run(
                check["cmd"],
                capture_output=True,
                text=True,
                timeout=check["timeout"],
                env=env,
            )
            passed = result.returncode == 0
            stdout_lines = [ln for ln in result.stdout.strip().split("\n") if ln.strip()]
            summary = stdout_lines[-1] if stdout_lines else "no stdout"
            results.append({"name": name, "passed": passed, "summary": summary})

            if passed:
                sys.stderr.write(f"[maintainer:{name}] OK: {summary}\n")
            else:
                err = result.stderr.strip().split("\n")[-1] if result.stderr else "unknown"
                sys.stderr.write(f"[maintainer:{name}] FAIL: {err}\n")

        except subprocess.TimeoutExpired:
            results.append({"name": name, "passed": False, "summary": "timed out"})
            sys.stderr.write(f"[maintainer:{name}] TIMEOUT\n")
        except Exception as e:
            results.append({"name": name, "passed": False, "summary": str(e)})
            sys.stderr.write(f"[maintainer:{name}] ERROR: {e}\n")

    passed_count = sum(1 for r in results if r["passed"])
    total = len(results)

    status = f"Maintainer: {passed_count}/{total} start checks passed"
    sys.stderr.write(f"[maintainer:session-start] {status}\n")

    # Output status message for Claude Code UI
    print(json.dumps({"statusMessage": status}))

    # Always exit 0 — warn-only
    sys.exit(0)


if __name__ == "__main__":
    main()
