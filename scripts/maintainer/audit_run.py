#!/usr/bin/env python3
"""Audit — cross-session health scoring.

Computes a health score (0-100) based on:
- Artifact presence (report files, failure log)
- Failure log hygiene (no stale opens, proper status vocab)
- Unsupported claim patterns across recent sessions
- Evidence density (claims backed by refs)

Usage:
    python3 scripts/maintainer/audit_run.py --sessions 1 --dry-run
    python3 scripts/maintainer/audit_run.py --sessions 3
"""
from __future__ import annotations

import argparse
import sys
from datetime import datetime, timezone
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.report_schema import (
    MaintainerReport, Finding, Action, EvidenceRef,
    now_iso, report_to_markdown,
)
from scripts.maintainer.lib.jsonl_parser import (
    parse_session_jsonl, get_assistant_messages,
    find_latest_session_jsonl, find_recent_session_jsonls,
)
from scripts.maintainer.lib.failure_log import (
    read_failure_log, find_open_entries, find_stale_open_entries,
    find_long_open_entries, CANONICAL_FAILURE_LOG,
)

ARTIFACTS_DIR = _PROJECT_ROOT / "artifacts" / "maintainer"

# Health score weights (total must sum to 100)
WEIGHTS = {
    "failure_log_exists": 15,
    "failure_log_hygiene": 20,
    "artifact_presence": 15,
    "claim_quality": 25,
    "stale_open_penalty": 25,
}


def _check_failure_log_exists() -> tuple[int, list[Finding]]:
    """Check that the canonical failure log exists."""
    path = _PROJECT_ROOT / CANONICAL_FAILURE_LOG
    if path.exists():
        return WEIGHTS["failure_log_exists"], []
    return 0, [Finding(
        id="AUD-001",
        severity="high",
        category="missing_artifact",
        summary="Canonical failure log missing",
        details=f"Expected at {CANONICAL_FAILURE_LOG}",
    )]


def _check_failure_log_hygiene() -> tuple[int, list[Finding]]:
    """Check failure log entries for proper status vocabulary and structure."""
    path = _PROJECT_ROOT / CANONICAL_FAILURE_LOG
    if not path.exists():
        return 0, []

    try:
        entries = read_failure_log(path)
    except Exception:
        return 0, [Finding(
            id="AUD-002",
            severity="high",
            category="parse_error",
            summary="Failure log could not be parsed",
        )]

    if not entries:
        # Empty but exists — partial credit
        return WEIGHTS["failure_log_hygiene"] // 2, [Finding(
            id="AUD-003",
            severity="info",
            category="empty_log",
            summary="Failure log exists but has no entries",
        )]

    findings: list[Finding] = []
    # Check that all entries have required fields.
    # date_opened is optional for table-format entries (no date column).
    malformed = 0
    for entry in entries:
        if not entry.description:
            malformed += 1

    if malformed > 0:
        findings.append(Finding(
            id="AUD-004",
            severity="medium",
            category="log_hygiene",
            summary=f"{malformed} failure log entries missing description",
        ))
        return WEIGHTS["failure_log_hygiene"] // 2, findings

    return WEIGHTS["failure_log_hygiene"], findings


def _check_artifact_presence() -> tuple[int, list[Finding]]:
    """Check that maintainer artifact directory exists and has recent reports."""
    if not ARTIFACTS_DIR.exists():
        return 0, [Finding(
            id="AUD-005",
            severity="medium",
            category="missing_artifact",
            summary="No maintainer artifacts directory",
        )]

    reports = list(ARTIFACTS_DIR.glob("*.md"))
    if not reports:
        return WEIGHTS["artifact_presence"] // 2, [Finding(
            id="AUD-006",
            severity="info",
            category="no_reports",
            summary="Artifacts directory exists but no reports found",
        )]

    return WEIGHTS["artifact_presence"], []


def _check_claim_quality(
    session_jsonls: list[Path],
) -> tuple[int, list[Finding]]:
    """Check assistant messages for unsupported completion claims across sessions."""
    if not session_jsonls:
        return WEIGHTS["claim_quality"] // 2, [Finding(
            id="AUD-007",
            severity="info",
            category="no_session",
            summary="No session JSONL for claim quality check",
        )]

    # Import claim_guard's check logic
    from scripts.maintainer.claim_guard import check_message

    unsupported_count = 0
    sessions_checked = 0
    for jsonl_path in session_jsonls:
        if not jsonl_path.exists():
            continue
        session = parse_session_jsonl(jsonl_path)
        assistant_msgs = get_assistant_messages(session)
        sessions_checked += 1
        for msg in assistant_msgs:
            report = check_message(msg.text, mode="warn")
            unsupported = [f for f in report.findings
                           if f.category == "unsupported_claim"]
            unsupported_count += len(unsupported)

    if sessions_checked == 0:
        return WEIGHTS["claim_quality"] // 2, [Finding(
            id="AUD-007",
            severity="info",
            category="no_session",
            summary="No valid session JSONLs found",
        )]

    findings: list[Finding] = []
    if unsupported_count > 0:
        findings.append(Finding(
            id="AUD-008",
            severity="high" if unsupported_count >= 3 else "medium",
            category="claim_quality",
            summary=f"{unsupported_count} unsupported claims across {sessions_checked} session(s)",
        ))
        penalty = min(unsupported_count * 5, WEIGHTS["claim_quality"])
        return WEIGHTS["claim_quality"] - penalty, findings

    return WEIGHTS["claim_quality"], findings


