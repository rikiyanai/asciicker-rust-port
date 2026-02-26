"""Tests for cleaner_apply — dry-run, execute, safe-only, error handling, dumpster backup."""
import json
import sys
import tarfile
from pathlib import Path
from unittest.mock import patch

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.cleaner_apply import (
    CONFIRM_STRING,
    apply_to_result,
    check_git_dirty,
    create_dumpster_archive,
    execute_removals,
    find_latest_manifest,
    get_tracked_files,
    load_manifest,
    plan_removals,
    validate_dumpster_dir,
    validate_root,
    write_deletion_log,
)


@pytest.fixture
def mock_repo(tmp_path):
    """Create a repo with stale artifacts matching scan categories."""
    # Create safe artifacts
    cache = tmp_path / "scripts" / "__pycache__"
    cache.mkdir(parents=True)
    (cache / "foo.pyc").write_bytes(b"\x00" * 100)
    (cache / "bar.pyc").write_bytes(b"\x00" * 50)

    (tmp_path / "old.bak").write_text("backup stuff")
    (tmp_path / "debug.debug.log").write_text("debug info")

    staging = tmp_path / "scripts" / "staging"
    staging.mkdir(parents=True)
    (staging / "output.png").write_bytes(b"\x89PNG" + b"\x00" * 200)

    # Create unsafe artifact
    (tmp_path / "test_scratch_foo.py").write_text("# scratch test")

    return tmp_path


@pytest.fixture
def manifest(mock_repo):
    """Build a manifest matching the mock_repo artifacts."""
    return {
        "timestamp": "2026-02-20T00:00:00+00:00",
        "root": str(mock_repo),
        "stale_artifacts": [
            {
                "category": "pycache",
                "path": "scripts/__pycache__/foo.pyc",
                "size_bytes": 100,
                "age_days": 1.0,
                "safe_to_remove": True,
                "description": "Python bytecode cache",
            },
            {
                "category": "pycache",
                "path": "scripts/__pycache__/bar.pyc",
                "size_bytes": 50,
                "age_days": 1.0,
                "safe_to_remove": True,
                "description": "Python bytecode cache",
            },
            {
                "category": "backup_dirs",
                "path": "old.bak",
                "size_bytes": 12,
                "age_days": 5.0,
                "safe_to_remove": True,
                "description": "Backup files",
            },
            {
                "category": "debug_output",
                "path": "debug.debug.log",
                "size_bytes": 10,
                "age_days": 2.0,
                "safe_to_remove": True,
                "description": "Debug output",
            },
            {
                "category": "stale_staging",
                "path": "scripts/staging/output.png",
                "size_bytes": 204,
                "age_days": 0.5,
                "safe_to_remove": True,
                "description": "Staging output",
            },
            {
                "category": "temp_tests",
                "path": "test_scratch_foo.py",
                "size_bytes": 15,
                "age_days": 1.0,
                "safe_to_remove": False,
                "description": "Temporary test file",
            },
        ],
        "duplicate_utils": [],
        "conflicting_docs": [],
        "totals": {"stale_count": 6, "stale_safe_count": 5, "duplicate_groups": 0, "doc_conflicts": 0},
    }


@pytest.fixture
def manifest_file(tmp_path, manifest):
    """Write manifest to a JSON file."""
    path = tmp_path / "artifacts" / "maintainer" / "cleanup_manifest_20260220T000000.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(manifest))
    return path


class TestPlanRemovals:
    def test_splits_safe_and_unsafe(self, manifest):
        safe, skipped = plan_removals(manifest)
        assert len(safe) == 5
        assert len(skipped) == 1
        assert skipped[0]["category"] == "temp_tests"

    def test_all_safe_items_have_safe_flag(self, manifest):
        safe, _ = plan_removals(manifest)
        for item in safe:
            assert item["safe_to_remove"] is True

    def test_empty_manifest(self):
        safe, skipped = plan_removals({"stale_artifacts": []})
        assert safe == []
        assert skipped == []


