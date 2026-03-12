# 2026-03-09 Water Next-Step Architecture Audit

## Question

Given repeated failed water fixes, should work continue on the water port directly, or should earlier phase feature-port completion happen first?

## Short Answer

Earlier feature-port completion should happen first.

The water bug is currently sitting on top of an incomplete renderer and several approximation paths. Continuing to patch water in isolation is likely to keep producing false negatives because the visible symptom is influenced by missing or simplified functionality in Phase 5, Phase 6, and Phase 7 visual work.

## Audit Basis

This audit uses:

- current code in `engine-port/src`
- planning docs in `.planning`
- failure evidence in `docs/FAILURE_LOG.md`
- the deterministic replay/capture work from 2026-03-09

## Architecture Findings

### 1. Planning status is not trustworthy after the snapshot restore

The restored planning docs currently claim:

- `.planning/ROADMAP.md`: Phases 5, 6, 7 complete
- `.planning/STATE.md`: Phase 7 complete and next is Phase 8

Code reality does not support those claims.

This matters because it changes the decision boundary: if earlier phases are only partially complete, then water is not a self-contained Phase 6 polish item.

### 2. The renderer core is still simplified in ways that directly affect water

#### 2.1 Resolve is still a simplified Phase 4-style compositor

In [resolve.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/resolve.rs), the output cell is still chosen from a dominant sample:

- `dominant_sample(...)` at line 156
- material/mesh branching from the OR-ed 2x2 block state
- header explicitly says it is "simplified for Phase 4"

That is a major architectural warning. Water/reflection artifacts often come from mixed-cell compositing, and the current resolve path is still structurally simpler than the original C++ resolve path.

#### 2.2 Stage 4 shadow is still a stub

In [pipeline.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/pipeline.rs), Stage 4 is still:

- `// Stage 4: SHADOW (stub -- future)`

That means the full pipeline advertised by the docs is not actually present.

#### 2.3 World frustum culling is disabled

Also in [pipeline.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/pipeline.rs):

- `TODO(frustum): BSP frustum culling disabled -- coordinate system mismatch`

The world stage is not using its intended visibility path. That is a correctness issue first, not just a performance issue.

#### 2.4 Water reflections are terrain-only

In [water.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/water.rs):

- `_world` is unused
- Step 4 is explicitly `TODO: re-query world with flipped frustum`

So the current reflection architecture is incomplete by definition.

### 3. Sprite rendering is still placeholder-grade

In [sprite_blit.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/sprite_blit.rs):

- the current blit is explicitly labeled `Phase 5 placeholder sprite blit`
- it stamps a literal `S`

That means at least one major post-resolve visual layer is still debug output rather than a ported runtime path. Any perceived overlay/jitter involving NPCs, items, or player-adjacent visuals is therefore harder to trust during water debugging.

### 4. Physics still uses approximation paths

In [geometry.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/physics/geometry.rs):

- world collision still uses AABB proxy triangles
- the file explicitly says: `TODO (Phase 7): Replace bbox proxy with actual AkmMesh triangles.`

That affects camera/player interaction and can contaminate render diagnosis through motion or contact instability.

### 5. Phase 7 visual work is only a partial implementation

#### 5.1 The current Alex Harri path is the simplified six-samples variant

In [shape_vector.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/shape_vector.rs):

- the file uses the compiled-in `six-samples` alphabet
- it is 6 internal samples only

Research and planning docs already note this is not the final quality path:

- `.planning/phases/07-game-systems/07-RESEARCH.md`
- `docs/research/alexharri-asciicker-integration.md`

Those documents explicitly call out directional crunch and external samples as later work with quality impact.

So the current six-samples matcher is an intermediate port, not the final visual target.

#### 5.2 Font1 exists as an API but is not an integrated gameplay/UI path

In [font.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/font.rs):

- `Font1` is a resource-only API
- the file explicitly says it does not register systems

So the Phase 7 “Font1 3 skins complete” claim is overstated. The API exists, but the full runtime wiring is not there.

### 6. Mage Core reuse is intentionally narrow

The Mage Core work was adopted for the GPU output architecture, not as a broader engine-logic transplant.

The relevant evidence:

- [gpu_plugin.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/output/gpu_plugin.rs) clearly implements the Mage Core-style 4-texture output path
- [research-mage-core.md](/Users/rikihernandez/Downloads/asciicker-rust-port/docs/research-mage-core.md) explicitly says Mage Core lacks the bulk of Asciicker game-engine logic

So if the expectation was “copy Mage Core engine logic broadly,” that did not happen and should not be the plan. The actual architecture is:

- Bevy for the game framework
- Mage Core ideas for final ASCII display
- original Asciicker C++ logic ported subsystem by subsystem

That part is not a bug. The real issue is that several of those subsystem ports are still incomplete.

## Conclusion

The water port should not be treated as the primary next work item.

The repeated failures are consistent with an incomplete renderer stack:

- simplified resolve
- missing world reflections
- stub shadow stage
- disabled world frustum culling
- placeholder sprite blit
- approximation physics
- partial final-visual pipeline

In that architecture, water bugs are not isolated enough to debug efficiently.

## Recommended Next Sequence

### Track A: Finish the renderer paths that directly contaminate water

1. Port the original mixed-cell resolve/compositing behavior into [resolve.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/resolve.rs).
2. Replace the placeholder `S` path in [sprite_blit.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/sprite_blit.rs) with real XP sprite blitting.
3. Implement world reflection rendering in [water.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/water.rs), not terrain-only reflections.

Only after those three are in place should water-specific visual tuning continue.

### Track B: Close the highest-value earlier-phase incompletions

4. Replace the shadow stub in [pipeline.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/pipeline.rs) with the real shadow stage or explicitly defer it and remove completion claims.
5. Restore proper BSP/world frustum culling in [pipeline.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/render/pipeline.rs).
6. Replace bbox collision proxies with real AKM mesh triangles in [geometry.rs](/Users/rikihernandez/Downloads/asciicker-rust-port/engine-port/src/physics/geometry.rs).

### Track C: Finish the “final visual” work honestly

7. Treat the current six-samples shape-vector path as intermediate.
8. Add the missing directional/external-sample work or explicitly scope it as deferred with new acceptance criteria.
9. Wire `Font1` into real runtime systems instead of treating the resource API as feature completion.

## Recommendation

Do not spend the next session on another narrow water patch.

The best next milestone is:

**Renderer correctness closure before water polish**

Concretely:

1. `resolve.rs`
2. `sprite_blit.rs`
3. `water.rs` world reflections
4. `pipeline.rs` shadow/frustum cleanup

That ordering gives the best chance of turning water debugging into a real signal instead of another cycle of false regressions.
