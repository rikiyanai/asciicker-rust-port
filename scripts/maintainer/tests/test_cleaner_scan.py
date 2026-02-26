"""Tests for cleaner_scan — stale detection, duplicates, conflicts, dry-run."""
import json
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.cleaner_scan import (
    run_scan,
    scan_conflicting_docs,
    scan_duplicate_utils,
    scan_stale_artifacts,
    scan_to_report,
)


@pytest.fixture
def mock_repo(tmp_path):
    """Create a minimal repo structure for scanning."""
    # Create standard dirs
    (tmp_path / "scripts").mkdir()
    (tmp_path / "docs" / "plans").mkdir(parents=True)
    (tmp_path / ".planning").mkdir()
    (tmp_path / "artifacts" / "maintainer").mkdir(parents=True)
    return tmp_path


class TestScanStaleArtifacts:
    def test_finds_pycache(self, mock_repo):
        cache = mock_repo / "scripts" / "__pycache__"
        cache.mkdir(parents=True)
        (cache / "foo.pyc").write_bytes(b"\x00" * 10)

        results = scan_stale_artifacts(mock_repo)
        pycache = [r for r in results if r["category"] == "pycache"]
        assert len(pycache) >= 1

    def test_finds_backup_files(self, mock_repo):
        (mock_repo / "test.bak").write_text("old stuff")

        results = scan_stale_artifacts(mock_repo)
        backups = [r for r in results if r["category"] == "backup_dirs"]
        assert len(backups) >= 1
        assert backups[0]["safe_to_remove"] is True

    def test_empty_repo_returns_empty(self, mock_repo):
        results = scan_stale_artifacts(mock_repo)
        assert isinstance(results, list)
        # No artifacts to find in clean repo
        assert len(results) == 0

    def test_staging_dirs_detected(self, mock_repo):
        staging = mock_repo / "scripts" / "staging"
        staging.mkdir(parents=True)
        (staging / "output.png").write_bytes(b"\x89PNG")

        results = scan_stale_artifacts(mock_repo)
        staging_hits = [r for r in results if r["category"] == "stale_staging"]
        assert len(staging_hits) >= 1
        assert staging_hits[0]["safe_to_remove"] is True

    def test_debug_output_detected(self, mock_repo):
        (mock_repo / "test.debug.log").write_text("debug info")

        results = scan_stale_artifacts(mock_repo)
        debug = [r for r in results if r["category"] == "debug_output"]
        assert len(debug) >= 1


class TestScanDuplicateUtils:
    def test_identical_files_detected(self, mock_repo):
        scripts = mock_repo / "scripts" / "asset_gen"
        scripts.mkdir(parents=True)
        content = "def helper():\n    return 42\n"
        (scripts / "util_a.py").write_text(content)
        (scripts / "util_b.py").write_text(content)

        results = scan_duplicate_utils(mock_repo)
        assert len(results) >= 1
        assert len(results[0]["files"]) == 2

    def test_different_files_not_flagged(self, mock_repo):
        scripts = mock_repo / "scripts" / "asset_gen"
        scripts.mkdir(parents=True)
        (scripts / "util_a.py").write_text("def a(): return 1\n")
        (scripts / "util_b.py").write_text("def b(): return 2\n")

        results = scan_duplicate_utils(mock_repo)
        assert len(results) == 0

    def test_empty_dir_returns_empty(self, mock_repo):
        results = scan_duplicate_utils(mock_repo)
        assert results == []


class TestScanConflictingDocs:
    def test_conflicting_statuses_detected(self, mock_repo):
        plans = mock_repo / "docs" / "plans"
        # Same stem but different status claims
        (plans / "my-feature.md").write_text(
            "# My Feature\nstatus: complete\nDone.\n"
        )
        # Different status in .planning
        planning = mock_repo / ".planning"
        (planning / "my-feature.md").write_text(
            "# My Feature\nstatus: active\nIn progress.\n"
        )

        results = scan_conflicting_docs(mock_repo)
        assert len(results) >= 1
        assert results[0]["topic"] == "my-feature"

    def test_same_status_no_conflict(self, mock_repo):
        plans = mock_repo / "docs" / "plans"
        (plans / "my-feature.md").write_text(
            "# My Feature\nstatus: active\nWorking.\n"
        )
        planning = mock_repo / ".planning"
        (planning / "my-feature.md").write_text(
            "# My Feature\nstatus: active\nAlso working.\n"
        )

        results = scan_conflicting_docs(mock_repo)
        assert len(results) == 0

    def test_no_status_lines_no_conflict(self, mock_repo):
        plans = mock_repo / "docs" / "plans"
        (plans / "my-feature.md").write_text("# My Feature\nNo status here.\n")

        results = scan_conflicting_docs(mock_repo)
        assert len(results) == 0


