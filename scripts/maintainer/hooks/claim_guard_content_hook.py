#!/usr/bin/env python3
"""PreToolUse:Write+Edit+MultiEdit hook — block forbidden claim words in file content.

Extends claim guard beyond git commits to file content being written or edited.
Checks the content/new_string for forbidden status words without evidence refs
in the same content or recent transcript.

Hook interface:
  stdin: {"tool_name": "Write"|"Edit"|"MultiEdit", "tool_input": {...}, "transcript": "..."}
  stdout: {"blocked": true, "message": "..."} to block
  exit 0: allow, exit 2: block
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

_MAINTAINER_DIR = Path(__file__).resolve().parent.parent
_PROJECT_ROOT = _MAINTAINER_DIR.parent.parent
sys.path.insert(0, str(_PROJECT_ROOT))

try:
    from scripts.maintainer.claim_guard import check_message
except ImportError:
    sys.path.insert(0, str(_MAINTAINER_DIR))
    from claim_guard import check_message

# Only check these file patterns — skip generated/binary/config files
WATCHED_PATTERNS = [
    r"\.md$",
    r"\.rs$",
    r"\.toml$",
    r"\.py$",
    r"\.ts$",
    r"\.js$",
    r"\.json$",
]

# Skip files where claim words are expected (e.g., the claim guard itself)
SKIP_PATTERNS = [
    r"claim_guard",
    r"maintainer/",
    r"POLICY\.md$",
    r"FAILURE_LOG\.md$",
    r"node_modules/",
    r"\.claude/hooks/",
]


def main():
    try:
        data = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)

    tool_name = data.get("tool_name", "")
    if tool_name not in ("Write", "Edit", "MultiEdit"):
        sys.exit(0)

    tool_input = data.get("tool_input", {})
    file_path = tool_input.get("file_path", "")

    # Only check watched file types
    if not any(re.search(p, file_path) for p in WATCHED_PATTERNS):
        sys.exit(0)

    # Skip files where claim words are expected
    if any(re.search(p, file_path) for p in SKIP_PATTERNS):
        sys.exit(0)

    # Extract content being written
    content = ""
    if tool_name == "Write":
        content = tool_input.get("content", "")
    elif tool_name == "Edit":
        content = tool_input.get("new_string", "")
    elif tool_name == "MultiEdit":
        # MultiEdit payload contains multiple edit operations against one file.
        edits = tool_input.get("edits", [])
        chunks = []
        if isinstance(edits, list):
            for edit in edits:
                if isinstance(edit, dict):
                    new_string = edit.get("new_string", "")
                    if isinstance(new_string, str) and new_string.strip():
                        chunks.append(new_string)
        content = "\n".join(chunks)

    if not content or len(content) < 10:
        sys.exit(0)

    # Run claim guard on the content
    report = check_message(content, mode="block")

    unsupported = [
        f for f in report.findings if f.category == "unsupported_claim"
    ]
    if not unsupported:
        sys.exit(0)

    # Check transcript for evidence that might justify the claim
    transcript = data.get("transcript", "")
    recent = transcript[-8000:] if len(transcript) > 8000 else transcript

    fl_ref = bool(re.search(r"\bFL-\d{3,4}\b", recent))
    commit_ref = bool(re.search(r"\b[0-9a-f]{7,40}\b", recent))

    if fl_ref or commit_ref:
        # Evidence exists in recent transcript — allow
        sys.exit(0)

    detail = unsupported[0].summary
    msg = (
        f"[maintainer:claim-guard-content] BLOCKED: {detail}\n"
        f"File: {file_path}\n"
        f"Add evidence refs (FL-NNN, commit hash) to justify status claims, "
        f"or use PARTIAL/MONITORING vocabulary instead."
    )
    sys.stderr.write(msg + "\n")
    print(json.dumps({"blocked": True, "message": msg}))
    sys.exit(2)


if __name__ == "__main__":
    main()
