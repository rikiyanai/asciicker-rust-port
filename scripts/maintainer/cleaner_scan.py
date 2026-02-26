#!/usr/bin/env python3
"""Cleaner scan — detect stale and redundant artifacts in the repo.

Scan-only mode: identifies candidates for cleanup without modifying anything.
Phase 3 will add `cleaner_apply.py` for approved removals.

Usage:
    python3 scripts/maintainer/cleaner_scan.py                    # Full scan
    python3 scripts/maintainer/cleaner_scan.py --dry-run           # Same as full (scan is always read-only)
    python3 scripts/maintainer/cleaner_scan.py --json              # Output as JSON manifest
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.report_schema import (
    MaintainerReport, Finding, EvidenceRef, now_iso, report_to_markdown,
)

# Artifact categories and their known-safe patterns
STALE_PATTERNS = {
    "backup_dirs": {
        "globs": ["*.backup/", "*.bak"],
        "description": "Backup directories and files",
        "safe_to_remove": True,
    },
    "debug_output": {
        "globs": ["*.debug.log", "debug_*.txt", "*.trace.jsonl"],
        "description": "Debug and trace output files",
        "safe_to_remove": True,
    },
    "temp_tests": {
        "globs": ["test_scratch_*.py", "test_temp_*.py", "test_debug_*.py"],
        "description": "Temporary test files",
        "safe_to_remove": False,  # Proposal only
    },
    "stale_staging": {
        "paths": ["scripts/staging/", "scripts/asset_gen/staging/"],
        "description": "Pipeline staging output (gitignored but may accumulate)",
        "safe_to_remove": True,
    },
    "pycache": {
        "globs": ["**/__pycache__/"],
        "description": "Python bytecode cache",
        "safe_to_remove": True,
    },
    "coverage": {
        "globs": [".coverage", "htmlcov/"],
        "description": "Test coverage artifacts",
        "safe_to_remove": True,
    },
}

# Directories to search
SCAN_DIRS = [
    "scripts/",
    "docs/",
    "tests/",
    "artifacts/",
]

# Files to check for duplication
DUPLICATE_CHECK_DIRS = [
    "scripts/asset_gen/",
    "scripts/maintainer/",
]


def _find_by_glob(root: Path, pattern: str) -> list[Path]:
    """Find files matching a glob pattern under root."""
    return sorted(root.glob(pattern))


def _file_age_days(path: Path) -> float:
    """Age of file in days since last modification."""
    mtime = datetime.fromtimestamp(path.stat().st_mtime, tz=timezone.utc)
    now = datetime.now(timezone.utc)
    return (now - mtime).total_seconds() / 86400


def _file_hash(path: Path) -> str:
    """SHA256 of file contents."""
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def scan_stale_artifacts(root: Path) -> list[dict]:
    """Scan for stale artifacts matching known patterns."""
    candidates = []

    for category, spec in STALE_PATTERNS.items():
        globs = spec.get("globs", [])
        paths = spec.get("paths", [])

        found: list[Path] = []
        for g in globs:
            found.extend(_find_by_glob(root, g))
        for p in paths:
            full = root / p
            if full.exists():
                if full.is_dir():
                    found.extend(f for f in full.rglob("*") if f.is_file())
                else:
                    found.append(full)

        for f in found:
            candidates.append({
                "category": category,
                "path": str(f.relative_to(root)),
                "size_bytes": f.stat().st_size if f.is_file() else 0,
                "age_days": round(_file_age_days(f), 1) if f.is_file() else 0,
                "safe_to_remove": spec["safe_to_remove"],
                "description": spec["description"],
            })

    return candidates


def scan_duplicate_utils(root: Path) -> list[dict]:
    """Find potential duplicate utility functions across Python files."""
    duplicates = []
    seen_hashes: dict[str, list[str]] = {}

    for scan_dir in DUPLICATE_CHECK_DIRS:
        full_dir = root / scan_dir
        if not full_dir.exists():
            continue

        for py_file in full_dir.rglob("*.py"):
            if "__pycache__" in str(py_file):
                continue
            try:
                content = py_file.read_text()
            except (OSError, UnicodeDecodeError):
                continue

            h = _file_hash(py_file)
            rel = str(py_file.relative_to(root))
            if h in seen_hashes:
                seen_hashes[h].append(rel)
            else:
                seen_hashes[h] = [rel]

    for h, files in seen_hashes.items():
        if len(files) > 1:
            duplicates.append({
                "hash": h,
                "files": files,
                "description": "Files with identical content",
            })

    return duplicates


def scan_conflicting_docs(root: Path) -> list[dict]:
    """Find docs with potentially conflicting status claims."""
    conflicts = []
    status_docs = list((root / "docs" / "plans").glob("*.md"))
    status_docs.extend((root / ".planning").glob("*.md"))

    # Look for docs claiming different statuses for the same topic
    seen_topics: dict[str, list[dict]] = {}
    for doc in status_docs:
        try:
            content = doc.read_text()
        except (OSError, UnicodeDecodeError):
            continue

        # Extract status lines
        for line in content.split("\n"):
            lower = line.lower().strip()
            if "status:" in lower:
                rel = str(doc.relative_to(root))
                topic = doc.stem
                entry = {"path": rel, "status_line": line.strip()[:120]}
                if topic in seen_topics:
                    seen_topics[topic].append(entry)
                else:
                    seen_topics[topic] = [entry]

    for topic, entries in seen_topics.items():
        if len(entries) > 1:
            statuses = {e["status_line"] for e in entries}
            if len(statuses) > 1:
                conflicts.append({
                    "topic": topic,
                    "entries": entries,
                    "description": "Multiple docs with different status claims",
                })

    return conflicts


def run_scan(root: Path) -> dict:
    """Run full scan and return structured results."""
    stale = scan_stale_artifacts(root)
    dupes = scan_duplicate_utils(root)
    conflicts = scan_conflicting_docs(root)

    return {
        "timestamp": now_iso(),
        "root": str(root),
        "stale_artifacts": stale,
        "duplicate_utils": dupes,
        "conflicting_docs": conflicts,
        "totals": {
            "stale_count": len(stale),
            "stale_safe_count": len([s for s in stale if s["safe_to_remove"]]),
            "duplicate_groups": len(dupes),
            "doc_conflicts": len(conflicts),
        },
    }


def scan_to_report(scan_result: dict) -> MaintainerReport:
    """Convert scan results to a MaintainerReport."""
    findings: list[Finding] = []
    totals = scan_result["totals"]

    if totals["stale_count"] > 0:
        safe = totals["stale_safe_count"]
        findings.append(Finding(
            id="CLN-001",
            severity="info",
            category="stale_artifacts",
            summary=f"{totals['stale_count']} stale artifacts found ({safe} safe to remove)",
        ))

    if totals["duplicate_groups"] > 0:
        findings.append(Finding(
            id="CLN-002",
            severity="info",
            category="duplicate_utils",
            summary=f"{totals['duplicate_groups']} duplicate file groups found",
        ))

    if totals["doc_conflicts"] > 0:
        findings.append(Finding(
            id="CLN-003",
            severity="high",
            category="doc_conflicts",
            summary=f"{totals['doc_conflicts']} docs with conflicting status claims",
        ))

    return MaintainerReport(
        tool_name="cleaner_scan",
        timestamp=scan_result["timestamp"],
        mode="warn",
        findings=tuple(findings),
        summary=(
            f"Cleaner scan: {totals['stale_count']} stale, "
            f"{totals['duplicate_groups']} dupes, "
            f"{totals['doc_conflicts']} conflicts"
        ),
    )


def main():
    parser = argparse.ArgumentParser(description="Cleaner scan — detect stale artifacts")
    parser.add_argument("--dry-run", action="store_true",
                        help="Scan is always read-only; flag accepted for consistency")
    parser.add_argument("--json", action="store_true",
                        help="Output full scan manifest as JSON")
    args = parser.parse_args()

    scan_result = run_scan(_PROJECT_ROOT)

    if args.json:
        print(json.dumps(scan_result, indent=2))
    else:
        report = scan_to_report(scan_result)
        md = report_to_markdown(report)

        if args.dry_run:
            # Dry-run: print report to stdout only, no artifact writes
            print(md)
        else:
            # Write artifact
            artifact_dir = _PROJECT_ROOT / "artifacts" / "maintainer"
            artifact_dir.mkdir(parents=True, exist_ok=True)
            ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
            artifact_path = artifact_dir / f"cleaner_scan_{ts}.md"
            artifact_path.write_text(md)
            print(f"Report written to {artifact_path}")

            # Also write JSON manifest for cleaner_apply.py (Phase 3)
            manifest_path = artifact_dir / f"cleanup_manifest_{ts}.json"
            manifest_path.write_text(json.dumps(scan_result, indent=2))
            print(f"Manifest written to {manifest_path}")

    sys.exit(0)


if __name__ == "__main__":
    main()