def _check_stale_opens() -> tuple[int, list[Finding]]:
    """Penalize failure log entries with no activity for >7 days.

    Uses hybrid model: stale = no activity in 7 days (based on
    max(date_opened, last_update_date)). Also flags long_open entries
    (opened >30 days ago, even if recently updated) as info-level.
    """
    path = _PROJECT_ROOT / CANONICAL_FAILURE_LOG
    if not path.exists():
        return WEIGHTS["stale_open_penalty"], []

    try:
        entries = read_failure_log(path)
    except Exception:
        return WEIGHTS["stale_open_penalty"] // 2, []

    stale_entries = find_stale_open_entries(entries, stale_days=7)
    long_open = find_long_open_entries(entries, long_open_days=30)

    findings: list[Finding] = []

    if stale_entries:
        penalty_per_entry = 5
        total_penalty = min(
            len(stale_entries) * penalty_per_entry,
            WEIGHTS["stale_open_penalty"],
        )

        findings.append(Finding(
            id="AUD-009",
            severity="medium" if len(stale_entries) <= 2 else "high",
            category="stale_open",
            summary=f"{len(stale_entries)} failure log entries with no activity >7 days",
            details=", ".join(e.failure_id for e in stale_entries),
        ))
    else:
        total_penalty = 0

    # Secondary flag: entries open >30 days even if recently updated
    # Info-only, no score penalty — surfaces ancient unresolved problems
    if long_open:
        # Exclude entries already in stale list to avoid double-reporting
        long_only = [e for e in long_open if e not in stale_entries]
        if long_only:
            findings.append(Finding(
                id="AUD-010",
                severity="info",
                category="long_open",
                summary=f"{len(long_only)} failure log entries open >30 days (recently active)",
                details=", ".join(e.failure_id for e in long_only),
            ))

    if not stale_entries:
        return WEIGHTS["stale_open_penalty"], findings

    return WEIGHTS["stale_open_penalty"] - total_penalty, findings


def run_audit(
    sessions: int = 1,
    dry_run: bool = False,
) -> MaintainerReport:
    """Run audit and compute health score."""
    all_findings: list[Finding] = []
    all_actions: list[Action] = []
    total_score = 0

    # Find session JSONLs (supports multi-session via --sessions N)
    session_jsonls = find_recent_session_jsonls(n=sessions)

    # Run all checks
    checks = [
        _check_failure_log_exists(),
        _check_failure_log_hygiene(),
        _check_artifact_presence(),
        _check_claim_quality(session_jsonls),
        _check_stale_opens(),
    ]

    for score, findings in checks:
        total_score += score
        all_findings.extend(findings)

    # Clamp score
    total_score = max(0, min(100, total_score))

    # Generate actions for low scores
    if total_score < 50:
        all_actions.append(Action(
            id="AUD-A01",
            priority="high",
            summary="Health score below 50 — review findings and address gaps",
        ))
    elif total_score < 75:
        all_actions.append(Action(
            id="AUD-A02",
            priority="medium",
            summary="Health score moderate — address open failure entries",
        ))

    evidence: list[EvidenceRef] = []
    for jp in session_jsonls:
        evidence.append(EvidenceRef(
            kind="file", value=str(jp),
            description=f"Session analyzed ({jp.name})",
        ))

    report = MaintainerReport(
        tool_name="audit",
        timestamp=now_iso(),
        dry_run=dry_run,
        findings=tuple(all_findings),
        actions=tuple(all_actions),
        evidence=tuple(evidence),
        health_score=total_score,
        summary=f"Audit health score: {total_score}/100 ({len(all_findings)} findings)",
    )

    if not dry_run:
        ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)
        ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
        out_path = ARTIFACTS_DIR / f"audit_report_{ts}.md"
        out_path.write_text(report_to_markdown(report), encoding="utf-8")
        print(f"Report written to {out_path}")
    else:
        print(report_to_markdown(report))

    return report


def main():
    parser = argparse.ArgumentParser(description="Audit — health scoring")
    parser.add_argument("--sessions", type=int, default=1,
                        help="Number of recent sessions to analyze")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    report = run_audit(sessions=args.sessions, dry_run=args.dry_run)
    # Warn mode is always exit 0 (non-blocking). Block mode reserved for Phase 2.
    sys.exit(0)


if __name__ == "__main__":
    main()
