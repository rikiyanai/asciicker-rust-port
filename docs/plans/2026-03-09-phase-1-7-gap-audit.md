# Phase 1-7 Gap Audit

Date: 2026-03-09

## Purpose

This document records the remaining gaps between the claimed Phase 1-7 completion state and the code/docs currently in the repository. It is intended to guide the next implementation wave before Phase 8 work proceeds.

## Executive Summary

The repo is not cleanly at "Phase 7 complete" despite `.planning/STATE.md` claiming that status. The largest remaining gaps are in rendering integration, water/reflection compositing, partial shape-vector integration, and planning/doc drift.

The highest-value next step is still in the renderer:

1. Port the original mixed reflection/non-reflection `auto_mat` resolve branch from `render.cpp`.
2. Replace the remaining placeholder/stub render paths.
3. Reconcile plan/state docs so future work is not built on false completion claims.

## Confirmed Gaps

### 1. Planning State Drift

- `.planning/ROADMAP.md` still marks `07-06-PLAN.md` and `07-07-PLAN.md` as incomplete.
- `.planning/STATE.md` says "Phase 7 COMPLETE" and "all 7 plans including gap closures".
- `AGENTS.md` requires `python3 scripts/conductor_tools.py status --auto-setup`, but `scripts/conductor_tools.py` does not exist in this repo.

Impact:
- The repo has no single trustworthy status source right now.
- Phase 8 execution on top of this state would inherit bad assumptions.

Required action:
- Reconcile `.planning/ROADMAP.md`, `.planning/STATE.md`, and actual code/test evidence.
- Restore or replace the missing conductor entrypoint referenced by `AGENTS.md`.

### 2. Phase 5 Rendering Integration Is Still Partial

Evidence:
- [pipeline.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/pipeline.rs) still has `Stage 4: SHADOW (stub -- future)`.
- [pipeline.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/pipeline.rs) disables BSP frustum culling in the WORLD stage with `TODO(frustum)`.
- [sprite_blit.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/sprite_blit.rs) still blits a placeholder `'S'` instead of full XP sprite frames.
- [water.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/water.rs) still has `TODO: re-query world with flipped frustum` for world mesh reflections.
- `.planning/phases/05-pipeline-integration/05-VERIFICATION.md` still marks VIS-02 as `PARTIAL`.

Impact:
- Real scene rendering still diverges from the original engine in exactly the areas that matter for final visual parity.
- Water/reflection behavior cannot be trusted while the resolve and reflection stages remain incomplete.

Required action:
- Finish Stage 4 shadow integration.
- Re-enable proper world/BSP frustum culling in render units.
- Replace placeholder sprite blit with real XP frame compositing and depth policy.
- Implement world mesh/sprite reflections in the water pass.
- Keep VIS-02 marked partial until actual C++ reference capture exists.

### 3. Current Water Bug Is Upstream of Ripple Animation

Evidence:
- [FAILURE_LOG.md](/Users/rikihernandez/Downloads/asciicker-rust-port/docs/FAILURE_LOG.md) entries `F243` and `F244`.
- First-frame diagnostics from the current build show underwater cells are already blank before ripple:
  - `underwater_cells=2321`
  - `blank_before=2034`
  - `invalid_projection=0`
  - `blank_after=2034`
