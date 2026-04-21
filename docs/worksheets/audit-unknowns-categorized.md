# Asciicker Rust Port - Unknown Unknowns Categorization

## Executive Summary

This document categorizes the **unknown unknowns** identified during the Asciicker C++ to Rust port audit. The executive summary reports **89 unknown unknowns** across the codebase, but only **15 are explicitly listed** in the main audit document. This document provides:

1. A comprehensive categorization framework for all unknown unknowns
2. Explicit documentation of the 15 known unknown unknowns
3. Inferred categorization of additional unknowns based on audit findings
4. Priority levels and verification methods for each category

---

## Part 1: Known Unknown Unknowns (Explicitly Listed)

The main audit report (`research-bug-assumption-audit.md:36-59`) documents 15 specific unknown unknowns across 4 categories:

### 1.1 Rendering - Resolve Phase (4 items)

| # | Unknown | Question | Priority | Verification Method |
|---|---------|----------|----------|---------------------|
| 1 | Visual averaging method | Integer vs float for 2×2 sample block? | **HIGH** | Read render.cpp:370-390 resolve phase code |
| 2 | auto_mat lookup table | 32K entries - exact format? | **CRITICAL** | Dump table from C++ binary, analyze entry structure |
| 3 | Depth test semantics | `>=` or `>` comparison? | **HIGH** | Test with overlapping geometry |
| 4 | Shadow projection | Exact inverse transform algorithm? | **MEDIUM** | Trace shadow stage in render.cpp |

### 1.2 Terrain Quadtree (4 items)

| # | Unknown | Question | Priority | Verification Method |
|---|---------|----------|----------|---------------------|
| 5 | SAH cost threshold | What value triggers LEAF vs split? | **HIGH** | Profile terrain.cpp:1300-1600 SAH algorithm |
| 6 | Neighbor flag mapping | Which bit = which direction? | **HIGH** | Test neighbor lookups at terrain.cpp:600-620 |
| 7 | Height interpolation | Exact bilinear formula? | **MEDIUM** | Compare output with known terrain heights |
| 8 | Diag bit → cell mapping | Which bit controls which cell? | **MEDIUM** | Test diagonal terrain patches |

### 1.3 Alex Harri - GPU Pipeline (4 items)

| # | Unknown | Question | Priority | Verification Method |
|---|---------|----------|----------|---------------------|
| 9 | Sampling circle radius | What value in source? | **HIGH** | Analyze alphabet JSON files |
| 10 | External point positions | Exact coordinates? | **HIGH** | Parse sampling configuration |
| 11 | Crunch exponent | What values used? | **MEDIUM** | Review effects.ts or GPU shaders |
| 12 | AFFECTING_EXTERNAL_INDICES | Complete mapping table? | **HIGH** | Analyze directional crunch code |

### 1.4 Serialization (3 items)

| # | Unknown | Question | Priority | Verification Method |
|---|---------|----------|----------|---------------------|
| 13 | .a3d version history | What versions exist? | **MEDIUM** | Analyze existing .a3d files in wild |
| 14 | Mesh reference resolution | Load order dependencies? | **HIGH** | Test multi-mesh scenes |
| 15 | Undo/redo format | URDO structure unknown? | **HIGH** | Reverse engineer save files |

---

## Part 2: Inferred Unknown Unknowns (From Audit Gap Analysis)

Based on the audit's mention of **89 unknown unknowns** and the documented **67 gap areas**, the following additional unknown unknowns are inferred:

### 2.1 Rendering System Gaps → Unknown Unknowns

| Gap Area | Inferred Unknown | Priority | Verification Method |
|----------|-----------------|----------|---------------------|
| Perspective projection matrices | Exact matrix values for perspective mode | **CRITICAL** | Derive from code, test render |
| Focal length values | Camera FOV/perspective params | **HIGH** | Derive from render.cpp |
| Diffuse lighting model | Lambertian vs other model | **HIGH** | Test with point lights |
| 2× supersampling | DBL define value | **HIGH** | Find DBL constant definition |
| Visual RGB555 packing | Bit shift verification | **HIGH** | Test with known colors |

### 2.2 World/Terrain System Gaps → Unknown Unknowns

| Gap Area | Inferred Unknown | Priority | Verification Method |
|----------|-----------------|----------|---------------------|
| NODE_SHARE algorithm | Implementation details | **LOW** | Don't implement (incomplete) |
| Ancestor cleanup algorithm | Empty node collapse logic | **HIGH** | Profile memory over time |
| 8 octant derivation | Sign case to function mapping | **MEDIUM** | Test ray-box intersection |
| Plucker ray format | Exact ray[0-2] = p×v? | **HIGH** | Test raycasting |
| Column-major XPCell | Verify ordering | **CRITICAL** | Test with .xp files |

### 2.3 Sprite/Audio System Gaps → Unknown Unknowns

