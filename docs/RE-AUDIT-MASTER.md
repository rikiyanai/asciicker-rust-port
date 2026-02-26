# Asciicker Rust Port - RE-AUDIT MASTER REPORT

## Executive Summary

This document consolidates findings from parallel re-audit agents that verified unknown unknowns against actual C++ source code. **42 unknowns have been resolved**, leaving **47 remaining unknowns** that require further investigation.

---

## VERIFIED / RESOLVED UNKNOWNS

### CRITICAL Priority (6 resolved)

| ID | Unknown | Finding | Status | Evidence |
|----|---------|---------|--------|----------|
| UU-001 | auto_mat format | 3 bytes: {bg, fg, glyph}, 32K entries | ✅ RESOLVED | render.cpp:708-840 |
| UU-002 | Perspective matrix | Uses 1/viewer_dist with view_ofs offset | ✅ RESOLVED | render.cpp:968-1008 |
| UU-003 | 2× DBL supersampling | `#define DBL` enables 2x2 samples | ✅ RESOLVED | render.cpp:88 |
| UU-004 | Depth test | `<=` comparison with HEIGHT_SCALE/2 bias | ✅ RESOLVED | render.cpp:587 |
| UU-005 | Column-major XPCell | Verified in terrain loading | ✅ RESOLVED | terrain.cpp |
| UU-006 | Plucker ray format | ray[0-2]=p×v confirmed | ✅ RESOLVED | world.cpp:2972-2980 |

### HIGH Priority (12 resolved)

| ID | Unknown | Finding | Status | Evidence |
|----|---------|---------|--------|----------|
| UU-007 | Visual averaging | INTEGER math with bit-shift rounding | ✅ RESOLVED | render.cpp:3493 |
| UU-008 | RGB555 packing | bits[14:10]=B (blue), NOT red | ✅ RESOLVED | render.cpp:3252 |
| UU-009 | Spare flags bit2/3 | bit2=grid, bit3=mesh confirmed | ✅ RESOLVED | render.cpp:565-566 |
| UU-010 | Coordinate system | Z-up (not Y-up) | ✅ RESOLVED | world.cpp |
| UU-011 | Shadow projection | Inverse transform, radius ~2.0 units | ✅ RESOLVED | render.cpp:3184-3263 |
| UU-012 | Neighbor flag mapping | 8-bit flag: bits 0-7 = 8 directions | ✅ RESOLVED | terrain.cpp:38-48 |
| UU-013 | Height interpolation | Bilinear per triangle with diag flag | ✅ RESOLVED | terrain.cpp:1630-1691 |
| UU-014 | 6D vector structure | 6 sampling points, 95 characters | ✅ RESOLVED | Alex Harri default.json |
| UU-015 | Sampling circle radius | 0.28125 (normalized) | ✅ RESOLVED | Alex Harri config |
| UU-016 | External point positions | 10 points, coords -0.25 to 1.25 | ✅ RESOLVED | Alex Harri JSON |
| UU-017 | Crunch exponent | global=3, directional=7 | ✅ RESOLVED | Alex Harri TS |
| UU-018 | WebGL shaders | Found in shaders.ts (341 lines) | ✅ RESOLVED | Alex Harri repo |

### MEDIUM Priority (15 resolved)

| ID | Unknown | Finding | Status | Evidence |
|----|---------|---------|--------|----------|
| UU-019 | Shadow radius | 2.0 world units | ✅ RESOLVED | render.cpp:3232 |
| UU-020 | Shadow intensity | 180-255 range | ✅ RESOLVED | render.cpp:3235-3238 |
| UU-021 | Diffuse averaging | Integer: (dif+34)/68 | ✅ RESOLVED | render.cpp:3493 |
| UU-022 | RGB conversion | `*527+23>>6` pattern | ✅ RESOLVED | render.cpp:3528-3542 |
| UU-023 | Tap3x3 boundary | Uses `>` vs `>=` inconsistency | ✅ RESOLVED | terrain.cpp:470-516 |
| UU-024 | Glyph dither set | " ..::%" (6 chars) | ✅ RESOLVED | render.cpp:36 |
| UU-025 | SampleBuffer border | +4 provides 1-sample border | ✅ RESOLVED | render.cpp:596-600 |
| UU-026 | Font aspect | 1.0 × 1.333 (4:3) | ✅ RESOLVED | Alex Harri metadata |
| UU-027 | Luma weights | Rec.709: 0.2126, 0.7152, 0.0722 | ✅ RESOLVED | Alex Harri shaders |
| UU-028 | Vogel spiral | Golden angle: 3.883... | ✅ RESOLVED | Alex Harri code |
| UU-029 | Material mode shadow | Fetches from material library | ✅ RESOLVED | render.cpp:3218-3228 |
| UU-030 | RGB mode shadow | Multiplies diffuse | ✅ RESOLVED | render.cpp:3211-3215 |
| UU-031 | Height scale bias | +HEIGHT_SCALE/2 z-fighting prevention | ✅ RESOLVED | render.cpp:587 |
| UU-032 | view_ofs calculation | dw/2 + shift[0]*2, dh/2 + shift[1]*2 | ✅ RESOLVED | render.cpp:110 |
| UU-033 | Perspective cull | viewer_dist <= 0 returns/culls | ✅ RESOLVED | render.cpp:82-101 |

### LOW Priority (9 resolved)

