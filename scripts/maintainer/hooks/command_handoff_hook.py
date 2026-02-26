#!/usr/bin/env python3
"""PreToolUse hook — write command-level handoff artifacts.

Generates:
- One session handoff directory per transcript-derived session key.
- One command JSONL record per tool invocation.
- One command markdown handoff file per invocation.
- A session-level rolling handoff index.

This hook is logging-only and never blocks tool execution.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

_MAINTAINER_DIR = Path(__file__).resolve().parent.parent
_PROJECT_ROOT = _MAINTAINER_DIR.parent.parent
_BASE_DIR = _PROJECT_ROOT / "artifacts" / "maintainer" / "handoffs"
_INDEX_PATH = _BASE_DIR / "INDEX.md"
_ALLOWED_TOOLS = {
    "Bash", "Write", "Edit", "MultiEdit",
    "Read", "Grep", "Glob", "Task",
}


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _slug(text: str, limit: int = 48) -> str:
    cleaned = re.sub(r"[^a-zA-Z0-9._-]+", "-", text.strip())
    cleaned = cleaned.strip("-._")
    if not cleaned:
        cleaned = "item"
    return cleaned[:limit]


def _session_key(data: dict[str, Any]) -> str:
    # Prefer explicit IDs when available from hook payload.
    for key in ("session_id", "conversation_id", "thread_id", "request_id"):
        raw = data.get(key)
        if raw is not None:
            value = str(raw).strip()
            if value:
                return hashlib.sha1(f"{key}:{value}".encode("utf-8")).hexdigest()[:12]

    transcript = data.get("transcript", "")
    if isinstance(transcript, str) and transcript.strip():
        head = transcript[:4000]
        return hashlib.sha1(head.encode("utf-8", errors="ignore")).hexdigest()[:12]

    # Last resort: rotate fallback key every 15 minutes to avoid cross-session collapse.
    now = datetime.now(timezone.utc)
    bucket = now.minute // 15
    fallback = now.strftime("%Y%m%dT%H") + f"-q{bucket}"
    return f"fallback-{fallback}"


def _preview(text: str, limit: int = 240) -> str:
    compact = " ".join((text or "").split())
    if len(compact) <= limit:
        return compact
    return compact[: limit - 3] + "..."


def _command_payload(tool_name: str, tool_input: dict[str, Any]) -> dict[str, Any]:
    payload: dict[str, Any] = {}
    if tool_name == "Bash":
        cmd = str(tool_input.get("command", ""))
        payload["command"] = cmd
        payload["preview"] = _preview(cmd, 300)
    elif tool_name == "Write":
        content = str(tool_input.get("content", ""))
        payload["file_path"] = str(tool_input.get("file_path", ""))
        payload["content_len"] = len(content)
        payload["content_preview"] = _preview(content)
    elif tool_name == "Edit":
        new_string = str(tool_input.get("new_string", ""))
        old_string = str(tool_input.get("old_string", ""))
        payload["file_path"] = str(tool_input.get("file_path", ""))
        payload["old_len"] = len(old_string)
        payload["new_len"] = len(new_string)
        payload["new_preview"] = _preview(new_string)
    elif tool_name == "MultiEdit":
        file_path = str(tool_input.get("file_path", ""))
        edits = tool_input.get("edits", [])
        edit_count = len(edits) if isinstance(edits, list) else 0
        previews: list[str] = []
        if isinstance(edits, list):
            for edit in edits[:3]:
                if isinstance(edit, dict):
                    previews.append(_preview(str(edit.get("new_string", "")), 120))
        payload["file_path"] = file_path
        payload["edit_count"] = edit_count
        payload["preview"] = " | ".join([p for p in previews if p])
    elif tool_name in {"Read", "Grep", "Glob", "Task"}:
        payload["tool_input_keys"] = sorted(tool_input.keys())
        for key in ("file_path", "path", "pattern", "query", "description", "prompt"):
            if key in tool_input:
                payload[key] = str(tool_input.get(key, ""))
        payload["preview"] = _preview(
            json.dumps(tool_input, sort_keys=True, default=str),
            300,
        )
    return payload


def _load_session_meta(session_dir: Path) -> dict[str, Any]:
    meta_path = session_dir / "meta.json"
    if not meta_path.exists():
        return {"seq": 0, "created_at": _now_iso()}
    try:
        data = json.loads(meta_path.read_text())
    except (json.JSONDecodeError, OSError):
        return {"seq": 0, "created_at": _now_iso()}
    if not isinstance(data, dict):
        return {"seq": 0, "created_at": _now_iso()}
    if not isinstance(data.get("seq"), int):
        data["seq"] = 0
    if not isinstance(data.get("created_at"), str):
        data["created_at"] = _now_iso()
    return data


def _save_session_meta(session_dir: Path, meta: dict[str, Any]) -> None:
    (session_dir / "meta.json").write_text(json.dumps(meta, indent=2))


def _append_index(session_key: str, session_dir: Path) -> None:
    rel = session_dir.relative_to(_PROJECT_ROOT)
    if not _INDEX_PATH.exists():
        _INDEX_PATH.write_text("# Session Handoffs\n\n")
    current = _INDEX_PATH.read_text()
    needle = f"`{session_key}`"
    if needle in current:
        return
    with _INDEX_PATH.open("a") as f:
        f.write(f"- Session `{session_key}`: `{rel}`\n")


def _append_session_rollup(session_dir: Path, record: dict[str, Any], cmd_file: Path) -> None:
    rollup = session_dir / "SESSION_HANDOFF.md"
    if not rollup.exists():
        rollup.write_text(
            "# Session Handoff\n\n"
            f"- Session key: `{record['session_key']}`\n"
            f"- Created at: `{record['timestamp']}`\n\n"
            "## Commands\n\n"
        )
    rel_cmd = cmd_file.name
    line = (
        f"- `{record['seq']:04d}` `{record['timestamp']}` `{record['tool_name']}` "
        f"`{record.get('preview','')}` -> `{rel_cmd}`\n"
    )
    with rollup.open("a") as f:
        f.write(line)


def _write_command_markdown(session_dir: Path, record: dict[str, Any]) -> Path:
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
    suffix = _slug(record["tool_name"].lower(), 16)
    cmd_file = session_dir / f"cmd-{record['seq']:04d}-{ts}-{suffix}.md"
    payload = record.get("payload", {})
    lines = [
        "# Command Handoff",
        "",
        f"- Session key: `{record['session_key']}`",
        f"- Sequence: `{record['seq']:04d}`",
        f"- Timestamp (UTC): `{record['timestamp']}`",
        f"- Tool: `{record['tool_name']}`",
        "",
        "## Details",
        "",
        "```json",
        json.dumps(payload, indent=2),
        "```",
        "",
    ]
    cmd_file.write_text("\n".join(lines))
    return cmd_file


def main() -> None:
    try:
        data = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)

    tool_name = str(data.get("tool_name", ""))
    if tool_name not in _ALLOWED_TOOLS:
        sys.exit(0)

    tool_input = data.get("tool_input", {})
    if not isinstance(tool_input, dict):
        tool_input = {}

    session_key = _session_key(data)
    session_dir = _BASE_DIR / f"session-{session_key}"
    session_dir.mkdir(parents=True, exist_ok=True)

    meta = _load_session_meta(session_dir)
    seq = int(meta.get("seq", 0)) + 1
    timestamp = _now_iso()
    payload = _command_payload(tool_name, tool_input)
    preview = payload.get("preview", "")
    if not preview and "file_path" in payload:
        preview = str(payload.get("file_path", ""))

    record = {
        "seq": seq,
        "timestamp": timestamp,
        "session_key": session_key,
        "tool_name": tool_name,
        "preview": _preview(str(preview), 200),
        "payload": payload,
    }

    log_path = session_dir / "commands.jsonl"
    with log_path.open("a") as f:
        f.write(json.dumps(record) + "\n")

    cmd_file = _write_command_markdown(session_dir, record)
    _append_session_rollup(session_dir, record, cmd_file)
    _append_index(session_key, session_dir)

    meta["seq"] = seq
    meta["updated_at"] = timestamp
    _save_session_meta(session_dir, meta)

    sys.exit(0)


if __name__ == "__main__":
    main()
