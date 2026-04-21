---
name: agent-context-doc-health
description: Use when auditing or updating agent-facing docs, roadmap/plan state, and git branch/stash/worktree hygiene; enforces cross-doc alignment and evidence-backed completion claims.
---

# Skill Pack: Agent Context Document Health

Keep agent-facing docs, roadmap state, and branch reality in sync. This skill prevents drift that causes repeated regressions, duplicate work, and false completion claims.

**Primary outcome:** one verified source of truth for status and completion claims, with explicit authority and commit/test evidence.

---

## 1. When To Use

Use this skill when any of these are true:

- Multiple agents (Codex/Claude) touched roadmap, plans, or handoff docs.
- A feature appears "done" in docs but missing in UI/code.
- Regressions reappear after branch switches/cherry-picks.
- There are stacked stashes/worktrees or unclear branch lineage.
- You are preparing a handoff or resuming after a long session.

---

## 2. Mandatory Preflight

Run in this order:

```bash
# 1) Snapshot git topology
git status --short
git branch --all
git worktree list
git stash list

# 2) Verify build
# Skip cargo commands if no Cargo.toml exists (pre-Phase 1)
test -f Cargo.toml && cargo build 2>&1 | tail -5 || echo "No Cargo.toml — skip build (pre-implementation)"
test -f Cargo.toml && cargo test 2>&1 | tail -10 || echo "No Cargo.toml — skip test (pre-implementation)"
```

---

## 3. Canonical Sources (Strict Order)

> **Note:** `.planning/` directory will be created by GSD initialization. Until then, `MASTER_ROADMAP.md` at project root serves as the status authority.

1. `AGENTS.md` (repo-global operating rules)
2. `docs/CANONICAL_SPEC.md` (durable architecture, decisions, and code/doc truth)
3. `docs/FAILURE_LOG.md` (**append-only failure record**)
4. `docs/worksheets/INDEX.md` (worksheet inventory)
5. `docs/worksheets/AGENT_PROTOCOL.md` (agent startup + evidence rules)
6. `.planning/ROADMAP.md` (**canonical status ledger**) — or `MASTER_ROADMAP.md` (interim authority before GSD init)
7. `.planning/PROJECT.md` (acceptance outcomes and active scope)
8. Active roadmap/plan docs in `docs/worksheets/plans/` (execution detail)
9. `.planning/STATE.md` (session context only; non-authoritative for completion)
10. `CLAUDE.md` (Claude memory/constraints)
11. Live git evidence (commits/branches/stashes/worktrees)

If any lower-priority source conflicts with a higher-priority one, fix the lower-priority source.

### Status Authority Rules

- Completion status lives in `.planning/ROADMAP.md` (once GSD creates it). Before GSD initialization, `MASTER_ROADMAP.md` at project root is the interim status authority.
- Plan docs (`docs/worksheets/plans/*.md`) may define tasks/checklists but may not override roadmap status.
- `.planning/STATE.md` can summarize progress but cannot be used as evidence to mark completion.
- If roadmap and plan differ, roadmap wins; update plan doc to align and include evidence.

---

## 4. Drift Audit Workflow

### A. Plan/roadmap vs code reality

```bash
# Phase references in code and plans
rg -n "Phase [0-9]" docs/worksheets/plans docs .planning

# Completion claims in docs
rg -n "status:\s*(complete|completed|done|active|draft)" docs/worksheets/plans .planning docs

# Commit evidence for claimed work
git log --oneline --decorate --graph --max-count=200 --grep='phase\|render\|terrain\|world\|physics\|game'
```

Rules:

- "Done" requires commit(s) and verification evidence.
- If evidence is missing, downgrade status to `active` or `draft`.
- If work is intentionally postponed, mark `deferred` with rationale.

### B. Cross-doc alignment matrix

Build and check this matrix for each active initiative:

- Roadmap item id/title
- Plan doc path + status
- Implementing commit hash(es)
- Verification command(s) and result
- Deferred items + reason

Any blank field is a blocker for completion claims.

Additionally enforce:
- `docs/CANONICAL_SPEC.md` and `docs/FAILURE_LOG.md` must not contradict `.planning/ROADMAP.md` status or proof state.
- `docs/worksheets/INDEX.md` entries must not present worksheet material as canonical.
- Ship decisions in plan docs must match active blockers in roadmap/project (e.g., policy gates).