- The original C++ renderer has a mixed reflected/non-reflected terrain resolve branch in [(ORIGINAL GAME)asciicker-Y9-2-main/render.cpp](/Users/rikihernandez/Downloads/asciicker-rust-port/(ORIGINAL%20GAME)asciicker-Y9-2-main/render.cpp#L3515) that the Rust port does not implement.

Impact:
- More Perlin/water-plane tuning is low leverage until the mixed-cell resolve/composite logic is ported.
- The "reflection overlay" symptom is plausibly a missing resolve path, not a missing animation primitive.

Required action:
- Port the original `use_auto_mat` mixed reflection/non-reflection resolve branch into [resolve.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/resolve.rs).
- Re-test water in release mode after that port before making more ripple changes.

### 4. Phase 6 Physics/Character Still Has Approximation Paths

Evidence:
- [geometry.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/physics/geometry.rs) still uses AABB proxy triangles for world collision with `TODO (Phase 7): Replace bbox proxy with actual AkmMesh triangles.`
- [state_machine.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/character/state_machine.rs) still documents dead-state respawn/menu-return as TODO.
- [physics/mod.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/physics/mod.rs) retains the auto-jump/step-climb TODO path noted in planning.

Impact:
- Collision fidelity and gameplay correctness are still approximate in mesh-heavy scenes.
- Character lifecycle is not actually finished even if basic locomotion works.

Required action:
- Replace bbox collision proxy with actual transformed mesh triangles.
- Implement respawn/menu-return flow for Dead state.
- Decide whether step-climb/auto-jump is required for parity or intentionally omitted.

### 5. Phase 7 Shape-Vector / 6D Sampling Is Only an Intermediate Bridge

Evidence:
- [shape_vector.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/shape_vector.rs) hardcodes the `six-samples` alphabet only.
- [shape_vector.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/shape_vector.rs) samples only six positions from the 2x2 sample block and falls back to auto_mat when distance exceeds a threshold.
- [shape_vector.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/shape_vector.rs) skips underwater cells entirely, so water never benefits from the matcher.
- [font.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/font.rs) provides a `Font1` API but there is no registration or calling system using it.
- [plan-SampleBuffer-bridge.md](/Users/rikihernandez/Downloads/asciicker-rust-port/docs/plan-SampleBuffer-bridge.md) is explicitly marked contingent and was never fully executed as written.
- The current runtime font atlas in the output path is separate from the matcher data, so glyph vectors are not proven font-specific.

Impact:
- The repo has a usable partial matcher, not a completed "Alex Harri 6D shape-vector integration".
- Edge quality may improve in some cases, but parity claims are overstated.
- This path should not be treated as final scope; it still needs the remaining full-port decisions and implementation work.

Required action:
- Treat the current six-samples matcher as intermediate only.
- Complete the full port work:
  - regenerate vectors for the actual runtime font atlas
  - validate sampling strategy against the chosen alphabet
  - wire `Font1` into actual menu/HUD/chat systems
  - decide whether water cells should participate in shape matching after water resolve is fixed

### 6. Mage Core Logic Was Only Partially Ported

Evidence:
- [output/mod.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/output/mod.rs) and [gpu_plugin.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/output/gpu_plugin.rs) clearly implement the Mage Core-style 4-texture GPU output pattern.
- The repo does not implement the full Mage Core engine/application logic; it uses Bevy as the game/application framework and only borrows the output architecture.
- `.planning/PROJECT.md` and older reference docs still describe the output path in ways that can be misread as "Mage Core fully copied", even though only the render/output approach was adopted.

Impact:
- The output stack is "Mage Core inspired / partially ported", not "Mage Core logic fully ported".
- Future work should not assume parity with Mage Core outside the GPU ASCII output layer.

Required action:
- Document the boundary explicitly: Bevy owns the app/runtime architecture; Mage Core logic is only reused at the output/render-pattern level.
- Audit the current output path against Mage Core to see whether any remaining shader/upload behaviors still matter for parity or performance.
### 7. Phase 7 Networking and Weather Claims Need Evidence Recheck

Evidence:
- `.planning/ROADMAP.md` still leaves `07-06-PLAN.md` and `07-07-PLAN.md` unchecked.
- [network/mod.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/network/mod.rs) defaults to standalone mode and only conditionally registers transport.
- No standalone integration test files were found under `engine-port/tests/` for network transport.
- `.planning/phases/07-game-systems/07-05-SUMMARY.md` says automatic weather state trigger was deferred.

Impact:
- Network/weather may be "implemented enough for local development" but not necessarily complete enough to justify Phase 7 completion claims.

Required action:
- Re-verify actual network integration tests and their execution status.
- Re-verify whether the weather debug keybind and runtime state transition path are truly present and tested.

## Recommended Execution Order

1. Reconcile planning status docs and missing conductor entrypoint.
2. Finish the water/reflection resolve port:
   - mixed-cell `auto_mat` branch
   - world reflections
   - release-mode visual verification
3. Replace render placeholders:
   - shadow stage
   - real sprite blit
   - world frustum culling
4. Close physics approximations:
   - real mesh collision triangles
   - dead-state flow
5. Re-baseline the visual architecture scope:
   - complete the full Alex Harri shape-vector port
   - document Mage Core as partial output-pattern reuse, not full engine parity
   - align font/matcher/sampling/runtime output
6. Re-run Phase 7 verification with evidence and only then unlock Phase 8.

## Files To Revisit First

- [resolve.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/resolve.rs)
- [pipeline.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/pipeline.rs)
- [water.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/water.rs)
- [sprite_blit.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/sprite_blit.rs)
- [geometry.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/physics/geometry.rs)
- [shape_vector.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/shape_vector.rs)
- [font.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/font.rs)
- [ROADMAP.md](/Users/rikihernandez/Downloads/asciicker-rust-port/.planning/ROADMAP.md)
- [STATE.md](/Users/rikihernandez/Downloads/asciicker-rust-port/.planning/STATE.md)
