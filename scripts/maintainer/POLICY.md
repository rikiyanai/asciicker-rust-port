# Maintainer Policy

Definitions and rules for the maintainer reliability system.

## Claim Guard

### Forbidden Words

The following words are flagged when used without evidence refs (FL-NNN or commit hash):
`resolved`, `fixed`, `complete`, `done`, `passed`, `green`, `working`, `closed`, `shipped`, `verified`

### Word Boundary Rules

- **Substring immunity**: "completeness" does NOT trigger "complete" (word boundary `\b` prevents this).
- **Hyphen compound immunity**: "closed-form", "fail-safe", "working-directory" do NOT trigger their base words. A hyphen adjacent to the word suppresses the match.
- **Standalone words always trigger**: "everything is complete", "the bug is fixed" — these trigger as expected.

### Evidence Refs

A forbidden word is downgraded from `unsupported_claim` to `claim_with_evidence` when the message also contains:
- An FL reference: `FL-001`, `FL-042`, etc.
- A commit hash: 7+ hex characters (e.g., `abc1234def`)

## Failure Log

### Status Vocabulary

| Status | Meaning |
|--------|---------|
| `OPEN` | Active, unresolved |
| `PARTIAL` | Partially addressed, still needs work |
| `MONITORING` | Fix applied, watching for regression |
| `RESOLVED` | Closed with evidence (requires resolution text + evidence ref) |

### Append-Only Updates

Status changes are recorded as blockquote subsections appended to the entry. The original `**Status:**` line is never edited. Example:

```markdown
**Status:** OPEN
...
> **[2026-02-20] Status update: OPEN -> PARTIAL**
> Added ceiling-division guard
> Evidence: commit abc1234
```

### Transition Labels

The `OLD -> NEW` label in status updates uses the **effective status** (from the most recent update subsection), not the original status line. After `OPEN -> PARTIAL`, the next update says `PARTIAL -> MONITORING`, not `OPEN -> MONITORING`.

## Stale vs Long-Open

Two complementary checks for unresolved failure log entries:

### Stale (score penalty)

- **Definition**: OPEN or PARTIAL entry with no activity for N days (default: 7).
- **Activity date**: `max(date_opened, last_update_date)` — a recent status update resets the staleness clock.
- **Effect**: Penalizes health score (5 points per stale entry, capped at 25).

### Long-Open (info-only)

- **Definition**: OPEN or PARTIAL entry opened more than M days ago (default: 30).
- **Activity date**: Uses `date_opened` only — recent updates do NOT reset this clock.
- **Effect**: Info-level finding in audit report. No score penalty.
- **Purpose**: Surfaces ancient unresolved problems even if they're being actively managed.

### Examples

| Opened | Last Updated | Stale (7d)? | Long-Open (30d)? |
|--------|-------------|-------------|-------------------|
| 90 days ago | Yesterday | No | Yes |
| 90 days ago | 30 days ago | Yes | Yes |
| 3 days ago | Never | No | No |
| 10 days ago | Never | Yes | No |

## Modes

- **warn** (MVP default): All tools exit 0. Findings are informational.
- **block** (Phase 2): High-severity findings cause exit 1. Reserved for future hook enforcement.