class TestExecuteRemovals:
    def test_deletes_files(self, mock_repo, manifest):
        safe, _ = plan_removals(manifest)
        removed, failed = execute_removals(mock_repo, safe)
        assert len(removed) == 5
        assert len(failed) == 0
        assert not (mock_repo / "scripts" / "__pycache__" / "foo.pyc").exists()
        assert not (mock_repo / "old.bak").exists()
        assert not (mock_repo / "debug.debug.log").exists()

    def test_unsafe_not_touched(self, mock_repo, manifest):
        safe, _ = plan_removals(manifest)
        execute_removals(mock_repo, safe)
        # The unsafe file should still exist
        assert (mock_repo / "test_scratch_foo.py").exists()

    def test_already_deleted_is_idempotent(self, mock_repo, manifest):
        safe, _ = plan_removals(manifest)
        # Delete one file manually first
        (mock_repo / "old.bak").unlink()
        removed, failed = execute_removals(mock_repo, safe)
        # Should still succeed (idempotent)
        assert len(removed) == 5
        assert len(failed) == 0

    def test_permission_error_reported(self, mock_repo, manifest):
        safe, _ = plan_removals(manifest)
        # Simulate OSError on unlink for one file
        original_unlink = Path.unlink

        def mock_unlink(self, *args, **kwargs):
            if self.name == "foo.pyc":
                raise OSError("Permission denied (mock)")
            return original_unlink(self, *args, **kwargs)

        with patch.object(Path, "unlink", mock_unlink):
            removed, failed = execute_removals(mock_repo, safe)
        assert len(failed) >= 1
        assert "Permission denied" in failed[0]["error"]


class TestApplyToResult:
    def test_dry_run_no_deletions(self, manifest_file, manifest, mock_repo):
        result = apply_to_result(manifest_file, manifest, dry_run=True)
        assert result["report"].dry_run is True
        assert len(result["removed"]) == 0
        # All files still exist
        assert (mock_repo / "scripts" / "__pycache__" / "foo.pyc").exists()
        assert (mock_repo / "old.bak").exists()

    def test_execute_actually_deletes(self, manifest_file, manifest, mock_repo):
        result = apply_to_result(manifest_file, manifest, dry_run=False, project_root=mock_repo)
        assert result["report"].dry_run is False
        assert len(result["removed"]) == 5
        assert not (mock_repo / "scripts" / "__pycache__" / "foo.pyc").exists()
        assert not (mock_repo / "old.bak").exists()

    def test_report_has_correct_tool_name(self, manifest_file, manifest):
        result = apply_to_result(manifest_file, manifest, dry_run=True)
        assert result["report"].tool_name == "cleaner_apply"

    def test_report_includes_evidence(self, manifest_file, manifest):
        result = apply_to_result(manifest_file, manifest, dry_run=True)
        assert len(result["report"].evidence) == 1
        assert result["report"].evidence[0].kind == "file"

    def test_dry_run_finding_is_cla_001(self, manifest_file, manifest):
        result = apply_to_result(manifest_file, manifest, dry_run=True)
        findings = result["report"].findings
        assert any(f.id == "CLA-001" for f in findings)

    def test_execute_finding_is_cla_002(self, manifest_file, manifest, mock_repo):
        result = apply_to_result(manifest_file, manifest, dry_run=False, project_root=mock_repo)
        findings = result["report"].findings
        assert any(f.id == "CLA-002" for f in findings)


class TestFindLatestManifest:
    def test_finds_most_recent(self, tmp_path):
        d = tmp_path / "artifacts" / "maintainer"
        d.mkdir(parents=True)
        (d / "cleanup_manifest_20260219T000000.json").write_text("{}")
        (d / "cleanup_manifest_20260220T000000.json").write_text("{}")
        result = find_latest_manifest(d)
        assert result is not None
        assert "20260220" in result.name

    def test_returns_none_when_empty(self, tmp_path):
        d = tmp_path / "artifacts" / "maintainer"
        d.mkdir(parents=True)
        assert find_latest_manifest(d) is None

    def test_returns_none_when_dir_missing(self, tmp_path):
        d = tmp_path / "nonexistent"
        # glob on non-existent dir
        assert find_latest_manifest(d) is None


