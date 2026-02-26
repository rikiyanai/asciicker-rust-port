#!/usr/bin/env python3
"""Cleaner apply — remove safe stale artifacts identified by cleaner_scan.

Reads a cleanup manifest (JSON from cleaner_scan.py) and removes artifacts
that are marked safe_to_remove. Default mode is dry-run; pass --execute to
actually delete files.

Usage:
    python3 scripts/maintainer/cleaner_apply.py                         # Dry-run, latest manifest
    python3 scripts/maintainer/cleaner_apply.py --manifest path.json    # Dry-run, specific manifest
    python3 scripts/maintainer/cleaner_apply.py --execute               # Actually delete safe items
    python3 scripts/maintainer/cleaner_apply.py --execute --json        # Delete + JSON output
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tarfile
from datetime import datetime, timezone
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.report_schema import (
    MaintainerReport, Finding, EvidenceRef, Action, now_iso, report_to_markdown,
)

# Categories considered safe for automatic removal.
# Must match safe_to_remove=True categories in cleaner_scan.py.
SAFE_CATEGORIES = frozenset({
    "pycache",
    "backup_dirs",
    "debug_output",
    "stale_staging",
    "coverage",
})

CONFIRM_STRING = "DELETE SAFE ARTIFACTS"


def get_tracked_files(project_root: Path) -> set[str]:
    """Return set of git-tracked file paths (relative to project_root).

    Returns empty set if git is unavailable or project_root is not a repo.
    """
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            capture_output=True, text=True, timeout=10,
            cwd=str(project_root),
        )
        if result.returncode != 0:
            return set()
        return set(result.stdout.strip().splitlines())
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
        return set()


def check_git_dirty(project_root: Path) -> bool:
    """Return True if git worktree has uncommitted changes."""
    try:
        result = subprocess.run(
            ["git", "status", "--porcelain"],
            capture_output=True, text=True, timeout=10,
            cwd=str(project_root),
        )
        # Non-zero exit = not a git repo or other error → assume dirty (safe default)
        if result.returncode != 0:
            return True
        return bool(result.stdout.strip())
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
        # If git isn't available or times out, assume dirty (safe default)
        return True


def find_latest_manifest(artifact_dir: Path) -> Path | None:
    """Find the most recent cleanup_manifest_*.json in artifact_dir."""
    manifests = sorted(
        artifact_dir.glob("cleanup_manifest_*.json"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    return manifests[0] if manifests else None


def load_manifest(path: Path) -> dict:
    """Load and validate a cleanup manifest JSON file."""
    with open(path) as f:
        data = json.load(f)

    # Minimal validation
    if "stale_artifacts" not in data:
        raise ValueError(f"Manifest missing 'stale_artifacts' key: {path}")
    if "root" not in data:
        raise ValueError(f"Manifest missing 'root' key: {path}")

    return data


def plan_removals(manifest: dict) -> tuple[list[dict], list[dict]]:
    """Split stale artifacts into safe (will remove) and unsafe (skip).

    Returns (safe_items, skipped_items).
    """
    safe = []
    skipped = []

    for item in manifest.get("stale_artifacts", []):
        if item.get("safe_to_remove", False) and item.get("category") in SAFE_CATEGORIES:
            safe.append(item)
        else:
            skipped.append(item)

    return safe, skipped


def execute_removals(root: Path, items: list[dict]) -> tuple[list[dict], list[dict]]:
    """Actually delete files/dirs. Returns (removed, failed)."""
    removed = []
    failed = []

    for item in items:
        target = root / item["path"]
        try:
            if target.is_dir():
                shutil.rmtree(target)
            elif target.exists():
                target.unlink()
            else:
                # Already gone — still count as removed (idempotent)
                pass
            removed.append(item)
        except OSError as e:
            failed.append({**item, "error": str(e)})

    return removed, failed


def build_report(
    manifest_path: Path,
    safe_items: list[dict],
    skipped_items: list[dict],
    removed: list[dict] | None,
    failed: list[dict] | None,
    dry_run: bool,
) -> MaintainerReport:
    """Build a structured MaintainerReport for the apply operation."""
    findings: list[Finding] = []

    total_safe = len(safe_items)
    total_skipped = len(skipped_items)
    total_size = sum(i.get("size_bytes", 0) for i in safe_items)

    if dry_run:
        findings.append(Finding(
            id="CLA-001",
            severity="info",
            category="dry_run_summary",
            summary=(
                f"Would remove {total_safe} safe artifacts "
                f"({total_size:,} bytes). {total_skipped} items skipped."
            ),
        ))
    else:
        removed_count = len(removed) if removed else 0
        failed_count = len(failed) if failed else 0
        findings.append(Finding(
            id="CLA-002",
            severity="info" if failed_count == 0 else "high",
            category="apply_result",
            summary=(
                f"Removed {removed_count} artifacts ({total_size:,} bytes). "
                f"{failed_count} failures. {total_skipped} items skipped."
            ),
        ))
        if failed:
            for item in failed:
                findings.append(Finding(
                    id="CLA-003",
                    severity="high",
                    category="removal_failure",
                    summary=f"Failed to remove: {item['path']}",
                    details=item.get("error", "unknown"),
                ))

    return MaintainerReport(
        tool_name="cleaner_apply",
        timestamp=now_iso(),
        mode="warn",
        dry_run=dry_run,
        findings=tuple(findings),
        evidence=(
            EvidenceRef(
                kind="file",
                value=str(manifest_path),
                description="Source cleanup manifest",
            ),
        ),
        summary=(
            f"Cleaner apply ({'dry-run' if dry_run else 'execute'}): "
            f"{total_safe} safe, {total_skipped} skipped"
        ),
    )


def validate_root(manifest_root: Path, project_root: Path) -> None:
    """Reject manifest roots that escape the project directory.

    Prevents path traversal via crafted manifests targeting /etc, ~, etc.
    """
    resolved = manifest_root.resolve()
    project_resolved = project_root.resolve()
    if not resolved.is_relative_to(project_resolved):
        raise ValueError(
            f"Manifest root {resolved} is outside project root {project_resolved}. "
            f"Refusing to execute — this could delete files outside the repo."
        )


def write_deletion_log(
    removed: list[dict],
    failed: list[dict],
    manifest_path: Path,
    artifact_dir: Path,
    dumpster_enabled: bool = False,
    dumpster_archive_path: str | None = None,
    dumpster_manifest_path: str | None = None,
    dumpster_result: str = "skipped",
) -> Path:
    """Write a JSON deletion log artifact with exact paths and sizes."""
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
    log_path = artifact_dir / f"cleaner_apply_{ts}.json"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    log_data = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "manifest_source": str(manifest_path),
        "removed": [
            {"path": i["path"], "category": i.get("category", ""), "size_bytes": i.get("size_bytes", 0)}
            for i in removed
        ],
        "failed": [
            {"path": i["path"], "error": i.get("error", "")}
            for i in failed
        ],
        "removed_count": len(removed),
        "failed_count": len(failed),
        "total_bytes_freed": sum(i.get("size_bytes", 0) for i in removed),
        "dumpster_enabled": dumpster_enabled,
        "dumpster_archive_path": dumpster_archive_path,
        "dumpster_manifest_path": dumpster_manifest_path,
        "dumpster_result": dumpster_result,
    }
    log_path.write_text(json.dumps(log_data, indent=2))
    return log_path


DEFAULT_DUMPSTER_DIR = Path.home() / "Downloads" / "asciicker-dumpster"


def _get_git_short_sha(project_root: Path) -> str:
    """Return short git SHA or 'unknown' if unavailable."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, timeout=5,
            cwd=str(project_root),
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
        pass
    return "unknown"


