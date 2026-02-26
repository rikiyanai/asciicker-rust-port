# Technical Audits: Phase 1 & 2 - Complete Analysis

This document consolidates the technical audits for Implementation Phases 1 and 2, with detailed task breakdowns, assumptions, gaps, gotchas, and sequential task lists.

---

# PHASE 1: FOUNDATION (Weeks 1-3)

## Milestone 1.1: Project Setup (Week 1)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 1.1.1 | Initialize Bevy project with Cargo | None | `cargo new asciicker` succeeds |
| 1.1.2 | Configure Cargo.toml with dependencies (bevy 0.18.0, bevy_kira_audio, serde, flate2) | 1.1.1 | No version conflicts |
| 1.1.3 | Set up project structure (components/, systems/, rendering/, loaders/, assets/) | 1.1.2 | All directories created |
| 1.1.4 | Create main.rs with Bevy app init | 1.1.3 | App builds |
| 1.1.5 | Configure window (80x24 chars * font size) | 1.1.4 | Window config matches ASCII |
| 1.1.6 | Set up logging (tracing) | 1.1.4 | Logs output to console |
| 1.1.7 | Create build verification (cargo check passes) | 1.1.5 | Clean build |
| 1.1.8 | Test window opens and runs | 1.1.7 | No crashes |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| A1 | Bevy 0.18+ stable | **VERIFY** | Medium |
| A4 | Rust 1.80+ features | Valid | Low |

### Gaps

- No CI/CD defined
- No logging/tracing config in plan
- No error handling strategy
- Platform-specific build config missing

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Bevy compile times (10-30 min) | Timeline | Use `cargo check` |
| Default window size wrong | Visual | Explicit config |
| WGPU backend fails | Runtime | Explicit backend preference |
| Dependency conflicts | Build | Use `cargo tree` |

---

## Milestone 1.2: ASCII Buffer System (Week 2)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 1.2.1 | Define ASCII buffer struct (CPU-side) | M1.1 complete | Struct defined |
| 1.2.2 | Create fg_texture (RGBA8UnormSrgb) | 1.2.1 | Texture created |
| 1.2.3 | Create bg_texture (RGBA8UnormSrgb) | 1.2.2 | Texture created |
| 1.2.4 | Create chars_texture (R8Unorm) | 1.2.3 | Texture created |
| 1.2.5 | Configure RenderAssetUsages (CPU-updatable) | 1.2.4 | Textures update each frame |
| 1.2.6 | Implement font atlas loader from PNG | 1.2.4 | Atlas loads |
| 1.2.7 | Implement char-to-UV mapping (16x16 grid) | 1.2.6 | Shader can sample |
| 1.2.8 | Create render target texture | 1.2.5 | Output target ready |
| 1.2.9 | Write WGSL vertex shader (full-screen quad) | 1.2.8 | Shader compiles |
| 1.2.10 | Write WGSL fragment shader (threshold rendering) | 1.2.9 | Characters render |
| 1.2.11 | Create AsciiMaterial with AsBindGroup | 1.2.10 | Bind groups work |
| 1.2.12 | Test basic output | 1.2.11 | Window shows ASCII |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| A2 | WGPU backend works | **VERIFY** | Medium-High |
| D3 | Glyph coverage table complete | **UNVERIFIED** | High |

### Gaps

- Buffer update mechanism not specified
- Character-to-UV mapping algorithm not detailed
- Color format conversion (RGB555→RGBA) not specified
- Alpha/blending config not specified
- Font size config not specified

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Texture format mismatch | Runtime error | Match formats exactly |
| UV calculations off-by-one | Wrong char display | Test with known chars |
| Sampler wrong (linear vs nearest) | Blurry chars | Use nearest filter |
| R8Unorm sampling in WGSL | Type error | Use `.r` accessor |

---

## Milestone 1.3: Triangle Rasterizer (Week 3)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 1.3.1 | Define triangle struct (3 vertices + color) | M1.2 complete | Struct defined |
| 1.3.2 | Implement edge function algorithm | 1.3.1 | Correct inside/outside |
| 1.3.3 | Implement scanline conversion | 1.3.2 | Correct pixel coverage |
| 1.3.4 | Implement pixel buffer writing | 1.3.3 | Buffer updates |
| 1.3.5 | Implement triangle clipping (boundary) | 1.3.4 | No crashes OOB |
| 1.3.6 | Create depth buffer struct | 1.3.1 | Buffer created |
| 1.3.7 | Implement depth test (less-than-or-equal) | 1.3.6 | Closer overwrites |
| 1.3.8 | Implement depth write | 1.3.7 | Depth updates |
| 1.3.9 | Clear depth buffer (-1000000.0f) between frames | 1.3.8 | Buffer clears |
| 1.3.10 | Implement RGB555 pack function | 1.3.1 | Bits packed correctly |
| 1.3.11 | Implement RGB555 unpack function | 1.3.10 | Bits unpacked |
| 1.3.12 | Define camera struct (pos, yaw, zoom) | 1.3.1 | Struct defined |
| 1.3.13 | Implement view matrix construction | 1.3.12 | Matrix correct |
| 1.3.14 | Implement perspective projection (focal) | 1.3.13 | Perspective works |
| 1.3.15 | Implement viewport transformation | 1.3.14 | NDC→screen |
| 1.3.16 | Test triangle renders correctly | 1.3.5, 1.3.9, 1.3.11, 1.3.15 | Output matches expected |
| 1.3.17 | Test depth test works | 1.3.16 | Front hides back |
| 1.3.18 | Compare output to C++ reference | 1.3.17 | Visual match |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| A5 | C++ rendering replicable | **LIKELY** | Medium |
| D2 | Perspective math complete | **UNVERIFIED** | High |