class TestLoadManifest:
    def test_loads_valid_manifest(self, manifest_file):
        data = load_manifest(manifest_file)
        assert "stale_artifacts" in data
        assert "root" in data

    def test_rejects_missing_stale_artifacts(self, tmp_path):
        bad = tmp_path / "bad.json"
        bad.write_text(json.dumps({"root": "/tmp"}))
        with pytest.raises(ValueError, match="stale_artifacts"):
            load_manifest(bad)

    def test_rejects_missing_root(self, tmp_path):
        bad = tmp_path / "bad.json"
        bad.write_text(json.dumps({"stale_artifacts": []}))
        with pytest.raises(ValueError, match="root"):
            load_manifest(bad)


class TestMainCLI:
    def test_dry_run_default(self, manifest_file, capsys):
        from scripts.maintainer.cleaner_apply import main

        with patch("scripts.maintainer.cleaner_apply._PROJECT_ROOT", manifest_file.parent.parent.parent), \
             patch("sys.argv", ["cleaner_apply.py", "--manifest", str(manifest_file)]), \
             pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 0
        out = capsys.readouterr().out
        assert "dry-run" in out.lower() or "Dry Run" in out

    def test_json_output(self, manifest_file, capsys):
        from scripts.maintainer.cleaner_apply import main

        with patch("scripts.maintainer.cleaner_apply._PROJECT_ROOT", manifest_file.parent.parent.parent), \
             patch("sys.argv", ["cleaner_apply.py", "--manifest", str(manifest_file), "--json"]), \
             pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["dry_run"] is True
        assert "safe_count" in data

    def test_missing_manifest_exits_1(self, tmp_path, capsys):
        from scripts.maintainer.cleaner_apply import main

        with patch("scripts.maintainer.cleaner_apply._PROJECT_ROOT", tmp_path), \
             patch("sys.argv", ["cleaner_apply.py", "--manifest", str(tmp_path / "nope.json")]), \
             pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 1


class TestRootEscape:
    """Regression: manifest root outside project must be rejected in execute mode."""

    def test_execute_rejects_root_outside_project(self, tmp_path):
        """A manifest pointing root at / should raise ValueError on execute."""
        evil_manifest = {
            "root": "/",
            "stale_artifacts": [
                {
                    "category": "pycache",
                    "path": "tmp/something.pyc",
                    "size_bytes": 10,
                    "safe_to_remove": True,
                },
            ],
        }
        manifest_path = tmp_path / "evil.json"
        manifest_path.write_text(json.dumps(evil_manifest))

        with pytest.raises(ValueError, match="outside project root"):
            apply_to_result(
                manifest_path, evil_manifest, dry_run=False,
                project_root=tmp_path,
            )

    def test_dry_run_allows_any_root(self, tmp_path):
        """Dry-run doesn't delete, so root validation is skipped."""
        manifest = {
            "root": "/",
            "stale_artifacts": [],
        }
        manifest_path = tmp_path / "m.json"
        manifest_path.write_text(json.dumps(manifest))

        # Should not raise
        result = apply_to_result(
            manifest_path, manifest, dry_run=True,
            project_root=tmp_path,
        )
        assert result["report"].dry_run is True

    def test_validate_root_accepts_subdirectory(self, tmp_path):
        """A root inside the project is accepted."""
        sub = tmp_path / "subdir"
        sub.mkdir()
        validate_root(sub, tmp_path)  # Should not raise

    def test_validate_root_rejects_parent(self, tmp_path):
        """A root that is a parent of the project is rejected."""
        with pytest.raises(ValueError, match="outside project root"):
            validate_root(tmp_path.parent, tmp_path)