def validate_dumpster_dir(dumpster_dir: Path, project_root: Path) -> None:
    """Reject dumpster paths that resolve inside the repo root."""
    resolved = dumpster_dir.resolve()
    project_resolved = project_root.resolve()
    if resolved.is_relative_to(project_resolved):
        raise ValueError(
            f"Dumpster directory {resolved} is inside project root {project_resolved}. "
            f"Use a path outside the repo (default: ~/Downloads/asciicker-dumpster)."
        )


def create_dumpster_archive(
    root: Path,
    items: list[dict],
    dumpster_dir: Path,
    project_root: Path,
    manifest_path: Path,
    cli_args: list[str] | None = None,
) -> tuple[Path, Path]:
    """Create a compressed tar.gz archive of files about to be deleted.

    Returns (archive_path, manifest_path) on success.
    Raises on failure (fail-closed contract).
    """
    validate_dumpster_dir(dumpster_dir, project_root)
    dumpster_dir.mkdir(parents=True, exist_ok=True)

    ts = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    short_sha = _get_git_short_sha(project_root)
    archive_name = f"cleanup_{ts}_{short_sha}.tar.gz"
    archive_path = dumpster_dir / archive_name
    sidecar_path = dumpster_dir / f"{archive_name}.manifest.json"

    archived_entries: list[dict] = []
    skipped_entries: list[dict] = []

    with tarfile.open(archive_path, "w:gz") as tar:
        for item in items:
            target = root / item["path"]
            if target.exists():
                tar.add(str(target), arcname=item["path"])
                archived_entries.append({
                    "path": item["path"],
                    "category": item.get("category", ""),
                    "size_bytes": item.get("size_bytes", 0),
                })
            else:
                skipped_entries.append({
                    "path": item["path"],
                    "reason": "file_not_found",
                })

    # Safety: if we have items to archive but archived nothing, and not all
    # candidates are confirmed missing, treat as failure.
    if items and not archived_entries and not all(
        not (root / i["path"]).exists() for i in items
    ):
        archive_path.unlink(missing_ok=True)
        raise RuntimeError(
            f"Dumpster archive wrote 0 files from {len(items)} candidates "
            f"but not all candidates are confirmed missing. Aborting."
        )

    sidecar_data = {
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "git_commit": short_sha,
        "project_root": str(project_root.resolve()),
        "cwd": os.getcwd(),
        "command_args": cli_args or sys.argv,
        "manifest_source": str(manifest_path),
        "deletion_candidates": [
            {
                "path": i["path"],
                "category": i.get("category", ""),
                "tracked": bool(i.get("skip_reason") != "git-tracked"),
                "exists": (root / i["path"]).exists(),
            }
            for i in items
        ],
        "archived_entries": archived_entries,
        "skipped_entries": skipped_entries,
        "archived_count": len(archived_entries),
        "skipped_count": len(skipped_entries),
    }
    sidecar_path.write_text(json.dumps(sidecar_data, indent=2))

    return archive_path, sidecar_path