### C. Git hygiene and loss-risk audit

```bash
git stash list
git worktree list
git branch --contains <critical_commit_hash>
```

Rules:

- Stash count must be `0` at session end.
- One active delivery branch per workstream.
- Critical commits must be reachable from intended integration branch.

---

## 5. Update Rules (No Exceptions)

- Never silently rewrite history in docs.
- Include concrete commit hashes when marking items complete.
- Include explicit "NOT DONE" checklist entries for skipped acceptance criteria.
- When renumbering/re-scoping phases, update all references in one pass.
- Add durable truths to `docs/CANONICAL_SPEC.md` or `docs/FAILURE_LOG.md` immediately.
- Keep `docs/worksheets/INDEX.md` limited to worksheet inventory and discovery.
- Mark superseded docs clearly as `legacy`/`superseded` to prevent accidental reuse.
- If changing status in `docs/worksheets/plans/*`, sync `.planning/ROADMAP.md` in the same commit.

---

## 6. Failure Logging (Mandatory)

Every failure, regression, broken claim, or visual-correctness rejection discovered during a doc-health audit **must** be appended to the active failure log. Failures reported only in chat or agent output are considered lost — the failure log is the durable record.

### Locate the active failure log

```bash
ls docs/FAILURE_LOG.md 2>/dev/null
```

If no failure log exists, create one at `docs/FAILURE_LOG.md`.

### What to log

Log an entry whenever any of these are true:

- A completion claim in roadmap/plan docs has no matching commit or verification evidence.
- A gate (AC-1 native sources, AC-2 typical sheets) fails or is rejected by the user.
- A previously "fixed" bug is found to be still broken or regressed.
- A test is identified as false-positive (claims coverage it does not provide).
- Pipeline output is visually wrong per user or MCP inspection.
- A config mismatch or behavioral divergence is discovered between docs and code.

### Entry format

Each entry must include:

- **Date and HEAD commit** at time of discovery.
- **What failed** — specific gate, claim, or behavior.
- **Evidence** — command output, user quote, MCP result, or commit hash.
- **Root cause** (if known) — or `UNKNOWN` with investigation notes.
- **Status** — `BROKEN`, `FIXED (commit <hash>)`, or `DEFERRED (reason)`.

Append to the `## ACTIVE ISSUES` section (or `## Open Failures` if using that format) if the failure is unresolved. Move to a numbered Cycle section if the failure has been investigated and classified.

### Rules

- The failure log is **append-only** during a session. Do not delete or rewrite existing entries.
- Existing entries may be updated to add root cause, fix commit, or status change.
- When marking a failure as FIXED, include the commit hash and verification evidence.
- Cross-reference the failure log entry number in roadmap/plan updates (e.g., "See failure log #8").
- At session end, verify the failure log's `Open Failures` section matches reality.

---

## 7. Required Handoff Artifact

Produce a single handoff block with:

1. `Branch + head commit`
2. `What is complete` (with hashes)
3. `What is deferred` (with reason)
4. `Known regressions/open risks`
5. `Exact next command to resume`

Template:

```md
## Handoff Snapshot
- Branch: <branch>
- HEAD: <hash>
- Completed:
  - <item> (<hash>)
- Deferred:
  - <item> (reason: <reason>)
- Open Risks:
  - <risk>
- Resume:
  - <command>
```

---

## 8. Definition Of Healthy Context

Context is healthy only when all are true:

- No contradictory status across roadmap/plan/protocol docs.
- Each "completed" claim maps to real commit evidence.
- Stashes are zero, and worktree intent is explicit.
- `docs/CANONICAL_SPEC.md` and `docs/FAILURE_LOG.md` reflect current durable truth.
- `docs/worksheets/INDEX.md` points to active worksheet material without claiming canonical authority.
- Failure log `Open Failures` section matches current reality (no stale entries, no unlogged failures).
- Incoming agent can resume from one handoff block without archaeology.

---

## 9. Porting Notes For Claude

To port this skill into Claude memory/rules:

- Keep the same canonical-source priority order.
- Keep the same evidence threshold (no completion claims without commit + verification).
- Keep the same stash/worktree zero-tolerance at session end.
- Reuse the handoff template verbatim for cross-agent continuity.