class TestConfirmInterlock:
    """--execute requires --confirm with the exact magic string."""

    def test_execute_without_confirm_exits_1(self, manifest_file, capsys):
        from scripts.maintainer.cleaner_apply import main

        with patch("scripts.maintainer.cleaner_apply._PROJECT_ROOT", manifest_file.parent.parent.parent), \
             patch("sys.argv", ["cleaner_apply.py", "--manifest", str(manifest_file), "--execute"]), \
             pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 1
        err = capsys.readouterr().err
        assert "DELETE SAFE ARTIFACTS" in err

    def test_execute_with_wrong_confirm_exits_1(self, manifest_file, capsys):
        from scripts.maintainer.cleaner_apply import main

        with patch("scripts.maintainer.cleaner_apply._PROJECT_ROOT", manifest_file.parent.parent.parent), \
             patch("sys.argv", [
                 "cleaner_apply.py", "--manifest", str(manifest_file),
                 "--execute", "--confirm", "wrong string"
             ]), \
             pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 1

    def test_execute_with_correct_confirm_proceeds(self, manifest_file, manifest, mock_repo, capsys):
        from scripts.maintainer.cleaner_apply import main

        with patch("scripts.maintainer.cleaner_apply._PROJECT_ROOT", mock_repo), \
             patch("scripts.maintainer.cleaner_apply.check_git_dirty", return_value=False), \
             patch("sys.argv", [
                 "cleaner_apply.py", "--manifest", str(manifest_file),
                 "--execute", "--confirm", CONFIRM_STRING,
             ]), \
             pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 0

    def test_confirm_string_constant_matches_docs(self):
        assert CONFIRM_STRING == "DELETE SAFE ARTIFACTS"


class TestGitDirtyGuard:
    """--execute refuses when worktree is dirty unless --allow-dirty."""

    def test_dirty_worktree_blocks_execute(self, manifest_file, capsys):
        from scripts.maintainer.cleaner_apply import main

        with patch("scripts.maintainer.cleaner_apply._PROJECT_ROOT", manifest_file.parent.parent.parent), \
             patch("scripts.maintainer.cleaner_apply.check_git_dirty", return_value=True), \
             patch("sys.argv", [
                 "cleaner_apply.py", "--manifest", str(manifest_file),
                 "--execute", "--confirm", CONFIRM_STRING,
             ]), \
             pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 1
        err = capsys.readouterr().err
        assert "uncommitted" in err.lower()

    def test_allow_dirty_overrides_check(self, manifest_file, manifest, mock_repo, capsys):
        from scripts.maintainer.cleaner_apply import main

        with patch("scripts.maintainer.cleaner_apply._PROJECT_ROOT", mock_repo), \
             patch("scripts.maintainer.cleaner_apply.check_git_dirty", return_value=True), \
             patch("sys.argv", [
                 "cleaner_apply.py", "--manifest", str(manifest_file),
                 "--execute", "--confirm", CONFIRM_STRING, "--allow-dirty",
             ]), \
             pytest.raises(SystemExit) as exc_info:
            main()

        assert exc_info.value.code == 0

    def test_check_git_dirty_with_clean_repo(self, tmp_path):
        """check_git_dirty returns False for a clean git repo."""
        import subprocess
        subprocess.run(["git", "init"], cwd=str(tmp_path), capture_output=True)
        subprocess.run(["git", "commit", "--allow-empty", "-m", "init"],
                       cwd=str(tmp_path), capture_output=True)
        assert check_git_dirty(tmp_path) is False

    def test_check_git_dirty_with_dirty_repo(self, tmp_path):
        """check_git_dirty returns True for a dirty git repo."""
        import subprocess
        subprocess.run(["git", "init"], cwd=str(tmp_path), capture_output=True)
        subprocess.run(["git", "commit", "--allow-empty", "-m", "init"],
                       cwd=str(tmp_path), capture_output=True)
        (tmp_path / "dirty.txt").write_text("dirty")
        assert check_git_dirty(tmp_path) is True

    def test_check_git_dirty_non_git_dir_returns_true(self, tmp_path):
        """Non-git directory returns True (safe default)."""
        assert check_git_dirty(tmp_path) is True


