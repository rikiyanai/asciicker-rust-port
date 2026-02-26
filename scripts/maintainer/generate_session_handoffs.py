#!/usr/bin/env python3
"""Generate session/command handoff artifacts from Claude session JSONL files.

Default behavior: process latest root session JSONL for this repo.
Optional: process all root session JSONLs with --all.
"""
from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_PROJECTS_DIR = Path.home() / ".claude" / "projects"
REPO_KEY = str(PROJECT_ROOT).replace("/", "-")
SESSION_ROOT = DEFAULT_PROJECTS_DIR / REPO_KEY
OUTPUT_ROOT = PROJECT_ROOT / "artifacts" / "maintainer" / "handoffs"


@dataclass
class CommandRecord:
    seq: int
    timestamp: str
    session_id: str
    tool_name: str
    preview: str
    payload: dict[str, Any]


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _preview(text: str, limit: int = 240) -> str:
    compact = " ".join((text or "").split())
    if len(compact) <= limit:
        return compact
    return compact[: limit - 3] + "..."


def _slug(text: str, limit: int = 48) -> str:
    cleaned = re.sub(r"[^a-zA-Z0-9._-]+", "-", text.strip())
    cleaned = cleaned.strip("-._")
    if not cleaned:
        cleaned = "item"
    return cleaned[:limit]


def _extract_content_blocks(msg: Any) -> list[dict[str, Any]]:
    if not isinstance(msg, dict):
        return []
    content = msg.get("content", [])
    if not isinstance(content, list):
        return []
    out: list[dict[str, Any]] = []
    for block in content:
        if isinstance(block, dict):
            out.append(block)
    return out


def _tool_payload(tool_name: str, tool_input: Any) -> tuple[str, dict[str, Any]]:
    payload = tool_input if isinstance(tool_input, dict) else {}
    if tool_name == "Bash":
        cmd = str(payload.get("command", ""))
        return _preview(cmd, 320), {"command": cmd}
    if tool_name in ("Write", "Edit", "MultiEdit"):
        fp = str(payload.get("file_path", ""))
        return _preview(fp, 200), payload
    return _preview(json.dumps(payload, sort_keys=True), 220), payload


def parse_session_jsonl(path: Path) -> list[CommandRecord]:
    records: list[CommandRecord] = []
    seq = 0
    with path.open("r", encoding="utf-8", errors="replace") as f:
        for line in f:
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(obj, dict):
                continue
            if obj.get("type") != "assistant":
                continue
            message = obj.get("message")
            blocks = _extract_content_blocks(message)
            if not blocks:
                continue
            ts = str(obj.get("timestamp") or _now_iso())
            session_id = str(obj.get("sessionId") or path.stem)
            for block in blocks:
                if block.get("type") != "tool_use":
                    continue
                tool_name = str(block.get("name", ""))
                tool_input = block.get("input", {})
                preview, payload = _tool_payload(tool_name, tool_input)
                seq += 1
                records.append(
                    CommandRecord(
                        seq=seq,
                        timestamp=ts,
                        session_id=session_id,
                        tool_name=tool_name,
                        preview=preview,
                        payload=payload,
                    )
                )
    return records


def write_handoff(session_file: Path, records: list[CommandRecord]) -> Path:
    session_id = session_file.stem
    out_dir = OUTPUT_ROOT / f"session-{session_id}"
    out_dir.mkdir(parents=True, exist_ok=True)

    # Clean previous per-command files for deterministic regeneration.
    for old in out_dir.glob("cmd-*.md"):
        old.unlink(missing_ok=True)

    commands_jsonl = out_dir / "commands.jsonl"
    with commands_jsonl.open("w") as f:
        for rec in records:
            f.write(
                json.dumps(
                    {
                        "seq": rec.seq,
                        "timestamp": rec.timestamp,
                        "session_id": rec.session_id,
                        "tool_name": rec.tool_name,
                        "preview": rec.preview,
                        "payload": rec.payload,
                    }
                )
                + "\n"
            )

    lines = [
        "# Session Handoff",
        "",
        f"- Session file: `{session_file}`",
        f"- Session id: `{session_id}`",
        f"- Commands captured: `{len(records)}`",
        "",
        "## Commands",
        "",
    ]

    for rec in records:
        cmd_name = f"cmd-{rec.seq:04d}-{_slug(rec.tool_name.lower(), 16)}.md"
        cmd_path = out_dir / cmd_name
        cmd_path.write_text(
            "\n".join(
                [
                    "# Command Handoff",
                    "",
                    f"- Session id: `{rec.session_id}`",
                    f"- Sequence: `{rec.seq:04d}`",
                    f"- Timestamp (UTC): `{rec.timestamp}`",
                    f"- Tool: `{rec.tool_name}`",
                    "",
                    "## Preview",
                    "",
                    f"`{rec.preview}`",
                    "",
                    "## Payload",
                    "",
                    "```json",
                    json.dumps(rec.payload, indent=2),
                    "```",
                    "",
                ]
            )
        )
        lines.append(
            f"- `{rec.seq:04d}` `{rec.timestamp}` `{rec.tool_name}` `{rec.preview}` -> `{cmd_name}`"
        )

    session_md = out_dir / "SESSION_HANDOFF.md"
    session_md.write_text("\n".join(lines) + "\n")
    return out_dir


def update_index(session_dirs: list[Path]) -> None:
    OUTPUT_ROOT.mkdir(parents=True, exist_ok=True)
    index = OUTPUT_ROOT / "INDEX.md"
    lines = ["# Session Handoffs", ""]
    for d in sorted(session_dirs):
        rel = d.relative_to(PROJECT_ROOT)
        lines.append(f"- `{d.name}` -> `{rel}`")
    index.write_text("\n".join(lines) + "\n")


def list_sessions(all_sessions: bool) -> list[Path]:
    if not SESSION_ROOT.exists():
        return []
    files = sorted(SESSION_ROOT.glob("*.jsonl"), key=lambda p: p.stat().st_mtime)
    if not files:
        return []
    if all_sessions:
        return files
    return [files[-1]]


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate command handoffs from session JSONL.")
    parser.add_argument("--all", action="store_true", help="Process all root session JSONLs.")
    args = parser.parse_args()

    sessions = list_sessions(all_sessions=args.all)
    if not sessions:
        print("No session JSONL files found.")
        return 1

    generated: list[Path] = []
    for session_file in sessions:
        records = parse_session_jsonl(session_file)
        out_dir = write_handoff(session_file, records)
        generated.append(out_dir)
        print(f"generated {out_dir} ({len(records)} commands)")

    update_index(generated)
    print(f"index {OUTPUT_ROOT / 'INDEX.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