def apply_to_result(
    manifest_path: Path,
    manifest: dict,
    dry_run: bool,
    project_root: Path | None = None,
    allow_tracked: bool = False,
    dumpster_dir: Path | None = None,
    no_dumpster: bool = False,
) -> dict:
    """Run the full apply workflow and return structured result."""
    root = Path(manifest["root"])

    # In execute mode, validate root is inside project to prevent path traversal
    effective_project_root = project_root or _PROJECT_ROOT
    if not dry_run:
        validate_root(root, effective_project_root)

    safe_items, skipped_items = plan_removals(manifest)

    # Filter out git-tracked files unless --allow-tracked
    if not allow_tracked:
        tracked = get_tracked_files(root)
        if tracked:
            untracked_safe = []
            for item in safe_items:
                if item["path"] in tracked:
                    skipped_items = [*skipped_items, {**item, "skip_reason": "git-tracked"}]
                else:
                    untracked_safe.append(item)
            safe_items = untracked_safe

    removed = None
    failed = None
    deletion_log_path = None
    dumpster_result = "skipped"
    dumpster_archive_path = None
    dumpster_manifest_path = None

    if not dry_run:
        # Dumpster archive: create backup BEFORE deletion (fail-closed)
        effective_dumpster = dumpster_dir or DEFAULT_DUMPSTER_DIR
        if not no_dumpster and safe_items:
            archive_path, sidecar_path = create_dumpster_archive(
                root, safe_items, effective_dumpster,
                effective_project_root, manifest_path,
            )
            dumpster_result = "created"
            dumpster_archive_path = str(archive_path)
            dumpster_manifest_path = str(sidecar_path)

        removed, failed = execute_removals(root, safe_items)
        # Write deletion log artifact
        artifact_dir = effective_project_root / "artifacts" / "maintainer"
        deletion_log_path = write_deletion_log(
            removed or [], failed or [], manifest_path, artifact_dir,
            dumpster_enabled=not no_dumpster,
            dumpster_archive_path=dumpster_archive_path,
            dumpster_manifest_path=dumpster_manifest_path,
            dumpster_result=dumpster_result,
        )

    report = build_report(
        manifest_path, safe_items, skipped_items,
        removed, failed, dry_run,
    )

    return {
        "report": report,
        "safe_items": safe_items,
        "skipped_items": skipped_items,
        "removed": removed or [],
        "failed": failed or [],
        "deletion_log": str(deletion_log_path) if deletion_log_path else None,
        "dumpster_enabled": not no_dumpster,
        "dumpster_archive_path": dumpster_archive_path,
        "dumpster_manifest_path": dumpster_manifest_path,
        "dumpster_result": dumpster_result if not dry_run else "skipped",
    }


