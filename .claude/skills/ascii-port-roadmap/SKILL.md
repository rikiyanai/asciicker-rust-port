---
name: ascii-port-roadmap
description: Use when converting research findings into a phased roadmap for finishing the asset pipeline and defining the northstar engine-port path (Rust or alternative), with explicit gates, risks, and compatibility contracts.
---

# Skill: ASCII Port Roadmap

Turn research evidence into an executable roadmap across two tracks:
- Track A: ship the current asset pipeline reliably.
- Track B: plan and de-risk engine migration (Rust or other engine).

## Inputs

> **Note:** `.planning/` directory will be created by GSD initialization. Until then, `MASTER_ROADMAP.md` at project root serves as the status authority.

Required inputs:
- Archived Mage Core research docs in `docs/archive/engine-port-magecore/` (original `*-inventory.md`, `*-capability-matrix.md`, etc. from the earlier Mage Core evaluation). New Bevy-aligned versions of these inventories will be created during GSD initialization.
- Current roadmap + active plans in `docs/plans/`
- Current repo constraints from `AGENTS.md`, `CLAUDE.md`, and `docs/AGENT_PROTOCOL.md`

## Workflow

1. Lock compatibility contracts first
- Define non-negotiable artifact contracts before migration work:
  - XP layer/metadata behavior
  - Sprite slicing semantics
  - Branch/workbench handoff payloads

2. Build a two-track phased plan
- Track A (pipeline completion): defect closure, QA gates, release hardening.
- Track B (port northstar): architecture options, spike/prototype, cutover criteria.

3. For each phase, require:
- Objective
- In-scope / out-of-scope
- Deliverables
- Verification commands
- Exit criteria
- Risk + rollback plan

4. Add explicit decision gates
- `Gate 1`: keep current engine only vs start adapter layer
- `Gate 2`: Rust target confirmed vs alternate runtime
- `Gate 3`: dual-run parity achieved before migration claims

## Standard Output Structure

Create/update planning docs with:
- `status`
- `owner`
- `dependencies`
- `evidence`
- `deferred_items`

All roadmap items marked `complete` must include commit hash(es) and verification evidence.

## Guardrails

- No migration phase may start without a frozen compatibility contract.
- Do not collapse research findings into a single recommendation without alternatives.
- Keep "northstar" goals separate from current-release blockers.
- If evidence is incomplete, mark decision as `deferred` with unblock condition.

## Output Path

New planning artifacts go to `docs/plans/`.

## Recommended Deliverables

- `docs/plans/<date>-northstar-port-options.md`
- `docs/plans/<date>-pipeline-closeout-plan.md`
- `docs/plans/<date>-migration-gates-and-parity.md`

