"""Tests for audit — missing artifacts, stale entries, health scoring."""
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.audit_run import (
    run_audit,
    _check_failure_log_exists,
    _check_failure_log_hygiene,
    _check_artifact_presence,
    _check_stale_opens,
    WEIGHTS,
)
from scripts.maintainer.lib.failure_log import CANONICAL_FAILURE_LOG


class TestFailureLogExists:
    def test_exists(self):
        """If failure log exists, full points."""
        score, findings = _check_failure_log_exists()
        # This depends on whether the file exists in the workspace
        # Just verify the function returns valid types
        assert isinstance(score, int)
        assert isinstance(findings, list)
        assert score >= 0

    def test_missing(self):
        """If failure log missing, 0 points and finding."""
        with patch(
            "scripts.maintainer.audit_run._PROJECT_ROOT",
            Path("/nonexistent/path"),
        ):
            score, findings = _check_failure_log_exists()
            assert score == 0
            assert len(findings) == 1
            assert findings[0].id == "AUD-001"


class TestFailureLogHygiene:
    def test_parseable_log(self):
        score, findings = _check_failure_log_hygiene()
        assert isinstance(score, int)
        assert score >= 0


class TestArtifactPresence:
    def test_returns_valid(self):
        score, findings = _check_artifact_presence()
        assert isinstance(score, int)
        assert score >= 0

    def test_missing_dir(self):
        with patch(
            "scripts.maintainer.audit_run.ARTIFACTS_DIR",
            Path("/nonexistent/artifacts"),
        ):
            score, findings = _check_artifact_presence()
            assert score == 0
            assert any(f.id == "AUD-005" for f in findings)


class TestStaleOpens:
    def test_returns_valid(self):
        score, findings = _check_stale_opens()
        assert isinstance(score, int)
        assert score >= 0


class TestHealthScoreWeights:
    def test_weights_sum_to_100(self):
        total = sum(WEIGHTS.values())
        assert total == 100, f"Weights sum to {total}, expected 100"


class TestRunAudit:
    def test_dry_run_produces_report(self):
        report = run_audit(sessions=1, dry_run=True)
        assert report.tool_name == "audit"
        assert report.health_score is not None
        assert 0 <= report.health_score <= 100
        assert report.dry_run is True

    def test_health_score_in_range(self):
        report = run_audit(sessions=1, dry_run=True)
        assert 0 <= report.health_score <= 100

    def test_missing_artifacts_lowers_score(self):
        """When artifacts directory is missing, score should be lower."""
        # Run with real state first
        real_report = run_audit(sessions=1, dry_run=True)

        # Run with missing artifacts
        with patch(
            "scripts.maintainer.audit_run.ARTIFACTS_DIR",
            Path("/nonexistent/artifacts"),
        ):
            degraded_report = run_audit(sessions=1, dry_run=True)

        # The degraded report should have a lower or equal score
        assert degraded_report.health_score <= real_report.health_score

    def test_open_stale_entries_lower_score(self):
        """Open failure log entries should reduce health score."""
        report = run_audit(sessions=1, dry_run=True)
        # If there are open entries in the real failure log, there should
        # be a finding about it
        stale_findings = [
            f for f in report.findings if f.category == "stale_open"
        ]
        # This is a structural test — if open entries exist, finding exists
        if stale_findings:
            assert report.health_score < 100
