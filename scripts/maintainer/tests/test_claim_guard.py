"""Tests for claim_guard — forbidden words, missing FL refs, clean pass."""
import sys
from pathlib import Path

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.claim_guard import check_message
from scripts.maintainer.lib.report_schema import FORBIDDEN_STATUS_WORDS


class TestCleanMessages:
    def test_neutral_message_passes(self):
        report = check_message("Investigating the slicer crash on manul.png")
        assert report.summary
        assert not any(
            f.category == "unsupported_claim" for f in report.findings
        )

    def test_technical_message_passes(self):
        report = check_message("Added ceiling-division guard in slicer.py")
        assert not any(
            f.category == "unsupported_claim" for f in report.findings
        )


class TestForbiddenWithoutEvidence:
    def test_resolved_without_ref(self):
        report = check_message("fix(pipeline): resolved the slicer crash")
        findings = [f for f in report.findings
                    if f.category == "unsupported_claim"]
        assert len(findings) == 1
        assert "resolved" in findings[0].summary.lower()

    def test_fixed_without_ref(self):
        report = check_message("Everything is fixed now")
        findings = [f for f in report.findings
                    if f.category == "unsupported_claim"]
        assert len(findings) >= 1

    def test_complete_without_ref(self):
        report = check_message("Pipeline is complete and working")
        findings = [f for f in report.findings
                    if f.category == "unsupported_claim"]
        assert len(findings) >= 1

    def test_multiple_forbidden_words(self):
        report = check_message("All fixed, resolved, and done!")
        findings = [f for f in report.findings
                    if f.category == "unsupported_claim"]
        assert len(findings) >= 1
        # Summary should mention all found words
        summary = findings[0].summary.lower()
        assert "fixed" in summary or "resolved" in summary or "done" in summary


class TestForbiddenWithEvidence:
    def test_fl_ref_passes(self):
        report = check_message("FL-001 partial fix for slicer crash, resolved with guard")
        # Should be "claim_with_evidence", not "unsupported_claim"
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_commit_hash_passes(self):
        report = check_message("Fixed in commit abc1234def")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_both_refs(self):
        report = check_message("FL-002 resolved in abc1234, all green")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0


class TestModes:
    def test_warn_mode_default(self):
        report = check_message("everything is resolved")
        assert report.mode == "warn"

    def test_block_mode(self):
        report = check_message("everything is resolved", mode="block")
        assert report.mode == "block"


class TestEdgeCases:
    def test_empty_message(self):
        report = check_message("")
        assert not any(
            f.category == "unsupported_claim" for f in report.findings
        )

    def test_case_insensitive(self):
        report = check_message("RESOLVED everything FIXED")
        findings = [f for f in report.findings
                    if f.category == "unsupported_claim"]
        assert len(findings) >= 1

    def test_all_forbidden_words_detected(self):
        """Every word in FORBIDDEN_STATUS_WORDS triggers when used alone."""
        for word in FORBIDDEN_STATUS_WORDS:
            report = check_message(f"The issue is {word}")
            has_finding = any(
                f.category in ("unsupported_claim", "claim_with_evidence")
                for f in report.findings
            )
            assert has_finding, f"Word {word!r} was not detected"


class TestWordBoundary:
    """Verify word-boundary matching avoids false positives."""

    def test_completeness_not_flagged(self):
        """'completeness' should NOT trigger 'complete'."""
        report = check_message("Checking completeness of the test suite")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_closed_form_not_flagged(self):
        """'closed-form' should NOT trigger 'closed' — hyphen compounds excluded."""
        report = check_message("Using a closed-form solution for the equation")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_fail_safe_not_flagged(self):
        """'fail-safe' should NOT trigger 'fail'."""
        report = check_message("The system has a fail-safe mechanism")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_working_directory_not_flagged(self):
        """'working-directory' should NOT trigger 'working'."""
        report = check_message("Changed the working-directory path")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_passing_not_flagged(self):
        """'passing' should NOT trigger 'passed'."""
        report = check_message("The values are passing through the filter")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_greenhouse_not_flagged(self):
        """'greenhouse' should NOT trigger 'green'."""
        report = check_message("Monitoring greenhouse gas emissions")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_workaround_not_flagged(self):
        """'workaround' should NOT trigger 'working'."""
        report = check_message("Applied a workaround for the issue")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) == 0

    def test_exact_word_still_flagged(self):
        """Exact forbidden words should still trigger."""
        report = check_message("Everything is complete")
        unsupported = [f for f in report.findings
                       if f.category == "unsupported_claim"]
        assert len(unsupported) >= 1
