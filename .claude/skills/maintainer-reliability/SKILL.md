---
name: maintainer-reliability
description: "Prevent repeated failure loops, false completion claims, doc drift, and repo mess. Enforce evidence-backed status claims and session-end hygiene."
---

# Skill: Maintainer Reliability

Prevent repeated failure loops, false completion claims, doc drift, and repo mess.
Enforce evidence-backed status claims and session-end hygiene.

## When To Use

- At session end: verify zero-stash policy and branch hygiene.
- Before committing: ensure completion claims have evidence.
- When auditing repo health: check doc alignment.
- When logging a regression or failure: use the failure log.

## Key Concepts

### Claim Guard
Forbidden words (`resolved`, `fixed`, `complete`, `done`, `passed`, etc.) trigger warnings
unless accompanied by evidence refs (failure log IDs or 7+ char commit hashes).

**Hyphen compound immunity:** "closed-form", "fail-safe", "working-directory" do NOT trigger.
**Word boundary matching:** "completeness" does NOT trigger "complete".

### Failure Log
Canonical path: `docs/FAILURE_LOG.md`

- Append-only: status changes are blockquote subsections, never in-place edits.
- Status vocabulary: OPEN, PARTIAL, MONITORING, RESOLVED.
- RESOLVED requires resolution text or evidence ref.
- Effective status = last update subsection's target status.

### Stale Detection
- **Stale** (score penalty): `now - max(date_opened, last_update_date) >= 7 days`
- **Long-open** (info only): `now - date_opened >= 30 days`

### Health Score
Weighted rubric summing to 100:
| Category | Weight | What It Checks |
|----------|--------|----------------|
| failure_log_exists | 15 | FAILURE_LOG.md is present |
| failure_log_hygiene | 20 | No stale OPEN/PARTIAL entries |
| artifact_presence | 15 | Recent audit artifacts exist |
| claim_quality | 25 | Low unsupported claim ratio |
| stale_open_penalty | 25 | Few stale entries (5 pts each, cap 25) |

## Session-End Checklist

> **Note:** `.planning/` directory will be created by GSD initialization. Until then, `MASTER_ROADMAP.md` at project root serves as the status authority.

```bash
# 1. Check git state
git status --short
git stash list        # Must be empty
git worktree list     # Single worktree expected

# 2. Verify tests pass (requires implementation to have started — Cargo.toml must exist)
cargo test

# 3. Check doc alignment
# Roadmap status matches commit evidence
# No orphaned plan docs
```

> **Note:** `cargo test` in step 2 requires Rust implementation to have started (Cargo.toml and at least one crate must exist). Before that point, skip step 2 and focus on git hygiene and doc alignment.

## Guardrails

- Failure log is strictly append-only during a session.
- All completion claims require commit hash evidence.
- No tool modifies `~/.claude/` directly.
- Prefer small, traceable doc updates over large speculative rewrites.