class TestRunScan:
    def test_returns_structured_result(self, mock_repo):
        result = run_scan(mock_repo)
        assert "timestamp" in result
        assert "stale_artifacts" in result
        assert "duplicate_utils" in result
        assert "conflicting_docs" in result
        assert "totals" in result
        assert "stale_count" in result["totals"]
        assert "duplicate_groups" in result["totals"]
        assert "doc_conflicts" in result["totals"]


class TestScanToReport:
    def test_report_has_correct_tool_name(self, mock_repo):
        scan = run_scan(mock_repo)
        report = scan_to_report(scan)
        assert report.tool_name == "cleaner_scan"
        assert report.mode == "warn"

    def test_stale_artifacts_produce_finding(self, mock_repo):
        cache = mock_repo / "scripts" / "__pycache__"
        cache.mkdir(parents=True)
        (cache / "foo.pyc").write_bytes(b"\x00" * 10)

        scan = run_scan(mock_repo)
        report = scan_to_report(scan)
        assert any(f.id == "CLN-001" for f in report.findings)

    def test_conflicts_produce_high_severity(self, mock_repo):
        plans = mock_repo / "docs" / "plans"
        (plans / "thing.md").write_text("status: done\n")
        planning = mock_repo / ".planning"
        (planning / "thing.md").write_text("status: active\n")

        scan = run_scan(mock_repo)
        report = scan_to_report(scan)
        conflict_findings = [f for f in report.findings if f.id == "CLN-003"]
        assert len(conflict_findings) == 1
        assert conflict_findings[0].severity == "high"


class TestDryRunBehavior:
    """Verify --dry-run does NOT write artifacts to disk."""

    def test_dry_run_no_files_written(self, mock_repo, capsys):
        """When --dry-run is used, no artifacts should be created."""
        from scripts.maintainer.cleaner_scan import main

        artifact_dir = mock_repo / "artifacts" / "maintainer"
        before = set(artifact_dir.iterdir()) if artifact_dir.exists() else set()

        with patch(
            "scripts.maintainer.cleaner_scan._PROJECT_ROOT", mock_repo
        ), patch(
            "sys.argv", ["cleaner_scan.py", "--dry-run"]
        ), pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 0
        after = set(artifact_dir.iterdir()) if artifact_dir.exists() else set()
        new_files = after - before
        assert len(new_files) == 0, f"Dry-run created files: {new_files}"

    def test_dry_run_prints_report(self, mock_repo, capsys):
        """Dry-run should print report to stdout."""
        from scripts.maintainer.cleaner_scan import main

        with patch(
            "scripts.maintainer.cleaner_scan._PROJECT_ROOT", mock_repo
        ), patch(
            "sys.argv", ["cleaner_scan.py", "--dry-run"]
        ), pytest.raises(SystemExit):
            main()

        out = capsys.readouterr().out
        assert "Maintainer Report" in out or "cleaner_scan" in out

    def test_non_dry_run_writes_files(self, mock_repo, capsys):
        """Without --dry-run, artifacts should be written."""
        from scripts.maintainer.cleaner_scan import main

        artifact_dir = mock_repo / "artifacts" / "maintainer"
        artifact_dir.mkdir(parents=True, exist_ok=True)
        before = set(artifact_dir.iterdir())

        with patch(
            "scripts.maintainer.cleaner_scan._PROJECT_ROOT", mock_repo
        ), patch(
            "sys.argv", ["cleaner_scan.py"]
        ), pytest.raises(SystemExit):
            main()

        after = set(artifact_dir.iterdir())
        new_files = after - before
        assert len(new_files) == 2  # .md report + .json manifest

    def test_json_mode_ignores_dry_run(self, mock_repo, capsys):
        """--json always prints to stdout regardless of --dry-run."""
        from scripts.maintainer.cleaner_scan import main

        with patch(
            "scripts.maintainer.cleaner_scan._PROJECT_ROOT", mock_repo
        ), patch(
            "sys.argv", ["cleaner_scan.py", "--json"]
        ), pytest.raises(SystemExit):
            main()

        out = capsys.readouterr().out
        parsed = json.loads(out)
        assert "totals" in parsed
