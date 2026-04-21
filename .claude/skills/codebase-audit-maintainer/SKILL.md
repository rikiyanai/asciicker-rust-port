---
name: codebase-audit-maintainer
description: Use when refreshing codebase audit docs and validating planner claims against source/commit evidence; prevents stale architecture docs and unsupported completion claims.
---

# Skill: Codebase Audit Maintainer

Keep architecture/audit docs current and force evidence-backed planning claims.

## When To Use

- Before marking roadmap or plan tasks `complete`.
- After major merges/cherry-picks/stash recoveries.
- When planner outputs include implementation claims.
- On a weekly maintenance cadence to prevent doc drift.

## Preflight (Required)

```bash
git status --short
git branch --all
git worktree list
git stash list
```

## Inputs (Canonical Priority)

> **Note:** `.planning/` directory will be created by GSD initialization. Until then, `MASTER_ROADMAP.md` at project root serves as the status authority.

1. `AGENTS.md`
2. `docs/CANONICAL_SPEC.md`
3. `docs/FAILURE_LOG.md`
4. `docs/worksheets/INDEX.md`
5. `docs/worksheets/AGENT_PROTOCOL.md`
6. `.planning/ROADMAP.md` (canonical status ledger) — or `MASTER_ROADMAP.md` (interim authority before GSD init)
7. `.planning/PROJECT.md` (acceptance outcomes and active scope)
8. Active roadmap/plan docs under `docs/worksheets/plans/` (execution detail)
9. `.planning/STATE.md` (session context only; non-authoritative for completion)
10. Source-of-truth code and commit history

If conflicts exist, update lower-priority docs to match higher-priority sources.

## Workflow

1. Inventory active claims
- Extract all "done/complete/active/deferred" statements from active plans.
- Build a checklist keyed by task id.

2. Verify each claim against code reality
- Confirm implementation exists at concrete file/line locations.
- Confirm commit evidence exists on the intended branch.
- Confirm verification command(s) exist and were run (`cargo test` output).
  > **Note:** `cargo test` requires `Cargo.toml` to exist first (created in Phase 1 project scaffolding). Before Phase 1, verification is limited to doc-level checks.

3. Refresh architecture audit docs
- Update subsystem docs only where behavior/contracts changed.
- Keep callgraphs, entrypoints, invariants, and known traps aligned with current source.
- Mark stale sections with `TODO(stale-audit):` instead of silently deleting context.

4. Reconcile branch/worktree/stash hygiene
- Ensure critical commits are reachable from the integration branch.
- Reduce stash count to zero by explicit integrate/drop decisions.

5. Publish evidence-backed handoff
- Emit one status block with: completed, deferred, risks, next command.

## Claim Verification Rules

A planner claim is valid only when all pass:

- `Code Evidence`: file path + line reference.
- `Commit Evidence`: hash containing the change.
- `Verification Evidence`: command + pass/fail result.
- `Status Alignment`: `ROADMAP` is authoritative; plans/STATE must align to it.

If any evidence is missing, mark `NOT VERIFIED` and downgrade completion status.

## Integration With Existing Skills

- Use `agent-context-doc-health` for cross-doc and git-state alignment.
- Use `xp-pipeline-verifier` for asset pipeline completion gates.
- Use subsystem skill docs (`engine-render`, `world-loading`, `physics-system`, `game-mechanics`) as audit baselines.

## Guardrails

- Never mark `complete` from test counts alone.
- Never trust planner prose without source/commit verification.
- Never leave ambiguous status wording ("mostly done", "should be done").
- Prefer small, traceable doc updates over large speculative rewrites.
