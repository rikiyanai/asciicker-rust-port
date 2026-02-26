#!/usr/bin/env python3
"""Fail-closed evidence gate for Phase 13 closure claims."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
_DEFAULT_FAILURE_LOG = _PROJECT_ROOT / "docs" / "research" / "ascii" / "verification" / "FAILURE_LOG.md"
_DEFAULT_SIGNOFF = _PROJECT_ROOT / "docs" / "research" / "ascii" / "verification" / "manual-image-inspection-signoff.json"

_REQUIRED_OUTPUT_GATES = {"G7_output_occupancy", "G8_output_coherence", "G9_output_degenerate"}


def _load_json(path: Path) -> dict:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"{path}: invalid JSON ({exc})") from exc
    if not isinstance(data, dict):
        raise ValueError(f"{path}: expected JSON object")
    return data


def _validate_gate_report(path: Path) -> list[str]:
    errs: list[str] = []
    try:
        data = _load_json(path)
    except ValueError as exc:
        return [str(exc)]

    gate_class = str(data.get("gate_class", "")).strip().lower()
    if gate_class != "output":
        errs.append(f"{path}: gate_class must be 'output'")

    gates = data.get("gates")
    if not isinstance(gates, list) or not gates:
        errs.append(f"{path}: missing non-empty 'gates' list")
        return errs

    seen = {
        str(item.get("gate", "")).strip()
        for item in gates
        if isinstance(item, dict)
    }
    missing = sorted(_REQUIRED_OUTPUT_GATES - seen)
    if missing:
        errs.append(f"{path}: missing required gates {', '.join(missing)}")
    return errs


def _validate_signoff(path: Path, xp_paths: list[Path]) -> list[str]:
    errs: list[str] = []
    try:
        data = _load_json(path)
    except ValueError as exc:
        return [str(exc)]

    if data.get("approved") is not True:
        errs.append(f"{path}: approved must be true")

    inspector_type = str(data.get("inspector_type", "")).strip().lower()
    if inspector_type != "human":
        errs.append(f"{path}: inspector_type must be 'human'")

    reviewer = str(data.get("reviewer", "")).strip()
    if not reviewer:
        errs.append(f"{path}: reviewer is required")

    notes = str(data.get("notes", "")).strip()
    if not notes:
        errs.append(f"{path}: notes are required")

    artifacts = data.get("inspected_artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        errs.append(f"{path}: inspected_artifacts must be a non-empty list")
        return errs

    artifact_set = {str(item).strip() for item in artifacts if str(item).strip()}
    if not artifact_set:
        errs.append(f"{path}: inspected_artifacts entries are empty")
        return errs

    # Require at least one XP output to be included in reviewed artifacts.
    xp_strings = {str(p) for p in xp_paths}
    if xp_strings and artifact_set.isdisjoint(xp_strings):
        errs.append(f"{path}: inspected_artifacts must include at least one provided --xp path")
    return errs


def run_truth_gate(
    xp_paths: list[Path],
    gate_reports: list[Path],
    previews: list[Path],
    signoff_path: Path,
    failure_log_path: Path,
    failure_log_ref: str,
) -> tuple[bool, list[str]]:
    errs: list[str] = []

    if not xp_paths:
        errs.append("at least one --xp is required")
    if not gate_reports:
        errs.append("at least one --gate-report is required")
    if not previews:
        errs.append("at least one --preview is required")

    for path in xp_paths:
        if not path.exists():
            errs.append(f"missing xp file: {path}")
        elif path.stat().st_size == 0:
            errs.append(f"empty xp file: {path}")

    for path in previews:
        if not path.exists():
            errs.append(f"missing preview artifact: {path}")
        elif path.stat().st_size == 0:
            errs.append(f"empty preview artifact: {path}")

    for path in gate_reports:
        if not path.exists():
            errs.append(f"missing gate report: {path}")
            continue
        errs.extend(_validate_gate_report(path))

    if not signoff_path.exists():
        errs.append(f"missing signoff file: {signoff_path}")
    else:
        errs.extend(_validate_signoff(signoff_path, xp_paths))

    if not failure_log_path.exists():
        errs.append(f"missing failure log: {failure_log_path}")
    else:
        content = failure_log_path.read_text(encoding="utf-8", errors="replace")
        if failure_log_ref not in content:
            errs.append(
                f"failure log reference not found: {failure_log_ref} in {failure_log_path}"
            )

    return len(errs) == 0, errs


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Phase 13 truth gate (fail-closed).")
    parser.add_argument("--xp", action="append", default=[], help="XP output path (repeatable).")
    parser.add_argument(
        "--gate-report",
        action="append",
        default=[],
        help="Output quality gate report JSON path (repeatable).",
    )
    parser.add_argument(
        "--preview",
        action="append",
        default=[],
        help="Preview/contact-sheet artifact path (repeatable).",
    )
    parser.add_argument(
        "--signoff-path",
        default=str(_DEFAULT_SIGNOFF),
        help="Human visual signoff JSON path.",
    )
    parser.add_argument(
        "--failure-log-path",
        default=str(_DEFAULT_FAILURE_LOG),
        help="Canonical failure log path.",
    )
    parser.add_argument(
        "--failure-log-ref",
        required=True,
        help="Required failure-log reference (for example: FL-015).",
    )
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON result.")
    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()

    ok, errors = run_truth_gate(
        xp_paths=[Path(p) for p in args.xp],
        gate_reports=[Path(p) for p in args.gate_report],
        previews=[Path(p) for p in args.preview],
        signoff_path=Path(args.signoff_path),
        failure_log_path=Path(args.failure_log_path),
        failure_log_ref=str(args.failure_log_ref).strip(),
    )

    if args.json:
        print(json.dumps({"ok": ok, "errors": errors}, indent=2))
    elif ok:
        print("THRESHOLD_MET: phase13 truth gate evidence bundle is complete")
    else:
        print("THRESHOLD_BREACHED: phase13 truth gate evidence bundle is incomplete")
        for err in errors:
            print(f"- {err}")

    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