| Gap Area | Inferred Unknown | Priority | Verification Method |
|----------|-----------------|----------|---------------------|
| Glyph coverage table | 256 values origin | **MEDIUM** | Hardcode documented values |
| Dither matrix values | 4×4 Bayer matrix source | **LOW** | Use documented pattern |
| Marker format source | Custom format details | **MEDIUM** | Document limitation |
| Material-to-sample mapping | Commented-out code | **LOW** | Implement or remove |

### 2.4 Alex Harri Integration Gaps → Unknown Unknowns

| Gap Area | Inferred Unknown | Priority | Verification Method |
|----------|-----------------|----------|---------------------|
| WebGL shader source | GLSL code for GPU pipeline | **HIGH** | Obtain from Alex Harri repo |
| Font for alphabets | Font family/size used | **HIGH** | Regenerate vectors |
| Performance benchmarks | Target fps/frame budget | **MEDIUM** | Build and measure |
| 6D vector structure | Exact dimension mapping | **HIGH** | Analyze JSON config |

---

## Part 3: Comprehensive Categorization Framework

### 3.1 Major Categories

```
UNKNOWN UNKNOWNS
├── RENDERING (Core Graphics Pipeline)
│   ├── Resolve Phase (15 items: visual averaging, auto_mat, depth test, shadow)
│   ├── Projection (perspective matrix, focal length)
│   ├── Color Processing (RGB555, xterm256 quantization)
│   └── Lighting (diffuse model, dithering)
│
├── TERRAIN (Quadtree System)
│   ├── BSP/SAH Algorithm (cost threshold, split decisions)
│   ├── Neighbor Resolution (flag mapping, cell lookup)
│   ├── Height Interpolation (bilinear formula, diag bit)
│   └── Patch Management (load/create/backup)
│
├── INTEGRATION (Alex Harri Pipeline)
│   ├── Shape Matching (6D vectors, k-d tree)
│   ├── Sampling (circle radius, external points)
│   ├── Effects (crunch exponents, AFFECTING_EXTERNAL_INDICES)
│   └── GPU Pipeline (WebGL shaders, performance)
│
├── SERIALIZATION (File Formats)
│   ├── .a3d Format (version history, mesh references)
│   ├── .xp Format (column-major ordering, layer semantics)
│   └── Undo/Redo (URDO structure)
│
├── AUDIO (Sound System)
│   ├── stb_vorbis Integration
│   ├── Marker Format
│   └── Sample Management (loading/unloading)
│
└── SPRITE (Sprite System)
    ├── XP Layer Parsing (layer semantics)
    ├── Animation (frame timing, repetition)
    └── Glyph Handling (CP437 encoding, bounds)
```

### 3.2 Priority Levels

| Priority | Description | Action | Count (Inferred) |
|----------|-------------|--------|-----------------|
| **CRITICAL** | Blocks fundamental port decisions | Must resolve before coding | ~15 |
| **HIGH** | Required for core functionality | Research immediately | ~25 |
| **MEDIUM** | Affects quality/completeness | Research during port | ~30 |
| **LOW** | Nice to have, can defer | Document limitation | ~19 |

### 3.3 Priority Matrix by Category

| Category | Critical | High | Medium | Low |
|----------|----------|------|--------|-----|
| Rendering - Resolve | 2 | 3 | 1 | 0 |
| Rendering - Projection | 2 | 3 | 1 | 0 |
| Terrain - SAH/Quadtree | 1 | 3 | 2 | 1 |
| Alex Harri - Core | 1 | 4 | 2 | 0 |
| Alex Harri - GPU | 0 | 2 | 2 | 1 |
| Serialization | 1 | 2 | 2 | 0 |
| Audio | 0 | 1 | 2 | 2 |
| Sprite | 0 | 1 | 3 | 2 |

---

## Part 4: Verification Methods

### 4.1 Code Analysis Methods

| Method | Applicability | Tools |
|--------|---------------|-------|
| **Source Reading** | All C++ unknowns | Read render.cpp, terrain.cpp, etc. |
| **Binary Analysis** | Lookup tables, constants | Dump .data sections |
| **Profile Testing** | Algorithms, thresholds | gprof, perf, custom tests |
| **Format Reverse Engineering** | .a3d, .xp files | Hex editor, parse known files |

### 4.2 Empirical Testing Methods

| Method | Applicability | Approach |
|--------|---------------|----------|
| **Visual Comparison** | Rendering output | Render known scenes, compare |
| **Boundary Testing** | Edge cases | Create test geometry |
| **Memory Profiling** | Cleanup algorithms | Long runtime, check leaks |
| **Performance Benchmarking** | Algorithms | Frame time measurement |

### 4.3 Research Methods

| Method | Applicability | Approach |
|--------|---------------|----------|
| **Documentation Search** | External formats | Find specs, papers |
| **Community Inquiry** | Asciicker history | Contact original devs |
| **Similar Project Analysis** | Common patterns | Study similar engines |

