"""Tests for failure_log — append-only, status update, id generation."""
import sys
import tempfile
from pathlib import Path

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from scripts.maintainer.lib.failure_log import (
    FailureEntry, read_failure_log, append_entry, update_status,
    next_failure_id, find_open_entries, find_stale_open_entries,
    find_long_open_entries, entry_to_markdown, VALID_STATUS,
)

FIXTURES_DIR = Path(__file__).parent / "fixtures"


class TestFailureEntry:
    def test_valid_status(self):
        for status in VALID_STATUS:
            kwargs = dict(
                failure_id="FL-001", title="test", status=status,
                date_opened="2026-01-01", category="test",
                description="desc",
            )
            # RESOLVED requires resolution or evidence per invariant
            if status == "RESOLVED":
                kwargs["resolution"] = "Fixed the issue"
            entry = FailureEntry(**kwargs)
            assert entry.status == status

    def test_invalid_status_raises(self):
        with pytest.raises(ValueError, match="Invalid status"):
            FailureEntry(
                failure_id="FL-001", title="test", status="DONE",
                date_opened="2026-01-01", category="test",
                description="desc",
            )

    def test_invalid_id_prefix_raises(self):
        with pytest.raises(ValueError, match="FL-"):
            FailureEntry(
                failure_id="BUG-001", title="test", status="OPEN",
                date_opened="2026-01-01", category="test",
                description="desc",
            )


class TestReadFailureLog:
    def test_read_sample_fixture(self):
        entries = read_failure_log(FIXTURES_DIR / "sample_failure_log.md")
        assert len(entries) == 3

    def test_entry_fields(self):
        entries = read_failure_log(FIXTURES_DIR / "sample_failure_log.md")
        fl001 = entries[0]
        assert fl001.failure_id == "FL-001"
        assert fl001.status == "OPEN"
        assert "spatial" in fl001.title.lower() or "resolution" in fl001.title.lower()
        assert fl001.category == "pipeline"
        assert len(fl001.evidence) >= 1

    def test_related_ids_parsed(self):
        entries = read_failure_log(FIXTURES_DIR / "sample_failure_log.md")
        fl001 = entries[0]
        assert "FL-002" in fl001.related_ids
        assert "FL-003" in fl001.related_ids

    def test_monitoring_status(self):
        entries = read_failure_log(FIXTURES_DIR / "sample_failure_log.md")
        fl003 = entries[2]
        assert fl003.status == "MONITORING"

    def test_nonexistent_file(self):
        entries = read_failure_log(Path("/nonexistent/FAILURE_LOG.md"))
        assert entries == []


