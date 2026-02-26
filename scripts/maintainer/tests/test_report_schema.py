"""Tests for report_schema — validation, forbidden words, markdown rendering."""
import pytest
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.report_schema import (
    MaintainerReport, Finding, Action, EvidenceRef, FrustrationSignal,
    FORBIDDEN_STATUS_WORDS, SEVERITY_LEVELS, VALID_FL_STATUS,
    validate_report, report_to_dict, report_to_markdown, now_iso,
)


class TestEvidenceRef:
    def test_frozen(self):
        ref = EvidenceRef(kind="commit", value="abc1234")
        with pytest.raises(AttributeError):
            ref.kind = "file"

    def test_fields(self):
        ref = EvidenceRef(kind="file", value="/tmp/x.md", description="test")
        assert ref.kind == "file"
        assert ref.value == "/tmp/x.md"
        assert ref.description == "test"


class TestFinding:
    def test_valid_severity(self):
        for sev in SEVERITY_LEVELS:
            f = Finding(id="T-001", severity=sev, category="test", summary="ok")
            assert f.severity == sev

    def test_invalid_severity_raises(self):
        with pytest.raises(ValueError, match="Invalid severity"):
            Finding(id="T-001", severity="banana", category="test", summary="nope")

    def test_frozen(self):
        f = Finding(id="T-001", severity="high", category="test", summary="ok")
        with pytest.raises(AttributeError):
            f.summary = "changed"


class TestMaintainerReport:
    def _make_report(self, **kwargs):
        defaults = dict(tool_name="test", timestamp=now_iso())
        defaults.update(kwargs)
        return MaintainerReport(**defaults)

    def test_validate_empty_tool_name(self):
        r = self._make_report(tool_name="")
        errors = validate_report(r)
        assert any("tool_name" in e for e in errors)

    def test_validate_empty_timestamp(self):
        r = self._make_report(timestamp="")
        errors = validate_report(r)
        assert any("timestamp" in e for e in errors)

    def test_validate_health_score_range(self):
        r = self._make_report(health_score=150)
        errors = validate_report(r)
        assert any("health_score" in e for e in errors)

    def test_validate_health_score_negative(self):
        r = self._make_report(health_score=-1)
        errors = validate_report(r)
        assert any("health_score" in e for e in errors)

    def test_validate_health_score_valid(self):
        r = self._make_report(health_score=75)
        errors = validate_report(r)
        assert not errors

    def test_validate_forbidden_word_in_summary_without_evidence(self):
        r = self._make_report(summary="Everything is fixed and working")
        errors = validate_report(r)
        assert any("forbidden word" in e for e in errors)

    def test_forbidden_word_with_evidence_ok(self):
        r = self._make_report(
            summary="Fixed with commit abc1234",
            evidence=(EvidenceRef(kind="commit", value="abc1234"),),
        )
        errors = validate_report(r)
        # Should pass — evidence refs present
        forbidden_errors = [e for e in errors if "forbidden" in e]
        assert not forbidden_errors

    def test_clean_report_validates(self):
        r = self._make_report(
            summary="3 findings detected",
            health_score=80,
        )
        errors = validate_report(r)
        assert not errors


class TestSerialization:
    def test_report_to_dict(self):
        r = MaintainerReport(
            tool_name="test",
            timestamp="2026-01-01T00:00:00Z",
            findings=(
                Finding(id="F-1", severity="high", category="test", summary="x"),
            ),
        )
        d = report_to_dict(r)
        assert d["tool_name"] == "test"
        assert len(d["findings"]) == 1
        assert d["findings"][0]["id"] == "F-1"

    def test_report_to_markdown_has_header(self):
        r = MaintainerReport(
            tool_name="janitor",
            timestamp="2026-01-01T00:00:00Z",
            summary="test summary",
        )
        md = report_to_markdown(r)
        assert "# Maintainer Report: janitor" in md
        assert "test summary" in md

    def test_report_to_markdown_findings(self):
        r = MaintainerReport(
            tool_name="test",
            timestamp="2026-01-01T00:00:00Z",
            findings=(
                Finding(id="F-1", severity="critical", category="bug", summary="bad thing"),
            ),
        )
        md = report_to_markdown(r)
        assert "F-1" in md
        assert "bad thing" in md
        assert "critical" in md

    def test_report_to_markdown_actions(self):
        r = MaintainerReport(
            tool_name="test",
            timestamp="2026-01-01T00:00:00Z",
            actions=(
                Action(id="A-1", priority="high", summary="do something"),
            ),
        )
        md = report_to_markdown(r)
        assert "A-1" in md
        assert "do something" in md

    def test_report_to_markdown_frustrations(self):
        r = MaintainerReport(
            tool_name="test",
            timestamp="2026-01-01T00:00:00Z",
            frustrations=(
                FrustrationSignal(
                    signal_type="hypothesis_churn",
                    count=3,
                    examples=("ex1", "ex2"),
                ),
            ),
        )
        md = report_to_markdown(r)
        assert "hypothesis_churn" in md
        assert "ex1" in md


class TestConstants:
    def test_forbidden_words_are_lowercase(self):
        for word in FORBIDDEN_STATUS_WORDS:
            assert word == word.lower(), f"{word!r} should be lowercase"

    def test_fl_status_values(self):
        assert "OPEN" in VALID_FL_STATUS
        assert "RESOLVED" in VALID_FL_STATUS
        assert "PARTIAL" in VALID_FL_STATUS
        assert "MONITORING" in VALID_FL_STATUS

    def test_now_iso_format(self):
        ts = now_iso()
        assert "T" in ts
        assert "+" in ts or "Z" in ts  # timezone info present