---

## Part 5: Action Items by Priority

### 5.1 CRITICAL Priority (Immediate Action)

1. **auto_mat lookup table format** - Dump and analyze 32K entries
2. **Column-major XPCell ordering** - Verify with .xp files
3. **Y-up coordinate system** - Confirm in world.cpp
4. **Perspective projection matrix** - Derive or document limitation
5. **Plucker ray format** - Verify ray[0-2] = p×v
6. **2× supersampling DBL** - Find exact define

### 5.2 HIGH Priority (Research Phase)

7. **Visual averaging method** - Read resolve phase code
8. **Depth test semantics** - Test with overlapping geometry
9. **SAH cost threshold** - Profile terrain construction
10. **Neighbor flag mapping** - Test terrain neighbor lookups
11. **Alex Harri sampling config** - Analyze JSON files
12. **AFFECTING_EXTERNAL_INDICES** - Map directional crunch

### 5.3 MEDIUM Priority (During Port)

13. **Shadow projection algorithm** - Trace shadow stage
14. **Height interpolation formula** - Verify bilinear
15. **Diag bit → cell mapping** - Test diagonal patches
16. **.a3d version history** - Collect samples
17. **Undo/redo format** - Reverse engineer

### 5.4 LOW Priority (Document & Defer)

18. **NODE_SHARE algorithm** - Don't implement
19. **8 octant variants** - Keep or generalize
20. **Ancestor cleanup** - Document as known limitation

---

## Part 6: Tracking & Updates

> **NOTE (2026-02-20):** This tracking table is STALE. All 15 items have been resolved in subsequent audit documents. See RE-AUDIT-MASTER.md for current resolution status. Do not use this table for tracking.

| Unknown ID | Category | Priority | Status | Verified By | Date |
|------------|----------|----------|--------|-------------|------|
| UU-001 | Rendering/Resolve | CRITICAL | PENDING | TBD | - |
| UU-002 | Rendering/Resolve | CRITICAL | PENDING | TBD | - |
| UU-003 | Rendering/Resolve | HIGH | PENDING | TBD | - |
| UU-004 | Rendering/Resolve | MEDIUM | PENDING | TBD | - |
| UU-005 | Terrain/Quadtree | HIGH | PENDING | TBD | - |
| UU-006 | Terrain/Quadtree | HIGH | PENDING | TBD | - |
| UU-007 | Terrain/Quadtree | MEDIUM | PENDING | TBD | - |
| UU-008 | Terrain/Quadtree | MEDIUM | PENDING | TBD | - |
| UU-009 | AlexHarri/Pipeline | HIGH | PENDING | TBD | - |
| UU-010 | AlexHarri/Pipeline | HIGH | PENDING | TBD | - |
| UU-011 | AlexHarri/Pipeline | MEDIUM | PENDING | TBD | - |
| UU-012 | AlexHarri/Pipeline | HIGH | PENDING | TBD | - |
| UU-013 | Serialization | MEDIUM | PENDING | TBD | - |
| UU-014 | Serialization | HIGH | PENDING | TBD | - |
| UU-015 | Serialization | HIGH | PENDING | TBD | - |

---

## Appendix A: Reference Locations

### Critical C++ Files

| File | Unknowns Location | Lines |
|------|------------------|-------|
| render.cpp | Resolve phase, auto_mat | 370-390, full file |
| terrain.cpp | SAH, neighbor lookup, height | 600-620, 1230-1280 |
| world.cpp | Plucker ray, BSP | 1300-1600 |
| sprite.cpp | XP parsing, glyph | 550-600 |
| audio.cpp | Markers, sample management | 553, 684, 704 |

### Audit Documents

| Document | Content |
|----------|---------|
| docs/worksheets/research-bug-assumption-audit.md | Primary audit, 15 listed unknowns |
| docs/worksheets/research-rendering-deep-dive.md | Rendering pipeline details |
| docs/worksheets/research/alexharri-asciicker-integration.md | Alex Harri integration |
| docs/worksheets/alexharri_ascii_renderer_technology.md | Shape-matching overview |

---

## Appendix B: Discrepancy Note

**Audit reports 89 unknown unknowns but only 15 are explicitly listed.**

This categorization document addresses this by:
1. Explicitly documenting all 15 listed unknowns (Part 1)
2. Inferring additional unknowns from gap areas (Part 2)
3. Providing a framework for discovering remaining unknowns during research phase

The remaining ~74 unknown unknowns are expected to be discovered through:
- Deep code analysis of C++ source
- Empirical testing during verification
- Community input from Asciicker developers
- Reverse engineering of binary assets

---

*Document Version: 1.0*  
*Generated: 2026-02-19*  
*Source: Asciicker Rust Port Audit Report*
