#!/usr/bin/env python3
"""Goal sanity checker — detect drift and churn in session goals.

Analyzes a session JSONL to measure:
- Goal drift (topic changes without completion)
- Plan churn (repeated re-planning without execution)
- Contradiction count (conflicting statements)
- Frustration recurrence

Recommendation: continue | pause | re-baseline

Usage:
    python3 scripts/maintainer/goal_sanity.py --dry-run
    python3 scripts/maintainer/goal_sanity.py --session-jsonl path/to/session.jsonl
"""
from __future__ import annotations

import argparse
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.report_schema import (
    MaintainerReport, Finding, Action, FrustrationSignal, EvidenceRef,
    now_iso, report_to_markdown,
)
from scripts.maintainer.lib.jsonl_parser import (
    parse_session_jsonl, get_assistant_messages, get_user_messages,
    find_latest_session_jsonl,
)

ARTIFACTS_DIR = _PROJECT_ROOT / "artifacts" / "maintainer"

# Signals of plan churn
PLAN_PATTERNS = [
    re.compile(r"\blet\s+me\s+try\s+a\s+different\s+approach\b", re.IGNORECASE),
    re.compile(r"\bactually,?\s+(?:let'?s?|I'?ll)\b", re.IGNORECASE),
    re.compile(r"\bnew\s+(?:plan|approach|strategy)\b", re.IGNORECASE),
    re.compile(r"\bscrap\s+(?:that|this|the\s+previous)\b", re.IGNORECASE),
]

# Contradiction signals
CONTRADICTION_PATTERNS = [
    re.compile(r"\bprevious\s+(?:analysis|diagnosis)\s+was\s+wrong\b", re.IGNORECASE),
    re.compile(r"\bcorrection:\b", re.IGNORECASE),
    re.compile(r"\bcontrary\s+to\s+(?:what\s+I|my\s+earlier)\b", re.IGNORECASE),
    re.compile(r"\bI\s+was\s+wrong\s+about\b", re.IGNORECASE),
]

# Topic shift markers
TOPIC_PATTERNS = [
    re.compile(r"\bswitching\s+to\b", re.IGNORECASE),
    re.compile(r"\bmoving\s+on\s+to\b", re.IGNORECASE),
    re.compile(r"\blet'?s?\s+focus\s+on\b", re.IGNORECASE),
    re.compile(r"\binstead,?\s+(?:let'?s?|I'?ll)\b", re.IGNORECASE),
]


def _count_pattern_matches(
    messages: list, patterns: list[re.Pattern],
) -> tuple[int, list[str]]:
    """Count messages matching any pattern, return count and examples."""
    count = 0
    examples: list[str] = []
    for msg in messages:
        for pat in patterns:
            if pat.search(msg.text):
                count += 1
                snippet = msg.text[:120].replace("\n", " ")
                examples.append(f"Line {msg.line_number}: {snippet}")
                break
    return count, examples


def _compute_recommendation(
    drift_count: int,
    churn_count: int,
    contradiction_count: int,
    frustration_count: int,
) -> str:
    """Compute a recommendation based on signal counts."""
    # Weighted score: higher = more troubled
    score = (
        drift_count * 1
        + churn_count * 2
        + contradiction_count * 3
        + frustration_count * 2
    )
    if score >= 10:
        return "re-baseline"
    elif score >= 5:
        return "pause"
    else:
        return "continue"


