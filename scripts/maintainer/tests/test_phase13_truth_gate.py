"""Tests for phase13_truth_gate fail-closed evidence checks."""
from __future__ import annotations

import json
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.phase13_truth_gate import run_truth_gate


def _write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def _gate_payload(include_g9: bool = True) -> dict:
    gates = [
        {"gate": "G7_output_occupancy", "verdict": "THRESHOLD_MET"},
        {"gate": "G8_output_coherence", "verdict": "THRESHOLD_BREACHED"},
    ]
    if include_g9:
        gates.append({"gate": "G9_output_degenerate", "verdict": "THRESHOLD_MET"})
    return {
        "gate_class": "output",
        "all_thresholds_met": True,
        "gates": gates,
    }


def _signoff_payload(xp_path: Path) -> dict:
    return {
        "approved": True,
        "inspector_type": "human",
        "reviewer": "r",
        "inspected_at": "2026-02-21T00:00:00Z",
        "inspected_artifacts": [str(xp_path)],
        "notes": "visual check complete",
    }


class TestPhase13TruthGate:
    def test_passes_with_complete_bundle(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload(include_g9=True))
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(xp))
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: Test entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )

        assert ok is True
        assert errors == []

    def test_fails_when_required_gate_missing(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload(include_g9=False))
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(xp))
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: Test entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )

        assert ok is False
        assert any("missing required gates" in err for err in errors)

    def test_fails_when_failure_log_ref_missing(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload(include_g9=True))
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(xp))
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-998: Other entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )

        assert ok is False
        assert any("reference not found" in err for err in errors)

    def test_fails_when_no_xp_paths(self, tmp_path):
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload())
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(tmp_path / "dummy.xp"))
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )
        assert ok is False
        assert any("at least one --xp" in err for err in errors)

    def test_fails_when_xp_file_missing(self, tmp_path):
        missing_xp = tmp_path / "missing.xp"
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload())
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(missing_xp))
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[missing_xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )
        assert ok is False
        assert any("missing xp file" in err for err in errors)

    def test_fails_when_empty_xp_file(self, tmp_path):
        xp = tmp_path / "empty.xp"
        xp.write_bytes(b"")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload())
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(xp))
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )
        assert ok is False
        assert any("empty xp file" in err for err in errors)

    def test_fails_when_no_previews(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload())
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(xp))
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )
        assert ok is False
        assert any("at least one --preview" in err for err in errors)

    def test_fails_when_signoff_not_approved(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload())
        signoff = tmp_path / "signoff.json"
        payload = _signoff_payload(xp)
        payload["approved"] = False
        _write_json(signoff, payload)
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )
        assert ok is False
        assert any("approved must be true" in err for err in errors)

    def test_fails_when_signoff_not_human(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload())
        signoff = tmp_path / "signoff.json"
        payload = _signoff_payload(xp)
        payload["inspector_type"] = "agent"
        _write_json(signoff, payload)
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )
        assert ok is False
        assert any("inspector_type must be 'human'" in err for err in errors)

    def test_fails_when_gate_report_wrong_class(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        bad_payload = _gate_payload()
        bad_payload["gate_class"] = "input"
        _write_json(gate, bad_payload)
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(xp))
        fl = tmp_path / "FAILURE_LOG.md"
        fl.write_text("### FL-999: entry\n", encoding="utf-8")

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=fl,
            failure_log_ref="FL-999",
        )
        assert ok is False
        assert any("gate_class must be 'output'" in err for err in errors)

    def test_fails_when_failure_log_missing(self, tmp_path):
        xp = tmp_path / "out.xp"
        xp.write_bytes(b"xp")
        preview = tmp_path / "preview.png"
        preview.write_bytes(b"png")
        gate = tmp_path / "quality.json"
        _write_json(gate, _gate_payload())
        signoff = tmp_path / "signoff.json"
        _write_json(signoff, _signoff_payload(xp))

        ok, errors = run_truth_gate(
            xp_paths=[xp],
            gate_reports=[gate],
            previews=[preview],
            signoff_path=signoff,
            failure_log_path=tmp_path / "nonexistent_FL.md",
            failure_log_ref="FL-999",
        )
        assert ok is False
        assert any("missing failure log" in err for err in errors)