### Gaps

- Coordinate system (left/right-handed, Y-up) not specified
- Depth buffer precision not specified
- Triangle fill rule not specified
- Clipping algorithm not specified
- No test data defined

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Depth precision causes z-fighting | Visual flicker | Use f32 |
| Perspective divide wrong | Distorted geometry | Test with cube |
| Integer overflow in rasterization | Wrong pixels | Use i64 |
| Viewport off-by-one | Offset display | Use standard formula |
| Color interpolation in wrong space | Wrong colors | Linear space interpolation |

---

## PHASE 2: RENDERING PIPELINE (Weeks 4-8)

## Milestone 2.1: 6-Stage Pipeline (Weeks 4-5)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 2.1.1 | Define pipeline stages enum | M1.3 complete | Stages defined |
| 2.1.2 | Implement CLEAR stage (clear all SampleBuffer fields) | 2.1.1 | Buffer cleared |
| 2.1.3 | Implement TERRAIN stage (height query) | 2.1.2 | Terrain renders |
| 2.1.4 | Implement TERRAIN stage (visual/material mapping) | 2.1.3 | Materials show |
| 2.1.5 | Implement TERRAIN stage (diagonal handling) | 2.1.4 | Correct diagonals |
| 2.1.6 | Implement WORLD stage (mesh query) | 2.1.5 | Meshes render |
| 2.1.7 | Implement WORLD stage (sprite queue) | 2.1.6 | Sprites queued |
| 2.1.8 | Implement SHADOW stage (projection matrix) | 2.1.7 | Shadows project |
| 2.1.9 | Implement SHADOW stage (inverse transform) | 2.1.8 | Height tested |
| 2.1.10 | Implement REFLECTION stage (conditional) | 2.1.9 | Reflections work |
| 2.1.11 | Implement RESOLVE stage (2x2 downsample) | 2.1.10 | Samples averaged |
| 2.1.12 | Implement RESOLVE stage (min depth) | 2.1.11 | Correct occlusion |
| 2.1.13 | Implement SPRITES stage (sort far-to-near) | 2.1.12 | Correct order |
| 2.1.14 | Implement SPRITES stage (depth test) | 2.1.13 | Sprites render |
| 2.1.15 | Orchestrate pipeline (stage order) | 2.1.14 | All stages run |
| 2.1.16 | Verify full pipeline output | 2.1.15 | Complete frame |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| A5 | C++ rendering replicable | **VERIFY** | Medium |
| D2 | Perspective math complete | From M1.3 | High |

### Gaps

- Edge function derivation not detailed
- BC_P pixel center sampling not detailed
- Double-sided rendering logic not detailed
- Shadow projection matrix algorithm not documented
- SampleBuffer row-major vs column-major not verified

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Depth test uses negative = closer | Wrong occlusion | Use `<=` test |
| Row-major vs column-major | Wrong data layout | Verify from C++ |
| Perspective-correct interpolation formula | Wrong UVs | Use 1/W formula |
| Sprite sort stability | Flickering | Stable sort |
| Spare flag bits lost | Missing features | Preserve bits |

---

## Milestone 2.2: auto_mat Lookup (Week 6)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 2.2.1 | Implement RGB555→RGB888 expansion (r<<3,g<<3,b<<3) | M2.1 complete | Colors expand |
| 2.2.2 | Implement RGB888→xterm256 cube mapping | 2.2.1 | xterm colors |
| 2.2.3 | Implement xterm256 full palette (0-255) | 2.2.2 | All 256 work |
| 2.2.4 | Allocate 32K-entry auto_mat table | 2.2.3 | Table allocated |
| 2.2.5 | Implement diffuse→11-level mapping | 2.2.4 | Levels correct |
| 2.2.6 | Implement dither matrix (4x4 Bayer) | 2.2.5 | Dither works |
| 2.2.7 | Implement glyph selection (" ..::%%") | 2.2.6 | Glyphs correct |
| 2.2.8 | Generate complete auto_mat table | 2.2.7 | Table full |
| 2.2.9 | Integrate with RESOLVE stage | 2.2.8 | Colors/glyphs output |
| 2.2.10 | Test color quantization output | 2.2.9 | Visual match |
| 2.2.11 | Test shading gradients | 2.2.10 | Smooth gradients |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| D4 | auto_mat format complete | Valid | Low |