class TestAppendEntry:
    def test_append_dry_run(self):
        entry = FailureEntry(
            failure_id="FL-100", title="test entry", status="OPEN",
            date_opened="2026-02-19", category="test",
            description="A test entry",
        )
        md = append_entry(entry, dry_run=True)
        assert "FL-100" in md
        assert "test entry" in md

    def test_append_creates_file(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="first", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="First entry",
            )
            append_entry(entry, path=path)
            assert path.exists()
            content = path.read_text()
            assert "FL-001" in content
            assert "# Failure Log" in content

    def test_append_preserves_existing(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            e1 = FailureEntry(
                failure_id="FL-001", title="first", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="First",
            )
            e2 = FailureEntry(
                failure_id="FL-002", title="second", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="Second",
            )
            append_entry(e1, path=path)
            append_entry(e2, path=path)
            content = path.read_text()
            assert "FL-001" in content
            assert "FL-002" in content


class TestUpdateStatus:
    def test_update_appends_subsection(self):
        """Status update must append a subsection, not edit original line."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            result = update_status("FL-001", "PARTIAL", path=path)
            assert result is True
            content = path.read_text()
            # Original status line is preserved
            assert "**Status:** OPEN" in content
            # New status appears in appended subsection
            assert "OPEN -> PARTIAL" in content

    def test_update_resolved_requires_resolution(self):
        with pytest.raises(ValueError, match="resolution"):
            update_status("FL-001", "RESOLVED", resolution="")

    def test_update_resolved_requires_evidence(self):
        with pytest.raises(ValueError, match="evidence"):
            update_status(
                "FL-001", "RESOLVED",
                resolution="Fixed the bug",
                evidence_refs=(),
            )

    def test_update_resolved_with_evidence(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            result = update_status(
                "FL-001", "RESOLVED",
                resolution="Fixed via commit abc1234",
                evidence_refs=("commit abc1234",),
                path=path,
            )
            assert result is True
            content = path.read_text()
            assert "OPEN -> RESOLVED" in content
            assert "commit abc1234" in content

    def test_update_invalid_status(self):
        with pytest.raises(ValueError, match="Invalid status"):
            update_status("FL-001", "FIXED")

    def test_update_nonexistent_entry(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            result = update_status("FL-999", "PARTIAL", path=path)
            assert result is False

    def test_update_dry_run(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            result = update_status("FL-001", "PARTIAL", path=path, dry_run=True)
            assert result is True
            # Content should NOT have appended subsection
            content = path.read_text()
            assert "OPEN -> PARTIAL" not in content

    def test_append_only_preserves_history(self):
        """Multiple updates create multiple subsections, never rewrite."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            update_status("FL-001", "PARTIAL", path=path)
            update_status(
                "FL-001", "RESOLVED",
                resolution="Done",
                evidence_refs=("commit xyz",),
                path=path,
            )
            content = path.read_text()
            # Both transitions recorded
            assert "OPEN -> PARTIAL" in content
            assert "-> RESOLVED" in content
            # Original line untouched
            assert "**Status:** OPEN" in content


class TestIdGeneration:
    def test_next_id_empty(self):
        assert next_failure_id([]) == "FL-001"

    def test_next_id_sequential(self):
        entries = [
            FailureEntry(
                failure_id=f"FL-{i:03d}", title="t", status="OPEN",
                date_opened="2026-01-01", category="t", description="d",
            )
            for i in range(1, 4)
        ]
        assert next_failure_id(entries) == "FL-004"


class TestFindOpenEntries:
    def test_from_fixture(self):
        entries = read_failure_log(FIXTURES_DIR / "sample_failure_log.md")
        open_entries = find_open_entries(entries)
        # FL-001 (OPEN) and FL-002 (PARTIAL) should be included
        ids = [e.failure_id for e in open_entries]
        assert "FL-001" in ids
        assert "FL-002" in ids
        # FL-003 (MONITORING) should NOT be in open
        assert "FL-003" not in ids


class TestStaleOpenEntries:
    def test_old_entries_are_stale(self):
        """Entries opened >7 days ago should be stale."""
        entries = [
            FailureEntry(
                failure_id="FL-001", title="old", status="OPEN",
                date_opened="2025-01-01", category="test",
                description="very old",
            ),
        ]
        stale = find_stale_open_entries(entries, stale_days=7)
        assert len(stale) == 1

    def test_recent_entries_not_stale(self):
        """Entries opened today should not be stale."""
        from datetime import datetime, timezone
        today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        entries = [
            FailureEntry(
                failure_id="FL-001", title="new", status="OPEN",
                date_opened=today, category="test",
                description="just opened",
            ),
        ]
        stale = find_stale_open_entries(entries, stale_days=7)
        assert len(stale) == 0

    def test_monitoring_excluded(self):
        """MONITORING entries are not OPEN/PARTIAL, so not stale."""
        entries = [
            FailureEntry(
                failure_id="FL-001", title="old", status="MONITORING",
                date_opened="2025-01-01", category="test",
                description="old but monitoring",
            ),
        ]
        stale = find_stale_open_entries(entries, stale_days=7)
        assert len(stale) == 0

    def test_no_date_skipped(self):
        """Entries without a date are skipped (not assumed stale)."""
        entries = [
            FailureEntry(
                failure_id="FL-001", title="no date", status="OPEN",
                date_opened="", category="test",
                description="missing date",
            ),
        ]
        stale = find_stale_open_entries(entries, stale_days=7)
        assert len(stale) == 0

    def test_from_fixture(self):
        """Sample fixture has entries dated 2026-02-18 — should be stale by now."""
        entries = read_failure_log(FIXTURES_DIR / "sample_failure_log.md")
        # FL-001 (OPEN) and FL-002 (PARTIAL) both dated 2026-02-18
        stale = find_stale_open_entries(entries, stale_days=1)
        ids = [e.failure_id for e in stale]
        assert "FL-001" in ids
        assert "FL-002" in ids


class TestEffectiveStatus:
    """Regression tests for effective_status — the append-only status update chain."""

    def test_update_then_reread_reflects_effective(self):
        """Update status to PARTIAL, re-read log, assert effective_status is PARTIAL."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            update_status("FL-001", "PARTIAL", path=path)

            entries = read_failure_log(path)
            assert len(entries) == 1
            assert entries[0].status == "OPEN"  # original preserved
            assert entries[0].effective_status == "PARTIAL"

    def test_resolved_effective_status_end_to_end(self):
        """Update to RESOLVED, re-read, assert effective_status is RESOLVED."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            update_status(
                "FL-001", "RESOLVED",
                resolution="Fixed via commit abc1234",
                evidence_refs=("commit abc1234",),
                path=path,
            )

            entries = read_failure_log(path)
            assert entries[0].effective_status == "RESOLVED"
            assert entries[0].status == "OPEN"  # original line untouched

    def test_resolved_excluded_from_open_entries(self):
        """Entries updated to RESOLVED should not appear in find_open_entries()."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            e1 = FailureEntry(
                failure_id="FL-001", title="resolved one", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="will be resolved",
            )
            e2 = FailureEntry(
                failure_id="FL-002", title="still open", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="stays open",
            )
            append_entry(e1, path=path)
            append_entry(e2, path=path)
            update_status(
                "FL-001", "RESOLVED",
                resolution="Fixed",
                evidence_refs=("commit xyz",),
                path=path,
            )

            entries = read_failure_log(path)
            open_entries = find_open_entries(entries)
            open_ids = [e.failure_id for e in open_entries]
            assert "FL-001" not in open_ids  # resolved, excluded
            assert "FL-002" in open_ids      # still open

    def test_resolved_excluded_from_stale_entries(self):
        """Entries updated to RESOLVED should not appear in find_stale_open_entries()."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="old resolved", status="OPEN",
                date_opened="2025-01-01", category="test",
                description="old but resolved",
            )
            append_entry(entry, path=path)
            update_status(
                "FL-001", "RESOLVED",
                resolution="Fixed",
                evidence_refs=("commit xyz",),
                path=path,
            )

            entries = read_failure_log(path)
            stale = find_stale_open_entries(entries, stale_days=7)
            assert len(stale) == 0  # resolved entries are never stale

    def test_multiple_updates_last_wins(self):
        """Multiple status updates: the last one becomes effective_status."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            update_status("FL-001", "PARTIAL", path=path)
            update_status("FL-001", "MONITORING", path=path)

            entries = read_failure_log(path)
            assert entries[0].effective_status == "MONITORING"
            assert entries[0].status == "OPEN"  # original unchanged

    def test_last_update_date_populated(self):
        """After status update, last_update_date should be set."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            update_status("FL-001", "PARTIAL", path=path)

            entries = read_failure_log(path)
            assert entries[0].last_update_date != ""