class TestDeletionLog:
    """Execute mode writes a JSON deletion log artifact."""

    def test_execute_creates_deletion_log(self, manifest_file, manifest, mock_repo):
        result = apply_to_result(manifest_file, manifest, dry_run=False, project_root=mock_repo)
        assert result["deletion_log"] is not None
        log_path = Path(result["deletion_log"])
        assert log_path.exists()

        log_data = json.loads(log_path.read_text())
        assert log_data["removed_count"] == 5
        assert log_data["failed_count"] == 0
        assert log_data["total_bytes_freed"] > 0
        assert len(log_data["removed"]) == 5

    def test_dry_run_has_no_deletion_log(self, manifest_file, manifest):
        result = apply_to_result(manifest_file, manifest, dry_run=True)
        assert result["deletion_log"] is None

    def test_deletion_log_has_correct_structure(self, tmp_path):
        removed = [{"path": "foo.pyc", "category": "pycache", "size_bytes": 42}]
        failed = [{"path": "bar.pyc", "error": "nope"}]
        manifest_path = tmp_path / "test.json"
        artifact_dir = tmp_path / "artifacts"

        log_path = write_deletion_log(removed, failed, manifest_path, artifact_dir)
        data = json.loads(log_path.read_text())

        assert data["removed_count"] == 1
        assert data["failed_count"] == 1
        assert data["total_bytes_freed"] == 42
        assert data["removed"][0]["path"] == "foo.pyc"
        assert data["failed"][0]["error"] == "nope"
        assert "timestamp" in data
        assert str(manifest_path) in data["manifest_source"]


class TestTrackedFileProtection:
    """Git-tracked files must be skipped unless --allow-tracked."""

    def test_tracked_files_skipped_by_default(self, manifest_file, manifest, mock_repo):
        """Tracked files in safe list are moved to skipped."""
        # Simulate git tracking the .bak file
        with patch(
            "scripts.maintainer.cleaner_apply.get_tracked_files",
            return_value={"old.bak"},
        ):
            result = apply_to_result(
                manifest_file, manifest, dry_run=False, project_root=mock_repo,
            )

        # old.bak should NOT be in removed (it's tracked)
        removed_paths = [i["path"] for i in result["removed"]]
        assert "old.bak" not in removed_paths
        # old.bak should still exist on disk
        assert (mock_repo / "old.bak").exists()
        # Should be in skipped
        skipped_paths = [i["path"] for i in result["skipped_items"]]
        assert "old.bak" in skipped_paths
        # Other safe items were still removed
        assert len(result["removed"]) == 4

    def test_allow_tracked_includes_tracked_files(self, manifest_file, manifest, mock_repo):
        """--allow-tracked removes tracked files too."""
        with patch(
            "scripts.maintainer.cleaner_apply.get_tracked_files",
            return_value={"old.bak"},
        ):
            result = apply_to_result(
                manifest_file, manifest, dry_run=False,
                project_root=mock_repo, allow_tracked=True,
            )

        removed_paths = [i["path"] for i in result["removed"]]
        assert "old.bak" in removed_paths
        assert len(result["removed"]) == 5

    def test_tracked_files_skipped_in_dry_run(self, manifest_file, manifest):
        """Dry-run also shows tracked files as skipped."""
        with patch(
            "scripts.maintainer.cleaner_apply.get_tracked_files",
            return_value={"old.bak", "debug.debug.log"},
        ):
            result = apply_to_result(
                manifest_file, manifest, dry_run=True,
            )

        # In dry-run, safe_items should exclude tracked files
        safe_paths = [i["path"] for i in result["safe_items"]]
        assert "old.bak" not in safe_paths
        assert "debug.debug.log" not in safe_paths
        assert len(result["safe_items"]) == 3
        # Tracked files appear in skipped
        skipped_paths = [i["path"] for i in result["skipped_items"]]
        assert "old.bak" in skipped_paths
        assert "debug.debug.log" in skipped_paths

    def test_get_tracked_files_returns_set(self, tmp_path):
        """get_tracked_files returns a set of tracked paths."""
        import subprocess
        subprocess.run(["git", "init"], cwd=str(tmp_path), capture_output=True)
        (tmp_path / "tracked.txt").write_text("hello")
        subprocess.run(["git", "add", "tracked.txt"], cwd=str(tmp_path), capture_output=True)
        subprocess.run(["git", "commit", "-m", "init"],
                       cwd=str(tmp_path), capture_output=True)
        result = get_tracked_files(tmp_path)
        assert "tracked.txt" in result

    def test_get_tracked_files_non_git_returns_empty(self, tmp_path):
        """Non-git directory returns empty set."""
        result = get_tracked_files(tmp_path)
        assert result == set()

    def test_skip_reason_tagged_on_tracked(self, manifest_file, manifest, mock_repo):
        """Skipped tracked files have skip_reason='git-tracked'."""
        with patch(
            "scripts.maintainer.cleaner_apply.get_tracked_files",
            return_value={"old.bak"},
        ):
            result = apply_to_result(
                manifest_file, manifest, dry_run=True,
            )

        tracked_skipped = [
            i for i in result["skipped_items"] if i.get("skip_reason") == "git-tracked"
        ]
        assert len(tracked_skipped) == 1
        assert tracked_skipped[0]["path"] == "old.bak"


