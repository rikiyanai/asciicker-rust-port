"""Failure log management — append-only structured failure tracking.

The canonical failure log path is configurable per project.
Each entry has a unique ID (FL-NNN or FNNN), status, and structured fields.
Updates enforce append-only semantics and status vocabulary.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from .report_schema import FORBIDDEN_STATUS_WORDS

# Project-configurable canonical path.
# Override via MAINTAINER_FAILURE_LOG env var or by editing this constant.
import os as _os
CANONICAL_FAILURE_LOG = Path(
    _os.environ.get(
        "MAINTAINER_FAILURE_LOG",
        "docs/FAILURE_LOG.md",
    )
)

VALID_STATUS = frozenset({"OPEN", "PARTIAL", "MONITORING", "RESOLVED"})

# Regex to parse FL entries from markdown (### header format)
_FL_HEADER_RE = re.compile(
    r"^###\s+(FL-\d{3,4})\s*[:\-—]\s*(.+)$", re.MULTILINE
)

# Regex to parse table-format entries: | F001 | description | severity | status | resolution |
_TABLE_ROW_RE = re.compile(
    r"^\|\s*(F\d{3,4})\s*\|\s*(.+?)\s*\|\s*(\w+)\s*\|\s*(\w+)\s*\|\s*(.*?)\s*\|$",
    re.MULTILINE,
)
_FL_STATUS_RE = re.compile(
    r"^\*{0,2}Status:\*{0,2}\s*(\w+)", re.MULTILINE
)


@dataclass
class FailureEntry:
    """A single failure log entry.

    ``status`` is the original status from the **Status:** line.
    ``effective_status`` reflects the latest append-only status update
    (if any), falling back to ``status`` when no updates exist.
    Consumers should use ``effective_status`` for filtering.
    """
    failure_id: str         # "FL-001"
    title: str
    status: str             # OPEN | PARTIAL | MONITORING | RESOLVED (original)
    date_opened: str        # ISO date
    category: str           # e.g. "pipeline", "quality_gate", "doc_drift"
    description: str
    root_cause: str = ""
    evidence: list[str] = field(default_factory=list)    # commit hashes, file paths
    resolution: str = ""
    date_resolved: str = ""
    related_ids: list[str] = field(default_factory=list)  # e.g. ["FL-002"]
    effective_status: str = ""   # latest status after updates (empty = same as status)
    last_update_date: str = ""   # date of most recent status update

    def __post_init__(self):
        # Validate original status
        if self.status not in VALID_STATUS:
            raise ValueError(
                f"Invalid status {self.status!r}, must be one of {VALID_STATUS}"
            )
        if not self.failure_id.startswith("FL-"):
            raise ValueError(
                f"failure_id must start with 'FL-', got {self.failure_id!r}"
            )
        # Default effective_status to original status
        if not self.effective_status:
            object.__setattr__(self, "effective_status", self.status)
        # Enforce RESOLVED invariant at creation time
        if self.effective_status == "RESOLVED":
            if not self.resolution and not self.evidence:
                raise ValueError(
                    "RESOLVED status requires resolution text or evidence"
                )


def _parse_entry_block(block: str, fid: str, title: str) -> FailureEntry:
    """Parse a single entry block into a FailureEntry."""
    status_match = _FL_STATUS_RE.search(block)
    status = status_match.group(1) if status_match else "OPEN"

    def _extract_field(name: str) -> str:
        # Markdown bold format: **Field:** value (colon inside bold)
        pattern = re.compile(
            rf"^\*{{0,2}}{name}:\*{{0,2}}\s*(.+)$",
            re.MULTILINE | re.IGNORECASE,
        )
        m = pattern.search(block)
        if not m:
            return ""
        return m.group(1).strip()

    date_opened = _extract_field("Date Opened") or _extract_field("Opened")
    category = _extract_field("Category")
    description = _extract_field("Description")
    root_cause = _extract_field("Root Cause")
    resolution = _extract_field("Resolution")
    date_resolved = _extract_field("Date Resolved") or _extract_field("Resolved")

    # Extract evidence lines (bulleted list after "Evidence:")
    evidence: list[str] = []
    ev_match = re.search(
        r"^\*{0,2}Evidence:\*{0,2}\s*\n((?:\s*[-*]\s+.+\n?)+)",
        block, re.MULTILINE
    )
    if ev_match:
        for line in ev_match.group(1).strip().split("\n"):
            line = re.sub(r"^\s*[-*]\s+", "", line).strip()
            if line:
                evidence.append(line)

    # Extract related IDs
    related: list[str] = []
    rel_match = re.search(
        r"^\*{0,2}Related:\*{0,2}\s*(.+)$", block, re.MULTILINE
    )
    if rel_match:
        related = [r.strip() for r in rel_match.group(1).split(",") if r.strip()]

    # Parse append-only status update subsections.
    # Format: > **[2026-02-19] Status update: OPEN -> PARTIAL**
    effective_status = status if status in VALID_STATUS else "OPEN"
    last_update_date = ""
    _STATUS_UPDATE_RE = re.compile(
        r">\s*\*{0,2}\[(\d{4}-\d{2}-\d{2})\]\s*Status update:\s*\w+\s*->\s*(\w+)\*{0,2}",
    )
    for m in _STATUS_UPDATE_RE.finditer(block):
        update_date = m.group(1)
        new_status = m.group(2)
        if new_status in VALID_STATUS:
            effective_status = new_status
            last_update_date = update_date
    # If resolved via update, pull resolution from the update subsection
    if effective_status == "RESOLVED" and not resolution:
        # Look for resolution line in blockquote after the last status update
        res_in_update = re.findall(
            r">\s*(?:Resolution:\s*)?(.+?)(?:\n|$)", block
        )
        if res_in_update:
            # Use the line right after the status update header
            for line in res_in_update:
                line = line.strip()
                if line and "Status update:" not in line and "Evidence:" not in line:
                    resolution = line
                    break

    return FailureEntry(
        failure_id=fid,
        title=title,
        status=status if status in VALID_STATUS else "OPEN",
        date_opened=date_opened,
        category=category,
        description=description,
        root_cause=root_cause,
        evidence=evidence,
        resolution=resolution,
        date_resolved=date_resolved,
        related_ids=related,
        effective_status=effective_status,
        last_update_date=last_update_date,
    )


def read_failure_log(path: Optional[Path] = None) -> list[FailureEntry]:
    """Read and parse the failure log markdown into entries.

    Supports two formats:
    1. Header format: ### FL-001: Title  (followed by **Status:** etc.)
    2. Table format:  | F001 | description | severity | status | resolution |
    """
    path = path or CANONICAL_FAILURE_LOG
    if not path.exists():
        return []

    content = path.read_text(encoding="utf-8")

    # Try header format first (### FL-NNN: Title)
    headers = list(_FL_HEADER_RE.finditer(content))
    if headers:
        entries: list[FailureEntry] = []
        for i, match in enumerate(headers):
            fid = match.group(1)
            title = match.group(2).strip()
            start = match.end()
            end = headers[i + 1].start() if i + 1 < len(headers) else len(content)
            block = content[start:end]
            try:
                entries.append(_parse_entry_block(block, fid, title))
            except ValueError:
                continue
        return entries

    # Fallback: table format (| FNNN | desc | severity | status | resolution |)
    table_rows = list(_TABLE_ROW_RE.finditer(content))
    if table_rows:
        return _parse_table_entries(table_rows)

    return []


def _parse_table_entries(
    rows: list[re.Match],
) -> list[FailureEntry]:
    """Parse table-format failure log rows into FailureEntry objects."""
    entries: list[FailureEntry] = []
    for match in rows:
        fid_raw = match.group(1).strip()       # "F001"
        description = match.group(2).strip()
        severity = match.group(3).strip().lower()
        status = match.group(4).strip().upper()
        resolution = match.group(5).strip()

        # Normalize ID: F001 -> FL-001 for internal consistency
        fid = f"FL-{fid_raw[1:]}"

        if status not in VALID_STATUS:
            continue

        try:
            entries.append(FailureEntry(
                failure_id=fid,
                title=description[:80],
                status=status,
                date_opened="",
                category=severity,
                description=description,
                resolution=resolution,
            ))
        except ValueError:
            continue

    return entries


def next_failure_id(entries: list[FailureEntry]) -> str:
    """Generate the next FL-NNN id."""
    if not entries:
        return "FL-001"
    max_num = 0
    for e in entries:
        try:
            num = int(e.failure_id.split("-")[1])
            max_num = max(max_num, num)
        except (IndexError, ValueError):
            continue
    return f"FL-{max_num + 1:03d}"


def find_open_entries(entries: list[FailureEntry]) -> list[FailureEntry]:
    """Return entries whose effective status is OPEN or PARTIAL."""
    return [e for e in entries if e.effective_status in ("OPEN", "PARTIAL")]


def _parse_iso_date(date_str: str) -> Optional[datetime]:
    """Parse a YYYY-MM-DD string into a timezone-aware datetime, or None."""
    if not date_str or not date_str.strip():
        return None
    try:
        return datetime.strptime(
            date_str.strip(), "%Y-%m-%d"
        ).replace(tzinfo=timezone.utc)
    except ValueError:
        return None


def _last_activity_date(entry: FailureEntry) -> Optional[datetime]:
    """Return max(date_opened, last_update_date) — the most recent activity."""
    opened = _parse_iso_date(entry.date_opened)
    updated = _parse_iso_date(entry.last_update_date)
    if opened and updated:
        return max(opened, updated)
    return updated or opened  # whichever is non-None, or None


def find_stale_open_entries(
    entries: list[FailureEntry],
    stale_days: int = 7,
) -> list[FailureEntry]:
    """Return OPEN/PARTIAL entries with no activity for stale_days.

    Uses max(date_opened, last_update_date) as the activity date.
    A recent status update resets the staleness clock.
    """
    open_entries = find_open_entries(entries)
    if not open_entries:
        return []

    now = datetime.now(timezone.utc)
    stale: list[FailureEntry] = []
    for entry in open_entries:
        activity = _last_activity_date(entry)
        if activity is None:
            # No parseable date — skip rather than assume stale.
            # Table-format entries lack dates; penalizing them is a false positive.
            continue
        days_since_activity = (now - activity).days
        if days_since_activity >= stale_days:
            stale.append(entry)

    return stale


def find_long_open_entries(
    entries: list[FailureEntry],
    long_open_days: int = 30,
) -> list[FailureEntry]:
    """Return OPEN/PARTIAL entries opened more than long_open_days ago.

    Unlike find_stale_open_entries(), this ignores recent updates —
    it flags entries that have been unresolved for a long time even
    if they're being actively managed.
    """
    open_entries = find_open_entries(entries)
    if not open_entries:
        return []

    now = datetime.now(timezone.utc)
    long_open: list[FailureEntry] = []
    for entry in open_entries:
        opened = _parse_iso_date(entry.date_opened)
        if opened is None:
            # No parseable date — skip rather than assume long-open.
            continue
        if (now - opened).days >= long_open_days:
            long_open.append(entry)

    return long_open


def entry_to_markdown(entry: FailureEntry) -> str:
    """Render a single entry as markdown block."""
    lines = [
        f"### {entry.failure_id}: {entry.title}",
        f"",
        f"**Status:** {entry.status}",
        f"**Date Opened:** {entry.date_opened}",
        f"**Category:** {entry.category}",
        f"**Description:** {entry.description}",
    ]
    if entry.root_cause:
        lines.append(f"**Root Cause:** {entry.root_cause}")
    if entry.evidence:
        lines.append(f"**Evidence:**")
        for ev in entry.evidence:
            lines.append(f"- {ev}")
    if entry.resolution:
        lines.append(f"**Resolution:** {entry.resolution}")
    if entry.date_resolved:
        lines.append(f"**Date Resolved:** {entry.date_resolved}")
    if entry.related_ids:
        lines.append(f"**Related:** {', '.join(entry.related_ids)}")
    lines.append("")
    return "\n".join(lines)


def append_entry(
    entry: FailureEntry,
    path: Optional[Path] = None,
    dry_run: bool = False,
) -> str:
    """Append a new entry to the failure log. Returns the markdown text.

    In dry_run mode, returns the text without writing to disk.
    """
    path = path or CANONICAL_FAILURE_LOG
    md = entry_to_markdown(entry)

    if dry_run:
        return md

    # Ensure parent directory exists
    path.parent.mkdir(parents=True, exist_ok=True)

    if not path.exists():
        header = "# Failure Log\n\nCanonical append-only failure tracking.\n\n"
        path.write_text(header + md, encoding="utf-8")
    else:
        with open(path, "a", encoding="utf-8") as f:
            f.write("\n" + md)

    return md


def update_status(
    failure_id: str,
    new_status: str,
    resolution: str = "",
    evidence_refs: tuple[str, ...] = (),
    path: Optional[Path] = None,
    dry_run: bool = False,
) -> bool:
    """Record a status change by appending a subsection (never edits prior lines).

    Enforces:
    - new_status must be in VALID_STATUS
    - RESOLVED requires non-empty resolution AND at least one evidence ref
    - Append-only: original status line is preserved, update is appended
    """
    if new_status not in VALID_STATUS:
        raise ValueError(f"Invalid status {new_status!r}")

    if new_status == "RESOLVED" and not resolution:
        raise ValueError("RESOLVED status requires a resolution description")

    if new_status == "RESOLVED" and not evidence_refs:
        raise ValueError("RESOLVED status requires at least one evidence ref")

    path = path or CANONICAL_FAILURE_LOG
    if not path.exists():
        return False

    content = path.read_text(encoding="utf-8")

    # Find this entry's block to locate where to append the update
    entry_header = re.compile(
        rf"^###\s+{re.escape(failure_id)}\s*[:\-—]",
        re.MULTILINE,
    )
    match = entry_header.search(content)
    if not match:
        return False

    # Determine the current effective status for the from->to label.
    # Check for append-only status updates first (most recent wins),
    # then fall back to the original **Status:** line.
    block_start = match.start()
    next_hdr = re.search(r"^### ", content[match.end():], re.MULTILINE)
    block_end = match.end() + next_hdr.start() if next_hdr else len(content)
    block_text = content[block_start:block_end]

    _STATUS_UPDATE_RE_LOCAL = re.compile(
        r">\s*\*{0,2}\[\d{4}-\d{2}-\d{2}\]\s*Status update:\s*\w+\s*->\s*(\w+)\*{0,2}",
    )
    update_matches = list(_STATUS_UPDATE_RE_LOCAL.finditer(block_text))
    if update_matches:
        old_status = update_matches[-1].group(1)  # last update's target
    else:
        status_in_block = _FL_STATUS_RE.search(block_text)
        old_status = status_in_block.group(1) if status_in_block else "?"

    if dry_run:
        return True

    # Reuse block_end computed above for the insert point
    insert_point = block_end

    # Build append-only status update subsection
    date_str = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    update_lines = [
        f"\n> **[{date_str}] Status update: {old_status} -> {new_status}**",
    ]
    if resolution:
        update_lines.append(f"> {resolution}")
    for ref in evidence_refs:
        update_lines.append(f"> Evidence: {ref}")
    update_lines.append("")

    update_text = "\n".join(update_lines)

    new_content = (
        content[:insert_point].rstrip("\n")
        + "\n"
        + update_text
        + "\n"
        + content[insert_point:]
    )

    path.write_text(new_content, encoding="utf-8")
    return True