| ID | Unknown | Finding | Status | Evidence |
|----|---------|---------|--------|----------|
| UU-034 | auto_mat index | 3*(r + 32*(g + 32*b)) | ✅ RESOLVED | render.cpp:810 |
| UU-035 | 32K entries | 32×32×32 = 32768 entries | ✅ RESOLVED | render.cpp:708 |
| UU-036 | BG/FG/glyph order | {bg, fg, glyph} per entry | ✅ RESOLVED | render.cpp:34-36 |
| UU-037 | Space char index | Index 0 is space " " | ✅ RESOLVED | Alex Harri JSON |
| UU-038 | Vector range | ~0.0 to ~0.36 | ✅ RESOLVED | Alex Harri JSON |
| UU-039 | Alternative alphabets | 2-sample, 6-sample variants exist | ✅ RESOLVED | Alex Harri files |
| UU-040 | focal length | Found in renderer struct | ✅ RESOLVED | render.cpp:694 |
| UU-041 | view_pos/view_dir | Camera vectors in renderer | ✅ RESOLVED | render.cpp |
| UU-042 | DBL usage | #ifdef DBL blocks throughout | ✅ RESOLVED | render.cpp:2854+ |

---

## REMAINING UNKNOWNS (47)

### Rendering - Still Unknown

| ID | Unknown | Priority | Next Step |
|----|---------|----------|-----------|
| R-001 | SAH cost threshold | **GAP** | Not used - quadtree uses "grow upward" not SAH |
| R-002 | Perspective exact matrix values | HIGH | Need focal_default and specific FOV |
| R-003 | HEIGHT_SCALE constant value | MEDIUM | Need to find in terrain.h |
| R-004 | Diag bit → cell mapping | MEDIUM | Test diagonal patches empirically |

### Terrain - Still Unknown

| ID | Unknown | Priority | Next Step |
|----|---------|----------|-----------|
| T-001 | Exact patch expansion threshold | HIGH | Profile terrain growth |
| T-002 | Node merge/collapse logic | MEDIUM | Look for merge function |
| T-003 | .xp file format details | HIGH | Reverse engineer test files |

### Integration - Still Unknown

| ID | Unknown | Priority | Next Step |
|----|---------|----------|-----------|
| I-001 | Font used for alphabet generation | HIGH | Contact Alex Harri |
| I-002 | Performance benchmarks at 60fps | HIGH | Build and measure |
| I-003 | k-d tree construction parameters | MEDIUM | Find build config |
| I-004 | Cache quantization steps | MEDIUM | Analyze cache code |

### Serialization - Still Unknown

| ID | Unknown | Priority | Next Step |
|----|---------|----------|-----------|
| S-001 | .a3d version history | HIGH | Collect sample files |
| S-002 | Mesh reference resolution | HIGH | Test multi-mesh scenes |
| S-003 | URDO undo/redo format | MEDIUM | Reverse engineer saves |

### Audio - Still Unknown

| ID | Unknown | Priority | Next Step |
|----|---------|----------|-----------|
| A-001 | stb_vorbis error handling | MEDIUM | Read audio.cpp |
| A-002 | Sample unload logic | HIGH | Find cleanup code |
| A-003 | Marker format specifics | MEDIUM | Analyze audio.cpp:553 |

### Sprite - Still Unknown

| ID | Unknown | Priority | Next Step |
|----|---------|----------|-----------|
| SP-001 | XP layer semantics | HIGH | Read sprite.cpp:550-600 |
| SP-002 | Glyph coverage table origin | MEDIUM | Hardcode 256 values |
| SP-003 | Animation timing formula | MEDIUM | Find frame logic |

---

## GAPS (Not Unknowns - Just Missing)

| Gap | Type | Impact | Workaround |
|-----|------|--------|------------|
| WebGL shader source | **MISSING** | Can't optimize | Found in Alex Harri repo! ✅ |
| Original font files | **MISSING** | Can't regenerate vectors | Contact author |
| Binary benchmarks | **MISSING** | Can't validate performance | Build and measure |
| Original .a3d files | **MISSING** | Can't test versions | Collect samples |

---

## BUGS CONFIRMED

### From Original Audit - CONFIRMED

| File | Line | Bug | Fix |
|------|------|-----|-----|
| terrain.cpp | 613 | `if(x)` twice, should be `if(y)` | Change to `if(y)` |
| terrain.cpp | 805 | `u < y` where y out of scope | Change to `u < v` |
| terrain.cpp | 480,492 | `>` vs `>=` boundary inconsistency | Verify intent |

### New Bugs Found

| File | Line | Bug | Status |
|------|------|-----|--------|
| terrain.cpp | 1671 | `if (u < y)` comparing wrong variable | CONFIRMED |
| terrain.cpp | 465-466 | Debug leftover `int a = 0;` | NEW |

---

## REVISED AUDIT STATISTICS

| Category | Original | Resolved | Remaining | Notes |
|----------|----------|----------|-----------|-------|
| **CRITICAL** | 6 | 6 | 0 | All resolved! |
| **HIGH** | 25 | 12 | 13 | More progress possible |
| **MEDIUM** | 30 | 15 | 15 | Some gaps are not unknowns |
| **LOW** | 28 | 9 | 19 | Many inferred unknowns |
| **TOTAL** | **89** | **42** | **47** | 47% resolved |

---

## FILES CREATED BY RE-AUDIT

1. `docs/audit-reaudit-critical.md` - Critical unknowns verified
2. `docs/audit-reaudit-high-visual.md` - Visual pipeline verified  
3. `docs/audit-reaudit-terrain.md` - Terrain system verified
4. `docs/audit-reaudit-alexharri.md` - Alex Harri tech verified

---

## RECOMMENDED NEXT STEPS

1. **Resolve remaining 13 HIGH priorities** - Focus on matrix values, .xp format, k-d tree params
2. **Fix terrain.cpp bugs** - Lines 613, 805, 1671 before porting
3. **Obtain original font** - Contact Alex Harri for font used
4. **Benchmark current C++** - Establish baseline performance

---

*Report generated: 2026-02-19*
*Source: Parallel re-audit agents*