class TestResolvedInvariant:
    """RESOLVED status requires resolution text or evidence at creation time."""

    def test_resolved_without_resolution_or_evidence_raises(self):
        with pytest.raises(ValueError, match="RESOLVED"):
            FailureEntry(
                failure_id="FL-001", title="test", status="RESOLVED",
                date_opened="2026-02-19", category="test",
                description="desc",
            )

    def test_resolved_with_resolution_passes(self):
        entry = FailureEntry(
            failure_id="FL-001", title="test", status="RESOLVED",
            date_opened="2026-02-19", category="test",
            description="desc",
            resolution="Fixed the bug",
        )
        assert entry.status == "RESOLVED"

    def test_resolved_with_evidence_passes(self):
        entry = FailureEntry(
            failure_id="FL-001", title="test", status="RESOLVED",
            date_opened="2026-02-19", category="test",
            description="desc",
            evidence=["commit abc1234"],
        )
        assert entry.status == "RESOLVED"

    def test_effective_resolved_without_evidence_raises(self):
        """Setting effective_status=RESOLVED without resolution/evidence should raise."""
        with pytest.raises(ValueError, match="RESOLVED"):
            FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
                effective_status="RESOLVED",
            )


class TestHybridStaleDetection:
    """Stale detection uses max(date_opened, last_update_date) as activity date."""

    def test_recent_update_resets_stale_clock(self):
        """An entry opened long ago but recently updated is NOT stale."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="old but active", status="OPEN",
                date_opened="2025-01-01", category="test",
                description="old entry",
            )
            append_entry(entry, path=path)
            # Update today — this resets the activity clock
            update_status("FL-001", "PARTIAL", path=path)

            entries = read_failure_log(path)
            stale = find_stale_open_entries(entries, stale_days=7)
            # Should NOT be stale — last_update_date is today
            assert len(stale) == 0

    def test_no_update_uses_date_opened(self):
        """Without any updates, date_opened is the activity date."""
        entries = [
            FailureEntry(
                failure_id="FL-001", title="old", status="OPEN",
                date_opened="2025-01-01", category="test",
                description="old and inactive",
            ),
        ]
        stale = find_stale_open_entries(entries, stale_days=7)
        assert len(stale) == 1


class TestLongOpenEntries:
    """find_long_open_entries: flags entries opened >N days ago regardless of updates."""

    def test_old_entry_is_long_open(self):
        entries = [
            FailureEntry(
                failure_id="FL-001", title="ancient", status="OPEN",
                date_opened="2025-01-01", category="test",
                description="very old",
            ),
        ]
        long_open = find_long_open_entries(entries, long_open_days=30)
        assert len(long_open) == 1

    def test_recent_entry_not_long_open(self):
        from datetime import datetime, timezone
        today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        entries = [
            FailureEntry(
                failure_id="FL-001", title="new", status="OPEN",
                date_opened=today, category="test",
                description="just opened",
            ),
        ]
        long_open = find_long_open_entries(entries, long_open_days=30)
        assert len(long_open) == 0

    def test_resolved_excluded(self):
        """RESOLVED entries (even old) are not long_open."""
        entries = [
            FailureEntry(
                failure_id="FL-001", title="old resolved", status="RESOLVED",
                date_opened="2025-01-01", category="test",
                description="old but fixed",
                resolution="Fixed",
                effective_status="RESOLVED",
            ),
        ]
        long_open = find_long_open_entries(entries, long_open_days=30)
        assert len(long_open) == 0

    def test_recently_updated_old_entry_still_long_open(self):
        """An entry opened long ago is long_open even if recently updated."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="ancient active", status="OPEN",
                date_opened="2025-01-01", category="test",
                description="old but managed",
            )
            append_entry(entry, path=path)
            update_status("FL-001", "PARTIAL", path=path)

            entries = read_failure_log(path)
            # Not stale (recently updated)
            stale = find_stale_open_entries(entries, stale_days=7)
            assert len(stale) == 0
            # But IS long_open (opened >30 days ago)
            long_open = find_long_open_entries(entries, long_open_days=30)
            assert len(long_open) == 1

    def test_no_date_skipped(self):
        """Entries without a date are skipped (not assumed long-open)."""
        entries = [
            FailureEntry(
                failure_id="FL-001", title="no date", status="OPEN",
                date_opened="", category="test",
                description="missing date",
            ),
        ]
        long_open = find_long_open_entries(entries, long_open_days=30)
        assert len(long_open) == 0


