"""Parse Claude session JSONL files for maintainer analysis.

Claude Code writes session transcripts as JSONL where each line is a JSON
object with a ``type`` field.  Known types include:

- ``"user"``   — human input (message field contains text or content blocks)
- ``"assistant"`` — Claude response (message field, may contain tool_use)
- ``"progress"``  — progress/status updates
- ``"file-history-snapshot"`` — workspace file state

Unknown record types are preserved with ``record_type="unknown"`` so that
new types added in future Claude versions don't crash the parser.
"""
from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class SessionRecord:
    """A single parsed record from a session JSONL."""
    record_type: str        # "user", "assistant", "progress", "unknown", etc.
    text: str = ""          # extracted human-readable text (empty for non-text)
    raw: dict = field(default_factory=dict)
    line_number: int = 0


@dataclass
class ParsedSession:
    """All records from a single session JSONL file."""
    path: str
    records: list[SessionRecord] = field(default_factory=list)
    parse_errors: list[str] = field(default_factory=list)
    total_lines: int = 0
    skipped_lines: int = 0


def _extract_text_from_message(msg) -> str:
    """Extract human-readable text from a message field.

    The message field can be:
    - A plain string
    - A list of content blocks (each with "type" and "text" or other fields)
    - A dict with a "content" key
    """
    if isinstance(msg, str):
        return msg

    if isinstance(msg, list):
        parts = []
        for block in msg:
            if isinstance(block, dict):
                if block.get("type") == "text":
                    parts.append(block.get("text", ""))
                elif block.get("type") == "tool_result":
                    # Skip tool results — not human text
                    continue
                elif block.get("type") == "tool_use":
                    # Skip tool use blocks
                    continue
            elif isinstance(block, str):
                parts.append(block)
        return "\n".join(parts)

    if isinstance(msg, dict):
        if "content" in msg:
            return _extract_text_from_message(msg["content"])
        if "text" in msg:
            return msg["text"]

    return ""


def _extract_user_text(record: dict) -> str:
    """Extract only human user text, excluding tool_result payloads."""
    msg = record.get("message", "")
    text = _extract_text_from_message(msg)
    return text.strip()


def _extract_assistant_text(record: dict) -> str:
    """Extract assistant text messages, excluding tool_use blocks."""
    msg = record.get("message", "")
    text = _extract_text_from_message(msg)
    return text.strip()


def parse_session_jsonl(path: str | Path) -> ParsedSession:
    """Parse a session JSONL file into structured records.

    Tolerant of malformed lines (logged as parse_errors, not raised).
    Unknown record types are kept with record_type="unknown".
    """
    path = Path(path)
    session = ParsedSession(path=str(path))

    if not path.exists():
        session.parse_errors.append(f"File not found: {path}")
        return session

    with open(path, "r", encoding="utf-8", errors="replace") as f:
        for line_num, line in enumerate(f, start=1):
            session.total_lines += 1
            line = line.strip()
            if not line:
                session.skipped_lines += 1
                continue

            try:
                obj = json.loads(line)
            except json.JSONDecodeError as exc:
                session.parse_errors.append(
                    f"Line {line_num}: JSON decode error: {exc}"
                )
                session.skipped_lines += 1
                continue

            if not isinstance(obj, dict):
                session.parse_errors.append(
                    f"Line {line_num}: Expected dict, got {type(obj).__name__}"
                )
                session.skipped_lines += 1
                continue

            rec_type = obj.get("type", "unknown")
            text = ""

            if rec_type == "user":
                text = _extract_user_text(obj)
            elif rec_type == "assistant":
                text = _extract_assistant_text(obj)
            # progress, file-history-snapshot, and unknown: no text extraction

            session.records.append(SessionRecord(
                record_type=str(rec_type),
                text=text,
                raw=obj,
                line_number=line_num,
            ))

    return session


def get_user_messages(session: ParsedSession) -> list[SessionRecord]:
    """Return only human user text records (non-empty)."""
    return [r for r in session.records
            if r.record_type == "user" and r.text]


def get_assistant_messages(session: ParsedSession) -> list[SessionRecord]:
    """Return only assistant text records (non-empty)."""
    return [r for r in session.records
            if r.record_type == "assistant" and r.text]


def _resolve_project_dir(
    project_dir: str | Path | None = None,
) -> Optional[Path]:
    """Resolve the Claude Code project sessions directory.

    Priority:
    1. Explicit project_dir argument
    2. Current repo path encoded in Claude's project key format
    3. Most recently modified project dir (fallback)
    """
    if project_dir:
        return Path(project_dir)

    claude_dir = Path.home() / ".claude" / "projects"
    if not claude_dir.exists():
        return None

    # Try to match current repo path to a project dir key.
    # Claude encodes project paths as: -Users-r-Downloads-project-name
    # (leading slash removed, slashes and spaces replaced with dashes).
    try:
        cwd = Path.cwd().resolve()
        cwd_key = str(cwd).replace("/", "-").replace(" ", "-").lstrip("-")
        for d in claude_dir.iterdir():
            if d.is_dir() and d.name.startswith("-") and cwd_key in d.name:
                return d
    except Exception:
        pass

    # Fallback: most recently modified project dir
    project_dirs = [d for d in claude_dir.iterdir() if d.is_dir()]
    if not project_dirs:
        return None
    return max(project_dirs, key=lambda d: d.stat().st_mtime)


def find_latest_session_jsonl(
    project_dir: str | Path | None = None,
) -> Optional[Path]:
    """Find the most recent session JSONL by mtime."""
    search_dir = _resolve_project_dir(project_dir)
    if not search_dir or not search_dir.exists():
        return None

    jsonl_files = list(search_dir.glob("*.jsonl"))
    if not jsonl_files:
        return None

    return max(jsonl_files, key=lambda f: f.stat().st_mtime)


def find_recent_session_jsonls(
    n: int = 1,
    project_dir: str | Path | None = None,
) -> list[Path]:
    """Find the N most recent session JSONLs by mtime, newest first."""
    search_dir = _resolve_project_dir(project_dir)
    if not search_dir or not search_dir.exists():
        return []

    jsonl_files = list(search_dir.glob("*.jsonl"))
    if not jsonl_files:
        return []

    jsonl_files.sort(key=lambda f: f.stat().st_mtime, reverse=True)
    return jsonl_files[:n]