def main():
    parser = argparse.ArgumentParser(
        description="Cleaner apply — remove safe stale artifacts",
    )
    parser.add_argument(
        "--manifest", type=Path, default=None,
        help="Path to cleanup_manifest_*.json (default: latest in artifacts/maintainer/)",
    )
    parser.add_argument(
        "--execute", action="store_true",
        help="Actually delete files (default: dry-run)",
    )
    parser.add_argument(
        "--confirm", type=str, default=None,
        help=f'Required with --execute: pass --confirm "{CONFIRM_STRING}"',
    )
    parser.add_argument(
        "--allow-dirty", action="store_true",
        help="Allow execution even if git worktree has uncommitted changes",
    )
    parser.add_argument(
        "--allow-tracked", action="store_true",
        help="Allow removal of git-tracked files (default: skip tracked files)",
    )
    parser.add_argument(
        "--dumpster-dir", type=Path, default=None,
        help=f"Directory for backup archives (default: {DEFAULT_DUMPSTER_DIR})",
    )
    parser.add_argument(
        "--no-dumpster", action="store_true",
        help="Skip creating backup archive before deletion",
    )
    parser.add_argument(
        "--json", action="store_true",
        help="Output result as JSON to stdout",
    )
    args = parser.parse_args()

    # Confirmation interlock for destructive mode
    if args.execute and args.confirm != CONFIRM_STRING:
        print(
            f'Error: --execute requires --confirm "{CONFIRM_STRING}"\n'
            f"This prevents accidental deletion. Run with --execute --confirm "
            f'"{CONFIRM_STRING}" to proceed.',
            file=sys.stderr,
        )
        sys.exit(1)

    # Git dirty check for destructive mode
    if args.execute and not args.allow_dirty:
        if check_git_dirty(_PROJECT_ROOT):
            print(
                "Error: git worktree has uncommitted changes. "
                "Commit or stash changes first, or pass --allow-dirty to override.",
                file=sys.stderr,
            )
            sys.exit(1)

    # Find manifest
    if args.manifest:
        manifest_path = args.manifest
    else:
        artifact_dir = _PROJECT_ROOT / "artifacts" / "maintainer"
        manifest_path = find_latest_manifest(artifact_dir)

    if not manifest_path or not manifest_path.exists():
        print("Error: No cleanup manifest found. Run cleaner_scan.py first.", file=sys.stderr)
        sys.exit(1)

    manifest = load_manifest(manifest_path)
    dry_run = not args.execute
    result = apply_to_result(
        manifest_path, manifest, dry_run,
        allow_tracked=args.allow_tracked,
        dumpster_dir=args.dumpster_dir,
        no_dumpster=args.no_dumpster,
    )
    report = result["report"]

    if args.json:
        output = {
            "dry_run": dry_run,
            "manifest_path": str(manifest_path),
            "safe_count": len(result["safe_items"]),
            "skipped_count": len(result["skipped_items"]),
            "removed_count": len(result["removed"]),
            "failed_count": len(result["failed"]),
            "removed": [i["path"] for i in result["removed"]],
            "failed": [{"path": i["path"], "error": i.get("error", "")} for i in result["failed"]],
            "summary": report.summary,
            "deletion_log": result.get("deletion_log"),
            "dumpster_enabled": result.get("dumpster_enabled"),
            "dumpster_archive_path": result.get("dumpster_archive_path"),
            "dumpster_result": result.get("dumpster_result"),
        }
        print(json.dumps(output, indent=2))
    else:
        md = report_to_markdown(report)
        print(md)

        if not dry_run:
            # Write artifact report
            artifact_dir = _PROJECT_ROOT / "artifacts" / "maintainer"
            artifact_dir.mkdir(parents=True, exist_ok=True)
            ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
            report_path = artifact_dir / f"cleaner_apply_{ts}.md"
            report_path.write_text(md)
            print(f"\nReport written to {report_path}")
            if result.get("deletion_log"):
                print(f"Deletion log written to {result['deletion_log']}")
            if result.get("dumpster_archive_path"):
                print(f"Dumpster archive: {result['dumpster_archive_path']}")

    sys.exit(0)


if __name__ == "__main__":
    main()