class TestDumpsterBackup:
    """Compressed dumpster archive created before deletion in execute mode."""

    @pytest.fixture
    def dumpster_dir(self, tmp_path_factory):
        """Create a dumpster directory truly outside the mock repo tmp_path."""
        return tmp_path_factory.mktemp("dumpster")

    def test_execute_creates_archive_and_manifest(
        self, manifest_file, manifest, mock_repo, dumpster_dir,
    ):
        """Execute mode creates .tar.gz + .manifest.json in dumpster dir."""
        result = apply_to_result(
            manifest_file, manifest, dry_run=False,
            project_root=mock_repo, dumpster_dir=dumpster_dir,
        )

        assert result["dumpster_result"] == "created"
        archive_path = Path(result["dumpster_archive_path"])
        manifest_path = Path(result["dumpster_manifest_path"])

        assert archive_path.exists()
        assert archive_path.suffix == ".gz"
        assert archive_path.stem.endswith(".tar")
        assert manifest_path.exists()
        assert manifest_path.name.endswith(".manifest.json")

        # Sidecar manifest has expected structure
        sidecar = json.loads(manifest_path.read_text())
        assert sidecar["archived_count"] == 5
        assert "git_commit" in sidecar
        assert "project_root" in sidecar
        assert len(sidecar["archived_entries"]) == 5

    def test_dry_run_skips_archive(self, manifest_file, manifest, dumpster_dir):
        """Dry-run does not create any archive."""
        result = apply_to_result(
            manifest_file, manifest, dry_run=True,
            dumpster_dir=dumpster_dir,
        )

        assert result["dumpster_result"] == "skipped"
        assert result["dumpster_archive_path"] is None
        # Dumpster dir should be empty (no archives)
        assert list(dumpster_dir.iterdir()) == []

    def test_no_dumpster_skips_archive(
        self, manifest_file, manifest, mock_repo, dumpster_dir,
    ):
        """--no-dumpster skips archive creation even in execute mode."""
        result = apply_to_result(
            manifest_file, manifest, dry_run=False,
            project_root=mock_repo, dumpster_dir=dumpster_dir,
            no_dumpster=True,
        )

        assert result["dumpster_result"] == "skipped"
        assert result["dumpster_archive_path"] is None
        # Files were still deleted
        assert len(result["removed"]) == 5

    def test_dumpster_inside_repo_rejected(self, mock_repo):
        """Dumpster path inside the project root is rejected."""
        inside = mock_repo / "dumpster"
        inside.mkdir()
        with pytest.raises(ValueError, match="inside project root"):
            validate_dumpster_dir(inside, mock_repo)

    def test_backup_failure_aborts_deletion(
        self, manifest_file, manifest, mock_repo, dumpster_dir,
    ):
        """If archive creation fails, no files are deleted (fail-closed)."""
        with patch(
            "scripts.maintainer.cleaner_apply.create_dumpster_archive",
            side_effect=RuntimeError("Simulated archive failure"),
        ):
            with pytest.raises(RuntimeError, match="Simulated archive failure"):
                apply_to_result(
                    manifest_file, manifest, dry_run=False,
                    project_root=mock_repo, dumpster_dir=dumpster_dir,
                )

        # All files must still exist (deletion never happened)
        assert (mock_repo / "old.bak").exists()
        assert (mock_repo / "scripts" / "__pycache__" / "foo.pyc").exists()
        assert (mock_repo / "debug.debug.log").exists()

    def test_archive_contains_expected_files(
        self, manifest_file, manifest, mock_repo, dumpster_dir,
    ):
        """Archive contains exactly the files from the deletion set."""
        result = apply_to_result(
            manifest_file, manifest, dry_run=False,
            project_root=mock_repo, dumpster_dir=dumpster_dir,
        )

        archive_path = Path(result["dumpster_archive_path"])
        with tarfile.open(archive_path, "r:gz") as tar:
            archived_names = set(tar.getnames())

        safe_paths = {
            "scripts/__pycache__/foo.pyc",
            "scripts/__pycache__/bar.pyc",
            "old.bak",
            "debug.debug.log",
            "scripts/staging/output.png",
        }
        assert archived_names == safe_paths

    def test_missing_file_candidates_handled(self, dumpster_dir, tmp_path_factory):
        """Candidates pointing to non-existent files are logged as skipped."""
        repo_dir = tmp_path_factory.mktemp("repo_missing")
        # Create only one of two candidates
        (repo_dir / "exists.pyc").write_bytes(b"\x00" * 10)

        items = [
            {"path": "exists.pyc", "category": "pycache", "size_bytes": 10},
            {"path": "gone.pyc", "category": "pycache", "size_bytes": 5},
        ]
        manifest_path = repo_dir / "m.json"

        archive_path, sidecar_path = create_dumpster_archive(
            repo_dir, items, dumpster_dir, repo_dir, manifest_path,
        )

        sidecar = json.loads(sidecar_path.read_text())
        assert sidecar["archived_count"] == 1
        assert sidecar["skipped_count"] == 1
        assert sidecar["skipped_entries"][0]["reason"] == "file_not_found"

        # Archive should contain only the existing file
        with tarfile.open(archive_path, "r:gz") as tar:
            assert tar.getnames() == ["exists.pyc"]

    def test_tracked_guard_still_applies_before_backup(
        self, manifest_file, manifest, mock_repo, dumpster_dir,
    ):
        """Tracked files are filtered BEFORE archive creation."""
        with patch(
            "scripts.maintainer.cleaner_apply.get_tracked_files",
            return_value={"old.bak"},
        ):
            result = apply_to_result(
                manifest_file, manifest, dry_run=False,
                project_root=mock_repo, dumpster_dir=dumpster_dir,
            )

        # old.bak should NOT be in the archive (filtered before backup)
        archive_path = Path(result["dumpster_archive_path"])
        with tarfile.open(archive_path, "r:gz") as tar:
            archived_names = set(tar.getnames())
        assert "old.bak" not in archived_names
        # old.bak should still exist on disk
        assert (mock_repo / "old.bak").exists()

    def test_deletion_log_includes_dumpster_metadata(
        self, manifest_file, manifest, mock_repo, dumpster_dir,
    ):
        """Deletion log JSON includes dumpster fields."""
        result = apply_to_result(
            manifest_file, manifest, dry_run=False,
            project_root=mock_repo, dumpster_dir=dumpster_dir,
        )

        log_path = Path(result["deletion_log"])
        log_data = json.loads(log_path.read_text())
        assert log_data["dumpster_enabled"] is True
        assert log_data["dumpster_result"] == "created"
        assert log_data["dumpster_archive_path"] is not None
        assert log_data["dumpster_manifest_path"] is not None