def run_goal_sanity(
    session_jsonl: str | Path | None = None,
    dry_run: bool = False,
) -> MaintainerReport:
    """Run goal sanity analysis."""
    findings: list[Finding] = []
    frustrations: list[FrustrationSignal] = []
    evidence: list[EvidenceRef] = []

    if session_jsonl:
        jsonl_path = Path(session_jsonl)
    else:
        jsonl_path = find_latest_session_jsonl()

    if not jsonl_path or not jsonl_path.exists():
        return MaintainerReport(
            tool_name="goal_sanity",
            timestamp=now_iso(),
            dry_run=dry_run,
            findings=(Finding(
                id="GS-010",
                severity="info",
                category="no_session",
                summary="No session JSONL found",
            ),),
            recommendation="continue",
            summary="No session data available for analysis",
        )

    session = parse_session_jsonl(jsonl_path)
    assistant_msgs = get_assistant_messages(session)
    user_msgs = get_user_messages(session)

    evidence.append(EvidenceRef(
        kind="file",
        value=str(jsonl_path),
        description=f"Session ({session.total_lines} lines, "
                    f"{len(assistant_msgs)} assistant msgs)",
    ))

    # Goal drift
    drift_count, drift_examples = _count_pattern_matches(
        assistant_msgs, TOPIC_PATTERNS
    )
    if drift_count > 0:
        findings.append(Finding(
            id="GS-001",
            severity="medium" if drift_count < 3 else "high",
            category="goal_drift",
            summary=f"Detected {drift_count} topic shifts",
            evidence=tuple(
                EvidenceRef(kind="log_entry", value=ex)
                for ex in drift_examples[:3]
            ),
        ))
        frustrations.append(FrustrationSignal(
            signal_type="goal_drift",
            count=drift_count,
            examples=tuple(drift_examples[:3]),
        ))

    # Plan churn
    churn_count, churn_examples = _count_pattern_matches(
        assistant_msgs, PLAN_PATTERNS
    )
    if churn_count > 0:
        findings.append(Finding(
            id="GS-002",
            severity="medium" if churn_count < 3 else "high",
            category="plan_churn",
            summary=f"Detected {churn_count} plan changes",
            details="Repeated re-planning without executing to completion.",
            evidence=tuple(
                EvidenceRef(kind="log_entry", value=ex)
                for ex in churn_examples[:3]
            ),
        ))
        frustrations.append(FrustrationSignal(
            signal_type="plan_churn",
            count=churn_count,
            examples=tuple(churn_examples[:3]),
        ))

    # Contradictions
    contradiction_count, contra_examples = _count_pattern_matches(
        assistant_msgs, CONTRADICTION_PATTERNS
    )
    if contradiction_count > 0:
        findings.append(Finding(
            id="GS-003",
            severity="high" if contradiction_count >= 2 else "medium",
            category="contradiction",
            summary=f"Detected {contradiction_count} self-contradictions",
            evidence=tuple(
                EvidenceRef(kind="log_entry", value=ex)
                for ex in contra_examples[:3]
            ),
        ))

    # User frustration
    frustration_patterns = [
        re.compile(r"\bstill\s+(?:broken|wrong|not\s+working)\b", re.IGNORECASE),
        re.compile(r"\bdidn'?t\s+work\b", re.IGNORECASE),
        re.compile(r"\bNO[,.]", re.IGNORECASE),
    ]
    frust_count, frust_examples = _count_pattern_matches(
        user_msgs, frustration_patterns
    )
    if frust_count > 0:
        frustrations.append(FrustrationSignal(
            signal_type="frustration_recurrence",
            count=frust_count,
            examples=tuple(frust_examples[:3]),
        ))

    recommendation = _compute_recommendation(
        drift_count, churn_count, contradiction_count, frust_count
    )

    report = MaintainerReport(
        tool_name="goal_sanity",
        timestamp=now_iso(),
        dry_run=dry_run,
        findings=tuple(findings),
        frustrations=tuple(frustrations),
        evidence=tuple(evidence),
        recommendation=recommendation,
        summary=(
            f"Goal sanity: drift={drift_count}, churn={churn_count}, "
            f"contradictions={contradiction_count}, frustration={frust_count} "
            f"=> {recommendation}"
        ),
    )

    if not dry_run:
        ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)
        ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
        out_path = ARTIFACTS_DIR / f"goal_sanity_{ts}.md"
        out_path.write_text(report_to_markdown(report), encoding="utf-8")
        print(f"Report written to {out_path}")
    else:
        print(report_to_markdown(report))

    return report


def main():
    parser = argparse.ArgumentParser(description="Goal sanity checker")
    parser.add_argument("--session-jsonl", help="Path to session JSONL")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    report = run_goal_sanity(
        session_jsonl=args.session_jsonl,
        dry_run=args.dry_run,
    )

    # Warn mode is always exit 0 (non-blocking). Block mode reserved for Phase 2.
    sys.exit(0)


if __name__ == "__main__":
    main()
