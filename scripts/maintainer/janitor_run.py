#!/usr/bin/env python3
"""Janitor — session hygiene analysis.

Scans a Claude session JSONL for:
- Hypothesis churn (root cause changed N times without resolution)
- Stale opens (failure log entries open > 7 days)
- Unsupported confidence language ("resolved", "fixed" without evidence)

Usage:
    python3 scripts/maintainer/janitor_run.py --dry-run --mode light
    python3 scripts/maintainer/janitor_run.py --session-jsonl path/to/session.jsonl
"""
from __future__ import annotations

import argparse
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

# Resolve imports relative to project root
_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.report_schema import (
    MaintainerReport, Finding, Action, FrustrationSignal, EvidenceRef,
    FORBIDDEN_STATUS_WORDS, now_iso, report_to_markdown, validate_report,
)
from scripts.maintainer.lib.jsonl_parser import (
    parse_session_jsonl, get_assistant_messages, get_user_messages,
    find_latest_session_jsonl,
)
from scripts.maintainer.lib.failure_log import (
    read_failure_log, find_open_entries, find_stale_open_entries,
    CANONICAL_FAILURE_LOG,
)

ARTIFACTS_DIR = _PROJECT_ROOT / "artifacts" / "maintainer"

# Patterns that suggest hypothesis churn
HYPOTHESIS_PATTERNS = [
    re.compile(r"\b(?:actually|wait|correction|no,?\s+the\s+real)", re.IGNORECASE),
    re.compile(r"\broot\s+cause\s+(?:is|was|might\s+be)\b", re.IGNORECASE),
    re.compile(r"\bI\s+(?:think|believe)\s+the\s+(?:real|actual)\b", re.IGNORECASE),
    re.compile(r"\bprevious\s+(?:diagnosis|hypothesis)\s+was\s+wrong\b", re.IGNORECASE),
]

# Patterns indicating unsupported confidence claims
CONFIDENCE_PATTERNS = [
    re.compile(
        r"\b(?:issue\s+is\s+resolved|everything\s+(?:is\s+)?(?:fixed|working|green))\b",
        re.IGNORECASE,
    ),
    re.compile(r"\ball\s+tests\s+pass\b", re.IGNORECASE),
    re.compile(r"\bpipeline\s+(?:is\s+)?(?:complete|done|working)\b", re.IGNORECASE),
]


def _detect_hypothesis_churn(
    assistant_msgs: list,
) -> tuple[int, list[str]]:
    """Count hypothesis shifts and collect examples."""
    count = 0
    examples: list[str] = []
    for msg in assistant_msgs:
        for pat in HYPOTHESIS_PATTERNS:
            if pat.search(msg.text):
                count += 1
                # Truncate long examples
                snippet = msg.text[:120].replace("\n", " ")
                examples.append(f"Line {msg.line_number}: {snippet}")
                break  # one match per message
    return count, examples


def _detect_unsupported_claims(
    assistant_msgs: list,
) -> tuple[int, list[str]]:
    """Find confidence claims that lack evidence backing."""
    count = 0
    examples: list[str] = []
    for msg in assistant_msgs:
        text_lower = msg.text.lower()
        for pat in CONFIDENCE_PATTERNS:
            if pat.search(msg.text):
                count += 1
                snippet = msg.text[:120].replace("\n", " ")
                examples.append(f"Line {msg.line_number}: {snippet}")
                break
    return count, examples


def _detect_user_frustration(
    user_msgs: list,
) -> tuple[int, list[str]]:
    """Detect repeated user frustration/correction signals."""
    frustration_patterns = [
        re.compile(r"\bstill\s+(?:broken|wrong|not\s+working)\b", re.IGNORECASE),
        re.compile(r"\bNO[,.]?\s+it'?s?\s+NOT\b", re.IGNORECASE),
        re.compile(r"\bdidn'?t\s+work\b", re.IGNORECASE),
        re.compile(r"\blook(?:s)?\s+(?:like\s+)?garbage\b", re.IGNORECASE),
    ]
    count = 0
    examples: list[str] = []
    for msg in user_msgs:
        for pat in frustration_patterns:
            if pat.search(msg.text):
                count += 1
                snippet = msg.text[:120].replace("\n", " ")
                examples.append(f"Line {msg.line_number}: {snippet}")
                break
    return count, examples


