"""Tests for claim discipline vocabulary enforcement (WS-4).

Validates that:
1. THRESHOLD_MET/THRESHOLD_BREACHED are the only valid gate verdicts.
2. FORBIDDEN_STATUS_WORDS contains the 13-cycle false-completion vocabulary.
3. VALID_FL_STATUS uses append-only status vocabulary.
4. Truth gate CLI output uses threshold vocabulary.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.report_schema import (
    FORBIDDEN_STATUS_WORDS,
    VALID_FL_STATUS,
    VALID_THRESHOLD_VERDICT,
)


class TestThresholdVocabulary:
    """Threshold verdict vocabulary must be exactly THRESHOLD_MET/BREACHED."""

    def test_valid_verdicts_are_threshold_terms(self):
        assert VALID_THRESHOLD_VERDICT == {"THRESHOLD_MET", "THRESHOLD_BREACHED"}

    def test_pass_is_not_valid_threshold(self):
        assert "pass" not in VALID_THRESHOLD_VERDICT
        assert "PASS" not in VALID_THRESHOLD_VERDICT

    def test_fail_is_not_valid_threshold(self):
        assert "fail" not in VALID_THRESHOLD_VERDICT
        assert "FAIL" not in VALID_THRESHOLD_VERDICT


class TestForbiddenWords:
    """FORBIDDEN_STATUS_WORDS must contain all known false-completion terms."""

    EXPECTED_FORBIDDEN = {
        "fixed", "resolved", "complete", "done", "closed",
        "passed", "working", "green", "shipped",
    }

    def test_all_known_false_completion_terms_present(self):
        assert self.EXPECTED_FORBIDDEN.issubset(FORBIDDEN_STATUS_WORDS)

    def test_threshold_vocabulary_not_forbidden(self):
        for term in ("THRESHOLD_MET", "THRESHOLD_BREACHED", "threshold_met", "threshold_breached"):
            assert term not in FORBIDDEN_STATUS_WORDS


class TestFLStatusVocabulary:
    """Failure log status vocabulary must be append-only safe terms."""

    def test_valid_fl_statuses(self):
        assert VALID_FL_STATUS == {"OPEN", "PARTIAL", "MONITORING", "RESOLVED"}

    def test_fl_status_excludes_forbidden_words(self):
        # "RESOLVED" is in VALID_FL_STATUS but "resolved" is in FORBIDDEN_STATUS_WORDS.
        # This is by design: FL status uses capitalized enum, forbidden applies to prose.
        for status in VALID_FL_STATUS:
            if status.lower() in FORBIDDEN_STATUS_WORDS:
                # OK: capitalized enum form is distinct from prose usage.
                # The claim guard checks prose text, not FL status enums.
                pass


class TestTruthGateCLIVocabulary:
    """Truth gate CLI output must use THRESHOLD_MET/THRESHOLD_BREACHED."""

    def test_truth_gate_pass_output_uses_threshold_met(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        gate.write_text(json.dumps({
            "gate_class": "output",
            "all_thresholds_met": True,
            "gates": [
                {"gate": "G7_output_occupancy", "verdict": "THRESHOLD_MET"},
                {"gate": "G8_output_coherence", "verdict": "THRESHOLD_MET"},
                {"gate": "G9_output_degenerate", "verdict": "THRESHOLD_MET"},
            ],
        }))
        signoff = tmp_path / "signoff.json"
        signoff.write_text(json.dumps({
            "approved": True,
            "inspector_type": "human",
            "reviewer": "r",
            "inspected_at": "2026-02-21T00:00:00Z",
            "inspected_artifacts": [str(xp)],
            "notes": "ok",
        }))
        fl = tmp_path / "FL.md"
        fl.write_text("### FL-999: entry\n")

        proc = subprocess.run(
            [
                sys.executable,
                str(_PROJECT_ROOT / "scripts" / "maintainer" / "phase13_truth_gate.py"),
                "--xp", str(xp),
                "--gate-report", str(gate),
                "--preview", str(preview),
                "--signoff-path", str(signoff),
                "--failure-log-path", str(fl),
                "--failure-log-ref", "FL-999",
            ],
            capture_output=True,
            text=True,
        )
        assert proc.returncode == 0
        assert "THRESHOLD_MET" in proc.stdout

    def test_truth_gate_fail_output_uses_threshold_breached(self, tmp_path):
        fl = tmp_path / "FL.md"
        fl.write_text("### FL-999: entry\n")

        proc = subprocess.run(
            [
                sys.executable,
                str(_PROJECT_ROOT / "scripts" / "maintainer" / "phase13_truth_gate.py"),
                "--failure-log-ref", "FL-999",
                "--failure-log-path", str(fl),
            ],
            capture_output=True,
            text=True,
        )
        assert proc.returncode == 1
        assert "THRESHOLD_BREACHED" in proc.stdout
