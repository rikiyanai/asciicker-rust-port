# Agent Protocol

This file defines the agent startup protocol and evidence rules. Entry point: AGENTS.md.

## Source of Truth
- Primary operational rules: [`AGENTS.md`](../../AGENTS.md)
- Canonical spec: [`docs/CANONICAL_SPEC.md`](../CANONICAL_SPEC.md)
- Failure log: [`docs/FAILURE_LOG.md`](../FAILURE_LOG.md)
- Claude-specific memory/instructions: [`CLAUDE.md`](../../CLAUDE.md)
- Worksheet index: [`docs/worksheets/INDEX.md`](INDEX.md)
- Canonical status ledger: [`.planning/ROADMAP.md`](../../.planning/ROADMAP.md)
- Acceptance outcomes and scope contract: [`.planning/PROJECT.md`](../../.planning/PROJECT.md)

### Status Authority Model
- Use `docs/CANONICAL_SPEC.md` for durable architecture, decision, and code/doc truth.
- Use `docs/FAILURE_LOG.md` for failures, blockers, regressions, and proof gaps.
- Use `.planning/ROADMAP.md` as the single source of truth for phase/status state.
- Use `docs/worksheets/plan-*.md` and `docs/worksheets/plans/*.md` for execution details and checklists, not canonical status.
- Use `.planning/STATE.md` as transient session context only (non-authoritative for completion claims).
- Any completion claim must match `docs/CANONICAL_SPEC.md`, `docs/FAILURE_LOG.md`, `.planning/ROADMAP.md`, and commit/test evidence.

## Required Startup Sequence
1. Read `AGENTS.md`.
2. Read `docs/CANONICAL_SPEC.md`.
3. Read `docs/FAILURE_LOG.md`.
4. Read `docs/worksheets/INDEX.md`.
5. Read `CLAUDE.md` if using Claude tooling.
6. Run `git status` and resolve any blockers first.
7. Confirm active planning artifacts in `.planning/` before making changes. If `.planning/` does not exist, GSD has not been initialized. Use `MASTER_ROADMAP.md` as interim status authority.

## Working Rules
- Prefer minimal, focused commits.
- Verify commands and file paths against the current workspace state.
- Log significant architectural changes in docs and planning artifacts.
- Treat exploration as hypothesis, not validation.
  - Code search / "explore agent" outputs are context only.
  - Runtime claims require executed test evidence (`cargo test` output).
- If user reports breakage, reproduce before extending scope.
  - Add a failing test first.
  - Only mark fixed after that test passes.

## Explorer Agent Guardrails
1. Explorer agents may map files, call graphs, and likely root causes.
2. Explorer agents may not label status as PASS/COMPLETE.
3. Any explorer conclusion must be followed by:
   - a runnable reproduction command, and
   - a verification command for the proposed fix.
4. If reproduction is missing, treat the explorer output as unverified.

## Documentation Hygiene
- Treat `docs/worksheets/INDEX.md` as the worksheet table of contents, not canon.
- Add new worksheet docs to `docs/worksheets/INDEX.md`.
- Move durable facts into `docs/CANONICAL_SPEC.md` or `docs/FAILURE_LOG.md`.
- Mark historical or stale material as legacy.
