"""Maintainer report schema — frozen dataclasses for structured findings.

Every maintainer tool produces a MaintainerReport.  Reports contain Findings
(things discovered), Actions (things to do), and EvidenceRefs (proof anchors).
Validation rejects unsupported status language to prevent false-completion claims.
"""
from __future__ import annotations

import json
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from typing import Optional

# --- Constants ----------------------------------------------------------------

VALID_METRIC_STATUS = frozenset({"pass", "fail", "partial", "skip", "unknown"})

VALID_QUALITY_VERDICT = frozenset({
    "pass", "fail", "partial", "deferred", "blocked",
})

VALID_FL_STATUS = frozenset({"OPEN", "PARTIAL", "MONITORING", "RESOLVED"})

# Words that MUST NOT appear in status/verdict fields without evidence refs.
# These were the exact patterns used in 13 cycles of false completion claims.
FORBIDDEN_STATUS_WORDS = frozenset({
    "fixed", "resolved", "complete", "done", "closed",
    "passed", "working", "green", "shipped",
})

# Canonical threshold vocabulary per CLAUDE.md claim discipline rule #3.
# Gate results MUST use these terms; "pass"/"fail" are forbidden for gate verdicts.
VALID_THRESHOLD_VERDICT = frozenset({
    "THRESHOLD_MET",
    "THRESHOLD_BREACHED",
})

SEVERITY_LEVELS = ("critical", "high", "medium", "low", "info")


# --- Dataclasses --------------------------------------------------------------

@dataclass(frozen=True)
class EvidenceRef:
    """Pointer to a concrete artifact that supports a claim."""
    kind: str          # "commit", "file", "test_output", "log_entry", "url"
    value: str         # e.g. commit hash, file path, URL
    description: str = ""


@dataclass(frozen=True)
class Finding:
    """A single observation from a maintainer tool."""
    id: str                         # e.g. "JAN-001", "CLM-003"
    severity: str                   # one of SEVERITY_LEVELS
    category: str                   # e.g. "hypothesis_churn", "stale_open"
    summary: str
    details: str = ""
    evidence: tuple[EvidenceRef, ...] = ()

    def __post_init__(self):
        if self.severity not in SEVERITY_LEVELS:
            raise ValueError(
                f"Invalid severity {self.severity!r}, "
                f"must be one of {SEVERITY_LEVELS}"
            )


@dataclass(frozen=True)
class Action:
    """A recommended action from a maintainer tool."""
    id: str
    priority: str       # "critical", "high", "medium", "low"
    summary: str
    target: str = ""    # file path, function name, or doc section


@dataclass(frozen=True)
class FrustrationSignal:
    """A detected frustration or churn signal from session analysis."""
    signal_type: str    # "hypothesis_churn", "repeated_failure", "goal_drift"
    count: int
    examples: tuple[str, ...] = ()
    first_seen: str = ""
    last_seen: str = ""


@dataclass(frozen=True)
class MaintainerReport:
    """Top-level report produced by any maintainer tool."""
    tool_name: str                           # e.g. "janitor", "claim_guard"
    timestamp: str                           # ISO 8601
    mode: str = "warn"                       # "warn" or "block"
    dry_run: bool = False
    findings: tuple[Finding, ...] = ()
    actions: tuple[Action, ...] = ()
    frustrations: tuple[FrustrationSignal, ...] = ()
    evidence: tuple[EvidenceRef, ...] = ()   # report-level evidence
    health_score: Optional[int] = None       # 0-100, used by audit
    recommendation: str = ""                 # e.g. "continue", "pause"
    summary: str = ""
    metadata: dict = field(default_factory=dict)


# --- Validation ---------------------------------------------------------------

def validate_report(report: MaintainerReport) -> list[str]:
    """Return list of validation errors (empty = valid)."""
    errors: list[str] = []

    if not report.tool_name:
        errors.append("tool_name is required")

    if not report.timestamp:
        errors.append("timestamp is required")

    # Check severity on all findings
    for f in report.findings:
        if f.severity not in SEVERITY_LEVELS:
            errors.append(f"Finding {f.id}: invalid severity {f.severity!r}")

    # Check health_score range
    if report.health_score is not None:
        if not (0 <= report.health_score <= 100):
            errors.append(
                f"health_score must be 0-100, got {report.health_score}"
            )

    # Check for forbidden words in summary without evidence
    if report.summary:
        summary_lower = report.summary.lower()
        for word in FORBIDDEN_STATUS_WORDS:
            if word in summary_lower and not report.evidence:
                errors.append(
                    f"Summary contains forbidden word {word!r} "
                    f"without evidence refs"
                )

    return errors


# --- Serialization ------------------------------------------------------------

def report_to_dict(report: MaintainerReport) -> dict:
    """Convert report to a JSON-serializable dict."""
    return asdict(report)


def report_to_markdown(report: MaintainerReport) -> str:
    """Render report as human-readable Markdown."""
    lines: list[str] = []
    lines.append(f"# Maintainer Report: {report.tool_name}")
    lines.append(f"")
    lines.append(f"**Generated:** {report.timestamp}")
    lines.append(f"**Mode:** {report.mode}")
    if report.dry_run:
        lines.append(f"**Dry Run:** yes")
    if report.health_score is not None:
        lines.append(f"**Health Score:** {report.health_score}/100")
    if report.recommendation:
        lines.append(f"**Recommendation:** {report.recommendation}")
    lines.append("")

    if report.summary:
        lines.append(f"## Summary")
        lines.append(f"")
        lines.append(report.summary)
        lines.append("")

    if report.findings:
        lines.append(f"## Findings ({len(report.findings)})")
        lines.append("")
        for f in report.findings:
            icon = {"critical": "!!!", "high": "!!", "medium": "!",
                    "low": "~", "info": "i"}.get(f.severity, "?")
            lines.append(f"### [{icon}] {f.id}: {f.summary}")
            lines.append(f"- **Severity:** {f.severity}")
            lines.append(f"- **Category:** {f.category}")
            if f.details:
                lines.append(f"- **Details:** {f.details}")
            if f.evidence:
                lines.append(f"- **Evidence:**")
                for e in f.evidence:
                    desc = f" — {e.description}" if e.description else ""
                    lines.append(f"  - [{e.kind}] `{e.value}`{desc}")
            lines.append("")

    if report.actions:
        lines.append(f"## Actions ({len(report.actions)})")
        lines.append("")
        for a in report.actions:
            lines.append(f"- **{a.id}** [{a.priority}]: {a.summary}")
            if a.target:
                lines.append(f"  - Target: `{a.target}`")
        lines.append("")

    if report.frustrations:
        lines.append(f"## Frustration Signals ({len(report.frustrations)})")
        lines.append("")
        for fr in report.frustrations:
            lines.append(f"- **{fr.signal_type}** (count: {fr.count})")
            if fr.examples:
                for ex in fr.examples[:3]:
                    lines.append(f"  - {ex}")
        lines.append("")

    if report.evidence:
        lines.append(f"## Report-Level Evidence")
        lines.append("")
        for e in report.evidence:
            desc = f" — {e.description}" if e.description else ""
            lines.append(f"- [{e.kind}] `{e.value}`{desc}")
        lines.append("")

    return "\n".join(lines)


def now_iso() -> str:
    """Return current UTC timestamp in ISO 8601."""
    return datetime.now(timezone.utc).isoformat()
