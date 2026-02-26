---
title: "Decision Framework for Engine Port"
type: research
status: REFERENCE
date: 2026-02-18
# blocked_by: docs/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md (not applicable to Rust port)
---

> **Note:** This document was created during the C++ project's pipeline closeout phase. The blocking dependency on pipeline closeout is not applicable to the Rust port. Content remains valid as reference material.

# Decision Framework for Engine Port

## Status: REFERENCE (originally deferred in C++ project)

---

## Port Options

### Option A: Full Rewrite

**Description:** Port entire C++ codebase to Rust, module by module.

**Approach:**
1. Port render.cpp → rasterizer.rs (trait-based)
2. Port sprite.cpp → xp_loader.rs
3. Port world.cpp → world_bsp.rs
4. Port game.cpp → ECS-based game module

**Pros:**
- 100% safe Rust (no FFI)
- Modern tooling (cargo, clippy, rust-analyzer)
- Easier maintenance long-term
- No C++ ABI constraints

**Cons:**
- Highest effort (~22500 lines → ~15000 lines Rust)
- Long timeline (16-32 weeks)
- Risk of bugs during translation
- May lose optimizations

**Estimated Effort:** 16-32 weeks

---

### Option B: Hybrid with FFI

**Description:** Keep C++ core, wrap with Rust via FFI.

**Approach:**
1. Build C++ as shared library (.so/.dylib)
2. Create Rust FFI bindings (bindgen)
3. Implement game logic in Rust
4. Call into C++ for rendering

**Pros:**
- Leverage existing C++ optimization
- Faster initial development
- Proven code path

**Cons:**
- FFI overhead at render boundary
- Mixed build system (make + cargo)
- Harder debugging across FFI
- Safety guarantees lost at boundary
- macOS linking complexity

**Estimated Effort:** 8-16 weeks

---

### Option C: Mage Core Extension

**Description:** Extend Mage Core with missing features.

**Approach:**
1. Add CPU rasterizer to Mage Core
2. Add XP loader module
3. Add world BSP module
4. Keep Mage Core GPU path for UI

**Pros:**
- Builds on existing Rust codebase
- Contributes back to open source
- 100% safe Rust
- Shared maintenance burden

**Cons:**
- Mage Core may have different goals
- Upstream merge conflicts
- GPU-centric architecture mismatch
- Need to coordinate with maintainer

**Estimated Effort:** 12-24 weeks

---

### Option D: Minimal Port + GPU Acceleration

**Description:** Port only essential logic, use WGPU for rendering.

**Approach:**
1. Port XP loader to Rust
2. Port game logic to Rust
3. Use WGPU compute shaders for rasterization
4. Skip CPU rasterizer entirely

**Pros:**
- Leverages GPU (matches Mage Core approach)
- Potentially higher performance
- Modern rendering stack
- Smaller Rust codebase

**Cons:**
- Requires compute shader expertise
- May not match C++ output exactly
- GPU driver compatibility issues
- Harder to debug shader code

**Estimated Effort:** 12-20 weeks

---

## Constraints

### Hard Constraints

| Constraint | Impact |
|------------|--------|
| XP Format Compatibility | Must load existing .xp assets |
| CP437 Glyph Set | Must render all 256 glyphs |
| 256-Color Palette | Output must use xterm-256 |
| macOS Support | Must build on Apple Silicon |

### Soft Constraints

| Constraint | Impact |
|------------|--------|
| Performance | Target 60 FPS minimum |
| Memory | Reasonable footprint (<500MB) |
| Build Time | Fast iteration (<2 min) |
| Binary Size | Minimal overhead |

### Asset Constraints

| Asset | Format | Port Impact |
|-------|--------|-------------|
| Sprites | .xp (gzip REXPaint) | XP loader required |
| Fonts | .xp (font-1.xp) | Subset CP437 support |
| Meshes | .akm (PLY-based) | New module or skip |
| World | .a3d (binary) | New module or skip |

---

## Risk Matrix

| Risk | Option A | Option B | Option C | Option D |
|------|----------|----------|----------|----------|
| Performance regression | Medium | Low | Medium | Low |
| Integration complexity | Low | High | Medium | Medium |
| Maintenance burden | Low | High | Low | Medium |
| Timeline overrun | High | Medium | Medium | Medium |
| Platform issues | Low | High | Low | Medium |
| Skill gap | Medium | Low | Medium | High |

### Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Performance regression | Profile early, benchmark against C++ |
| Integration complexity | Incremental FFI boundaries |
| Maintenance burden | Document all decisions, modular design |
| Timeline overrun | Phased delivery with working checkpoints |
| Platform issues | CI on multiple platforms |
| Skill gap | Training, pair programming, reference impl |

---

## Decision Criteria

### Must Have (Mandatory)

- [ ] Load player-0100.xp successfully
- [ ] Render sprite to screen
- [ ] Build on macOS (Apple Silicon)
- [ ] 60 FPS minimum

### Should Have (Important)

- [ ] Match C++ visual output exactly
- [ ] All 256 CP437 glyphs
- [ ] Font skin recoloring
- [ ] Animation support

### Nice to Have (Optional)

- [ ] World BSP system
- [ ] Mesh rendering
- [ ] Networking
- [ ] Audio

---

## Recommendation

**Recommended approach (pipeline gate dependency removed for Rust port):**

### Recommended Approach: Option A (Full Rewrite)

**Rationale:**

1. **Long-term maintainability:** 100% safe Rust eliminates entire classes of bugs
2. **Tooling:** Cargo, clippy, rust-analyzer provide better DX than C++
3. **Evidence:** Mage Core proves Rust ASCII engines are viable
4. **Portability:** No FFI means easier cross-platform support

### Phase 1 Start

1. Create `mage-port/` crate structure
2. Implement `xp_loader.rs` (smallest critical path)
3. Render first sprite using Mage Core rendering
4. Benchmark, iterate

### Decision Checkpoints

| Checkpoint | Gate | Go/No-Go Criteria |
|------------|------|-------------------|
| Pipeline verification | N/A (Rust port) | N/A |
| Phase 1 start | P1-1 | Can render test scene |
| Phase 2 start | P2-1 | Can load player-0100.xp |
| Phase 3 start | P3-1 | Character renders on screen |

---

## References

- Closeout Plan: `docs/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md`
- Mage Core: `/Users/r/Projects/ascii research/Mage-core/`
- Asciicker Source: `/Users/r/Downloads/asciicker-Y9-2/`
- XP Format: `sprite.cpp:293-332`
