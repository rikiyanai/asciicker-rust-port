#!/usr/bin/env python3
"""PreToolUse hook — block tool use when recent assistant output has unsupported claims.

Reads the current session JSONL (live on disk), extracts the last few assistant
messages, and runs claim_guard on them. Blocks the next tool use if forbidden
status words appear without evidence (FL-NNN, commit hash) nearby.

Recovery: include an FL-NNN or commit hash in your next response. The evidence
appears in the recent transcript window and the hook allows the tool use.

Hook interface:
  stdin: {"tool_name": "...", "tool_input": {...}, ...}
  stdout: {"blocked": true, "message": "..."} to block
  exit 0: allow, exit 2: block
"""
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

_MAINTAINER_DIR = Path(__file__).resolve().parent.parent
_PROJECT_ROOT = _MAINTAINER_DIR.parent.parent
sys.path.insert(0, str(_PROJECT_ROOT))

try:
    from scripts.maintainer.lib.report_schema import FORBIDDEN_STATUS_WORDS
    from scripts.maintainer.lib.jsonl_parser import (
        find_latest_session_jsonl,
    )
    _LIBS_AVAILABLE = True
except ImportError:
    _LIBS_AVAILABLE = False

# Evidence patterns
FL_REF = re.compile(r"\bFL-\d{3,4}\b")
COMMIT_REF = re.compile(r"\b[0-9a-f]{7,40}\b")

# How many bytes from the end of JSONL to read (perf: avoid parsing entire file)
TAIL_BYTES = 65536  # 64KB — covers ~10-20 recent messages

# Max assistant messages to check
MAX_MESSAGES = 3

# Skip if assistant text is clearly meta-discussion about claim guard itself
META_PATTERNS = [
    r"claim.guard",
    r"forbidden.word",
    r"maintainer.hook",
    r"CLM-\d{3}",
    r"claim.discipline",
    r"FORBIDDEN_STATUS_WORDS",
]

# Phase fact-check patterns
PHASE_13_COMPLETE_RE = re.compile(r"\bphase\s*13\b.*\b(complete|completed)\b", re.IGNORECASE | re.DOTALL)
PHASE_13_4_MISSING_RE = re.compile(
    r"\b(?:no|not)\b.{0,40}\b13\.4\b.{0,40}\b(?:exist|planned|plan|phase)\b|\b13\.4\b.{0,40}\b(?:does not|doesn't|not)\b.{0,40}\bexist\b",
    re.IGNORECASE | re.DOTALL,
)


def _phase_13_marked_incomplete() -> bool:
    """Return True when ROADMAP marks Phase 13 as not complete/failed."""
    roadmap = _PROJECT_ROOT / ".planning" / "ROADMAP.md"
    if not roadmap.exists():
        return False
    try:
        text = roadmap.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return False
    # Canonical status row currently uses unchecked box + FAILED text.
    return bool(re.search(r"^- \[ \]\s+\*\*Phase 13:.*FAILED", text, re.MULTILINE))


def _phase_13_4_exists() -> bool:
    phases_root = _PROJECT_ROOT / ".planning" / "phases"
    if not phases_root.exists():
        return False
    return any(p.name.startswith("13.4") for p in phases_root.iterdir() if p.is_dir())


def _phase_fact_findings(text: str) -> list[str]:
    findings: list[str] = []
    if PHASE_13_COMPLETE_RE.search(text) and _phase_13_marked_incomplete():
        findings.append("phase13_claimed_complete_but_roadmap_incomplete")
    if PHASE_13_4_MISSING_RE.search(text) and _phase_13_4_exists():
        findings.append("phase13_4_claimed_missing_but_directory_exists")
    return findings


def _tail_read_jsonl(path: Path, tail_bytes: int) -> list[dict]:
    """Read the last tail_bytes of a JSONL file and parse valid lines."""
    size = path.stat().st_size
    offset = max(0, size - tail_bytes)

    records = []
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        if offset > 0:
            f.seek(offset)
            f.readline()  # discard partial first line
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
                if isinstance(obj, dict):
                    records.append(obj)
            except json.JSONDecodeError:
                continue
    return records