class TestTransitionLabel:
    """update_status() should use effective status for the OLD in 'OLD -> NEW'."""

    def test_second_update_labels_from_effective(self):
        """After OPEN->PARTIAL, next update should say PARTIAL->MONITORING."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            update_status("FL-001", "PARTIAL", path=path)
            update_status("FL-001", "MONITORING", path=path)

            content = path.read_text()
            # First update: OPEN -> PARTIAL
            assert "OPEN -> PARTIAL" in content
            # Second update: PARTIAL -> MONITORING (NOT OPEN -> MONITORING)
            assert "PARTIAL -> MONITORING" in content
            assert "OPEN -> MONITORING" not in content

    def test_third_update_uses_last_effective(self):
        """After OPEN->PARTIAL->MONITORING, next says MONITORING->RESOLVED."""
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "FAILURE_LOG.md"
            entry = FailureEntry(
                failure_id="FL-001", title="test", status="OPEN",
                date_opened="2026-02-19", category="test",
                description="desc",
            )
            append_entry(entry, path=path)
            update_status("FL-001", "PARTIAL", path=path)
            update_status("FL-001", "MONITORING", path=path)
            update_status(
                "FL-001", "RESOLVED",
                resolution="Done",
                evidence_refs=("commit abc",),
                path=path,
            )

            content = path.read_text()
            assert "MONITORING -> RESOLVED" in content
            assert "OPEN -> RESOLVED" not in content


class TestEntryToMarkdown:
    def test_renders_all_fields(self):
        entry = FailureEntry(
            failure_id="FL-042", title="Test Entry",
            status="OPEN", date_opened="2026-02-19",
            category="pipeline", description="A test",
            root_cause="Bad code", evidence=["commit abc"],
            related_ids=["FL-001"],
        )
        md = entry_to_markdown(entry)
        assert "### FL-042" in md
        assert "OPEN" in md
        assert "pipeline" in md
        assert "commit abc" in md
        assert "FL-001" in md