### Gaps

- RGB555→RGB888 optimization formula not detailed
- auto_mat generation algorithm not fully documented

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Diffuse scale wrong (11 levels) | Wrong shading | diffuse/25.5 |
| Glyph string order wrong | Wrong dither | " ..::%%" lightest→densest |
| Table size wrong (98KB) | Memory issues | 32768 * 3 bytes |
| Cache unfriendly | Slow | Linear access pattern |

---

## Milestone 2.3: Sprite System (Weeks 7-8)

### Tasks (Sequential)

| # | Task | Dependencies | Success Criteria |
|---|------|-------------|-----------------|
| 2.3.1 | Implement gzip decompression (flate2) | M2.1 complete | Files decompress |
| 2.3.2 | Parse REXPaint header (version, layers, w, h) | 2.3.1 | Header parsed |
| 2.3.3 | Parse XPCell (10 bytes: glyph, fg, bg) | 2.3.2 | Cells parsed |
| 2.3.4 | Handle column-major ordering (NOT row-major!) | 2.3.3 | Correct layout |
| 2.3.5 | Implement layer semantics (L0=key, L1=height, L2=visual) | 2.3.4 | Layers correct |
| 2.3.6 | Implement height encoding ('0'-'9','A'-'Z'=0-35) | 2.3.5 | Height encoded |
| 2.3.7 | Extract sprite atlas texture | 2.3.6 | Atlas created |
| 2.3.8 | Implement screen-space billboard | 2.3.7 | Billboards face |
| 2.3.9 | Implement world-space billboard | 2.3.8 | Y-axis preserved |
| 2.3.10 | Implement animation state (frame, timing) | 2.3.9 | Anim advances |
| 2.3.11 | Implement animation sequences | 2.310 | Frames sequence |
| 2.3.12 | Integrate with SPRITES stage | 2.3.11 | Sprites render |
| 2.3.13 | Implement sprite depth sorting | 2.3.12 | Correct order |
| 2.3.14 | Test sprite loading | 2.3.13 | .xp loads |
| 2.3.15 | Test sprite animation | 2.3.14 | Anim plays |
| 2.3.16 | Compare to C++ reference | 2.3.15 | Visual match |

### Assumptions

| ID | Assumption | Status | Risk |
|----|------------|--------|------|
| D1 | .xp format reverse-engineered | Partial | Medium |

### Gaps

- Animation timing details not documented
- Swoosh overlay handling not detailed
- Glyph coverage table (256 entries) deferred

### Gotchas

| Gotcha | Impact | Mitigation |
|--------|--------|------------|
| Column-major ordering wrong | Corrupted sprites | Test with known data |
| GZIP vs ZLIB (use raw DEFLATE) | Parse fails | Configure flate2 correctly |
| Layer 0 vs Layer 1 confusion | Wrong data | Verify semantics |
| Animation timing off | Wrong speed | Research timing |

---

## GATE SUMMARY

| Gate | Must Complete | Enables |
|------|---------------|---------|
| 0 | - | M1.1 |
| 1 | M1.1 | M1.2 |
| 2 | M1.2 | M1.3 |
| 3 | M1.3 | M2.1 |
| 4 | M2.1 | M2.2, M2.3 |
| 5 | M2.2 | M3.1 |
| 6 | M2.3 | M3.1 |

---

## CRITICAL DATA DEPENDENCIES

| Dependency | Status | Location |
|------------|--------|----------|
| Perspective math (focal, view_pos, view_dir) | **UNVERIFIED** | Need from C++ |
| Glyph coverage table (256 entries) | **UNVERIFIED** | Need from C++ |
| auto_mat generation algorithm | **PARTIAL** | Need algorithm |
| Shadow projection matrix | **NOT DOCUMENTED** | Need research |
| Animation timing | **NOT DOCUMENTED** | Need research |

---

## VERIFICATION CHECKLIST BEFORE EACH PHASE

### Before Phase 1
- [ ] Verify Bevy 0.18.0 compiles
- [ ] Verify WGPU works on target platforms
- [ ] Verify Rust 1.80+ available
- [ ] Locate perspective math in C++ source
- [ ] Locate glyph coverage table in C++ source

### Before Phase 2
- [ ] Verify C++ rendering matches golden file
- [ ] Verify auto_mat table format
- [ ] Verify .xp file format details
- [ ] Verify shadow projection algorithm
- [ ] Verify animation timing

---

*Technical Audit Complete: 2026-02-20*
