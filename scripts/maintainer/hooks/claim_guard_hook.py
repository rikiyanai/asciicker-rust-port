#!/usr/bin/env python3
"""PreToolUse:Bash hook — block unsupported claims in git commit messages.

Reads Claude Code hook stdin JSON, extracts commit message from git commit -m,
runs claim_guard analysis. Hard-block mode: exits 2 on unsupported claims.

Hook interface:
  stdin: {"tool_name": "Bash", "tool_input": {"command": "..."}, "transcript": "..."}
  stdout: {"blocked": true, "message": "..."} to block
  exit 0: allow, exit 2: block
"""
from __future__ import annotations

import json
import re
import shlex
import sys
from pathlib import Path

# Add parent to path for claim_guard imports
_MAINTAINER_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_MAINTAINER_DIR.parent.parent))


def _extract_commit_message(cmd: str) -> str | None:
    """Extract commit message from a git commit command string.

    Handles:
      - git commit -m "message" / -m 'message'
      - git commit --message="message" / --message 'message'
      - git commit -m "$(cat <<'EOF'\\nmessage\\nEOF\\n)"  (heredoc)
      - git commit -F <file>  / --file <file>  (returns placeholder — file not read)
      - Multiple -m flags: git commit -m "line1" -m "line2" (concatenated)

    Returns None for non-commit commands or editor-driven commits (no -m/-F).
    """
    m = re.search(r'git\s+commit\b', cmd)
    if not m:
        return None

    # Pattern: heredoc style -m "$(cat <<'EOF'\n...\nEOF\n)"
    m_heredoc = re.search(r"-m\s+\"\$\(cat\s+<<'?EOF'?\n(.+?)\nEOF", cmd, re.DOTALL)
    if m_heredoc:
        return m_heredoc.group(1)

    # Pattern: -F / --file <path> — use shlex to handle quoted paths with spaces.
    # Falls back to regex if shlex fails (e.g. unfinished heredoc in cmd string).
    try:
        tokens = shlex.split(cmd)
    except ValueError:
        tokens = []
    for i, tok in enumerate(tokens):
        if tok in ("-F", "--file") and i + 1 < len(tokens):
            file_path = tokens[i + 1]
            try:
                return Path(file_path).expanduser().read_text()
            except (OSError, ValueError):
                return None

    # Pattern: --message="msg" or --message='msg' or --message msg
    m_long = re.search(r'''--message[= ]["'](.+?)["']''', cmd, re.DOTALL)
    if m_long:
        return m_long.group(1)

    # Pattern: repeated -m flags → concatenate with newlines (git behavior)
    parts = re.findall(r'''-m\s+["'](.+?)["']''', cmd, re.DOTALL)
    if parts:
        return "\n\n".join(parts)

    return None


def main():
    try:
        data = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)

    if data.get("tool_name") != "Bash":
        sys.exit(0)

    cmd = data.get("tool_input", {}).get("command", "")
    message = _extract_commit_message(cmd)
    if not message:
        sys.exit(0)

    # Import claim_guard logic
    try:
        from scripts.maintainer.claim_guard import check_message
    except ImportError:
        # Fallback: direct import
        sys.path.insert(0, str(_MAINTAINER_DIR))
        from claim_guard import check_message

    report = check_message(message)

    if not report.findings:
        sys.exit(0)

    unsupported = [
        f for f in report.findings if f.category == "unsupported_claim"
    ]
    if unsupported:
        detail = unsupported[0].summary
        msg = (
            f"[maintainer:claim-guard] BLOCKED: {detail} "
            f"Add evidence refs (FL-NNN, commit hash) or rephrase."
        )
        sys.stderr.write(msg + "\n")
        print(json.dumps({"blocked": True, "message": msg}))
        sys.exit(2)

    sys.exit(0)


if __name__ == "__main__":
    main()