def _extract_assistant_text(record: dict) -> str:
    """Extract text from an assistant record."""
    msg = record.get("message", "")
    if isinstance(msg, str):
        return msg.strip()
    if isinstance(msg, list):
        parts = []
        for block in msg:
            if isinstance(block, dict) and block.get("type") == "text":
                parts.append(block.get("text", ""))
            elif isinstance(block, str):
                parts.append(block)
        return "\n".join(parts).strip()
    if isinstance(msg, dict) and "content" in msg:
        return _extract_assistant_text({"message": msg["content"]})
    return ""


def _is_meta_discussion(text: str) -> bool:
    """Check if text is discussing claim guard itself (meta-context)."""
    lower = text.lower()
    matches = sum(1 for p in META_PATTERNS if re.search(p, lower))
    return matches >= 2  # at least 2 meta-patterns = likely discussing the system


def main():
    if not _LIBS_AVAILABLE:
        sys.exit(0)

    try:
        data = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)

    # Find current session JSONL
    session_path = find_latest_session_jsonl()
    if not session_path or not session_path.exists():
        sys.exit(0)

    # Efficient tail read
    records = _tail_read_jsonl(session_path, TAIL_BYTES)

    # Extract recent assistant messages
    assistant_texts = []
    for rec in records:
        if rec.get("type") == "assistant":
            text = _extract_assistant_text(rec)
            if text and len(text) > 10:
                assistant_texts.append(text)

    if not assistant_texts:
        sys.exit(0)

    # Check last N assistant messages
    recent = assistant_texts[-MAX_MESSAGES:]
    combined = "\n".join(recent)

    # Skip meta-discussion about claim guard
    if _is_meta_discussion(combined):
        sys.exit(0)

    # Search for forbidden words (word-boundary, hyphen-compound immune)
    found_forbidden = []
    for word in FORBIDDEN_STATUS_WORDS:
        if re.search(rf"(?<!-)\b{re.escape(word)}\b(?!-)", combined.lower()):
            found_forbidden.append(word)

    if not found_forbidden:
        # No forbidden words; still run factual phase checks.
        phase_findings = _phase_fact_findings(combined)
        if not phase_findings:
            sys.exit(0)
        msg = (
            "[maintainer:claim-guard-transcript] BLOCKED: "
            "Recent assistant output contains factual phase-status mismatch "
            f"[{', '.join(phase_findings)}]. "
            "Use current roadmap/phase evidence before phase-completion or phase-existence claims."
        )
        sys.stderr.write(msg + "\n")
        print(json.dumps({"blocked": True, "message": msg}))
        sys.exit(2)

    # Check for evidence in the same window
    has_fl = bool(FL_REF.search(combined))
    has_commit = bool(COMMIT_REF.search(combined))

    if has_fl or has_commit:
        # Evidence present — still enforce factual phase claims.
        phase_findings = _phase_fact_findings(combined)
        if not phase_findings:
            sys.exit(0)
        msg = (
            "[maintainer:claim-guard-transcript] BLOCKED: "
            "Recent assistant output contains factual phase-status mismatch "
            f"[{', '.join(phase_findings)}] despite evidence refs. "
            "Update claim to match current .planning/ROADMAP.md and .planning/phases/."
        )
        sys.stderr.write(msg + "\n")
        print(json.dumps({"blocked": True, "message": msg}))
        sys.exit(2)

    # Block: forbidden words without evidence in recent assistant output
    msg = (
        f"[maintainer:claim-guard-transcript] BLOCKED: "
        f"Recent assistant output contains unsupported status words "
        f"[{', '.join(found_forbidden)}] without evidence refs.\n"
        f"Recovery: include an FL-NNN or commit hash in your next response "
        f"to justify the claim, or rephrase using PARTIAL/MONITORING vocabulary."
    )
    sys.stderr.write(msg + "\n")
    print(json.dumps({"blocked": True, "message": msg}))
    sys.exit(2)


if __name__ == "__main__":
    main()