def run_janitor(
    session_jsonl: str | Path | None = None,
    mode: str = "full",
    dry_run: bool = False,
) -> MaintainerReport:
    """Run janitor analysis and return a structured report."""
    findings: list[Finding] = []
    actions: list[Action] = []
    frustrations: list[FrustrationSignal] = []
    evidence: list[EvidenceRef] = []

    # --- Parse session JSONL ---
    if session_jsonl:
        jsonl_path = Path(session_jsonl)
    else:
        jsonl_path = find_latest_session_jsonl()

    if jsonl_path and jsonl_path.exists():
        session = parse_session_jsonl(jsonl_path)
        assistant_msgs = get_assistant_messages(session)
        user_msgs = get_user_messages(session)

        evidence.append(EvidenceRef(
            kind="file",
            value=str(jsonl_path),
            description=f"Session JSONL ({session.total_lines} lines)",
        ))

        # Hypothesis churn detection
        churn_count, churn_examples = _detect_hypothesis_churn(assistant_msgs)
        if churn_count > 0:
            findings.append(Finding(
                id="JAN-001",
                severity="high" if churn_count >= 3 else "medium",
                category="hypothesis_churn",
                summary=f"Hypothesis shifted {churn_count} times in session",
                details="Root cause diagnosis changed without prior resolution.",
                evidence=tuple(
                    EvidenceRef(kind="log_entry", value=ex)
                    for ex in churn_examples[:3]
                ),
            ))
            frustrations.append(FrustrationSignal(
                signal_type="hypothesis_churn",
                count=churn_count,
                examples=tuple(churn_examples[:3]),
            ))

        # Unsupported confidence claims
        claim_count, claim_examples = _detect_unsupported_claims(assistant_msgs)
        if claim_count > 0:
            findings.append(Finding(
                id="JAN-002",
                severity="high",
                category="unsupported_claim",
                summary=f"Found {claim_count} unsupported confidence claims",
                details="Claims of completion/resolution without evidence refs.",
                evidence=tuple(
                    EvidenceRef(kind="log_entry", value=ex)
                    for ex in claim_examples[:3]
                ),
            ))
            actions.append(Action(
                id="JAN-A01",
                priority="high",
                summary="Add evidence refs to completion claims",
                target="claim_guard.py",
            ))

        # User frustration
        frust_count, frust_examples = _detect_user_frustration(user_msgs)
        if frust_count > 0:
            frustrations.append(FrustrationSignal(
                signal_type="repeated_failure",
                count=frust_count,
                examples=tuple(frust_examples[:3]),
            ))
            if frust_count >= 2:
                findings.append(Finding(
                    id="JAN-003",
                    severity="high",
                    category="user_frustration",
                    summary=f"User expressed frustration {frust_count} times",
                    details="Repeated corrections suggest approach is not working.",
                ))
    else:
        findings.append(Finding(
            id="JAN-010",
            severity="info",
            category="no_session",
            summary="No session JSONL found for analysis",
        ))

    # --- Failure log stale opens (full mode only) ---
    if mode == "full":
        try:
            fl_path = _PROJECT_ROOT / CANONICAL_FAILURE_LOG
            fl_entries = read_failure_log(fl_path)
            stale_entries = find_stale_open_entries(fl_entries, stale_days=7)
            if stale_entries:
                findings.append(Finding(
                    id="JAN-004",
                    severity="medium" if len(stale_entries) <= 3 else "high",
                    category="stale_open",
                    summary=(
                        f"{len(stale_entries)} failure log entries open >7 days"
                    ),
                    details=", ".join(e.failure_id for e in stale_entries),
                ))
            # Also report total open count as info
            all_open = find_open_entries(fl_entries)
            non_stale = len(all_open) - len(stale_entries)
            if non_stale > 0:
                findings.append(Finding(
                    id="JAN-005",
                    severity="info",
                    category="open_recent",
                    summary=f"{non_stale} failure log entries open <7 days",
                    details=", ".join(
                        e.failure_id for e in all_open
                        if e not in stale_entries
                    ),
                ))
        except Exception:
            # Failure log may not exist yet
            pass

    report = MaintainerReport(
        tool_name="janitor",
        timestamp=now_iso(),
        mode="warn",
        dry_run=dry_run,
        findings=tuple(findings),
        actions=tuple(actions),
        frustrations=tuple(frustrations),
        evidence=tuple(evidence),
        summary=f"Janitor scan ({mode}): {len(findings)} findings",
    )

    # Write report artifact
    if not dry_run:
        ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)
        ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
        out_path = ARTIFACTS_DIR / f"janitor_report_{ts}.md"
        out_path.write_text(report_to_markdown(report), encoding="utf-8")
        print(f"Report written to {out_path}")
    else:
        print(report_to_markdown(report))

    return report


def main():
    parser = argparse.ArgumentParser(description="Janitor — session hygiene analysis")
    parser.add_argument("--session-jsonl", help="Path to session JSONL file")
    parser.add_argument("--dry-run", action="store_true", help="Print report without writing")
    parser.add_argument("--mode", choices=["full", "light"], default="full",
                        help="full=session+failure log, light=session only")
    args = parser.parse_args()

    report = run_janitor(
        session_jsonl=args.session_jsonl,
        mode=args.mode,
        dry_run=args.dry_run,
    )

    # Warn mode is always exit 0 (non-blocking). Block mode reserved for Phase 2.
    sys.exit(0)


if __name__ == "__main__":
    main()
