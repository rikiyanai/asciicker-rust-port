#!/usr/bin/env python3
"""Claim guard — flag unsupported status claims.

Checks a message (or commit message) for forbidden completion language
without evidence references. MVP runs in warn mode only.

Usage:
    python3 scripts/maintainer/claim_guard.py --message "fix(pipeline): resolved everything"
    python3 scripts/maintainer/claim_guard.py --message "fix(pipeline): FL-001 partial fix" --mode warn
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.report_schema import (
    MaintainerReport, Finding, EvidenceRef, FORBIDDEN_STATUS_WORDS,
    VALID_FL_STATUS, now_iso, report_to_markdown,
)

# FL reference pattern: FL-001, FL-002, etc.
FL_REF_PATTERN = re.compile(r"\bFL-\d{3,4}\b")

# Short failure ID pattern: F001, F002, etc. (project-specific)
SHORT_FL_REF_PATTERN = re.compile(r"\bF\d{3,4}\b")

# Risk ID pattern: R01, R02, ... R62 (project-specific)
RISK_REF_PATTERN = re.compile(r"\bR\d{1,3}\b")

# Commit hash pattern: 7+ hex chars
COMMIT_REF_PATTERN = re.compile(r"\b[0-9a-f]{7,40}\b")


def check_message(
    message: str,
    mode: str = "warn",
) -> MaintainerReport:
    """Check a single message for unsupported claims.

    Returns a report with findings. In warn mode, findings are informational.
    In block mode (Phase 2), findings would prevent the action.
    """
    findings: list[Finding] = []
    msg_lower = message.lower()

    # Check for forbidden words (word-boundary match to avoid false positives
    # like "completeness" matching "complete").
    # Negative lookahead/lookbehind for hyphens excludes compound words
    # like "closed-form", "fail-safe", "working-directory".
    found_forbidden: list[str] = []
    for word in FORBIDDEN_STATUS_WORDS:
        if re.search(rf"(?<!-)\b{re.escape(word)}\b(?!-)", msg_lower):
            found_forbidden.append(word)

    if not found_forbidden:
        return MaintainerReport(
            tool_name="claim_guard",
            timestamp=now_iso(),
            mode=mode,
            summary="No unsupported claims detected",
        )

    # Check for evidence refs that would support the claims
    has_fl_ref = bool(FL_REF_PATTERN.search(message))
    has_short_fl_ref = bool(SHORT_FL_REF_PATTERN.search(message))
    has_risk_ref = bool(RISK_REF_PATTERN.search(message))
    has_commit_ref = bool(COMMIT_REF_PATTERN.search(message))
    has_evidence = has_fl_ref or has_short_fl_ref or has_risk_ref or has_commit_ref

    if has_evidence:
        # Claims backed by evidence — just note it
        findings.append(Finding(
            id="CLM-001",
            severity="info",
            category="claim_with_evidence",
            summary=(
                f"Status words [{', '.join(found_forbidden)}] "
                f"found with evidence refs"
            ),
            evidence=(
                EvidenceRef(
                    kind="log_entry",
                    value=message[:200],
                    description="Input message",
                ),
            ),
        ))
    else:
        # Claims WITHOUT evidence — this is the problem we're catching
        findings.append(Finding(
            id="CLM-002",
            severity="high",
            category="unsupported_claim",
            summary=(
                f"Forbidden status words [{', '.join(found_forbidden)}] "
                f"used without evidence refs (FL-NNN or commit hash)"
            ),
            details=(
                f"Message contains completion language but no FL- reference "
                f"or commit hash to support the claim. "
                f"Add a failure log ref (FL-NNN) or commit hash."
            ),
            evidence=(
                EvidenceRef(
                    kind="log_entry",
                    value=message[:200],
                    description="Input message",
                ),
            ),
        ))

    report = MaintainerReport(
        tool_name="claim_guard",
        timestamp=now_iso(),
        mode=mode,
        findings=tuple(findings),
        summary=(
            f"Claim guard: {len(found_forbidden)} status words, "
            f"evidence={'yes' if has_evidence else 'NO'}"
        ),
    )

    return report


def main():
    parser = argparse.ArgumentParser(description="Claim guard — flag unsupported claims")
    parser.add_argument("--message", required=True, help="Message to check")
    parser.add_argument("--mode", choices=["warn", "block"], default="warn",
                        help="warn=advisory, block=reject (Phase 2)")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    args = parser.parse_args()

    report = check_message(args.message, mode=args.mode)

    if args.json:
        from scripts.maintainer.lib.report_schema import report_to_dict
        print(json.dumps(report_to_dict(report), indent=2))
    else:
        print(report_to_markdown(report))

    # Exit code
    high = [f for f in report.findings if f.severity in ("critical", "high")]
    if high and args.mode == "block":
        sys.exit(1)
    else:
        sys.exit(0)


if __name__ == "__main__":
    main()
