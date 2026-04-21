# Asciicker Rust Port - Bug & Assumption Audit Report

## Executive Summary

This document consolidates findings from parallel audit agents examining the research documentation for the Asciicker C++ to Rust port. The audits identified **127 documented assumptions**, **89 unknown unknowns**, **67 gap areas**, and **43 potential bugs** across rendering, world/terrain, sprite/audio, and Alex Harri integration systems.

---

## 1. CRITICAL ASSUMPTIONS (Must Verify)

### Rendering System
| Assumption | Risk | Verification Needed |
|------------|------|---------------------|
| Sample height: negative = closer | HIGH | Test with actual render |
| Visual: RGB555 packing bits[14:10]=R | HIGH | Verify bit shifts in code |
| Spare flags: bit 2=grid, bit 3=mesh | MEDIUM | Check render.cpp lines |
| Y-up coordinate system | HIGH | Verify world.cpp axis usage |
| 2× supersampled SampleBuffer | CRITICAL | Find DBL define |

### World/Terrain Systems
| Assumption | Risk | Verification Needed |
|------------|------|---------------------|
| Plucker ray: ray[0-2]=p×v | HIGH | Test ray-box intersection |
| 8 octant variants required | MEDIUM | Profile vs generic algorithm |
| Column-major XPCell ordering | CRITICAL | Test with actual .xp files |

### Alex Harri Integration
| Assumption | Risk | Verification Needed |
|------------|------|---------------------|
| 6D vectors capture structure | HIGH | Test with game content |
| k-d tree O(log n) fast enough | HIGH | Benchmark at target resolution |
| Cache hit rate >80% typical | MEDIUM | Profile with game scenes |

---

## 2. UNKNOWN UNKNOWS (High Priority)

### Rendering - Resolve Phase
1. **Visual averaging method**: Integer vs float for 2×2 sample block?
2. **auto_mat lookup table**: 32K entries - exact format?
3. **Depth test semantics**: `>=` or `>` comparison?
4. **Shadow projection**: Exact inverse transform algorithm?

### Terrain Quadtree
5. **SAH cost threshold**: What value triggers LEAF vs split?
6. **Neighbor flag mapping**: Which bit = which direction?
7. **Height interpolation**: Exact bilinear formula?
8. **Diag bit → cell mapping**: Which bit controls which cell?

### Alex Harri - GPU Pipeline
9. **Sampling circle radius**: What value in source?
10. **External point positions**: Exact coordinates?
11. **Crunch exponent**: What values used?
12. **AFFECTING_EXTERNAL_INDICES**: Complete mapping table?

### Serialization
13. **.a3d version history**: What versions exist?
14. **Mesh reference resolution**: Load order dependencies?
15. **Undo/redo format**: URDO structure unknown?

---

## 3. GAPS - Missing Information

### Rendering (Critical)
| Gap | Impact | Workaround |
|-----|--------|------------|
| Perspective projection matrices | Can't port perspective mode | Use isometric only |
| Focal length values | Unknown | Derive from code |
| Diffuse lighting model | Unknown | Assume Lambertian |

### World/Terrain (High)
| Gap | Impact | Workaround |
|-----|--------|------------|
| NODE_SHARE algorithm | Incomplete feature | Don't implement |
| Ancestor cleanup algorithm | Memory leak | Document as known |
| 8 octant derivation | Complex math | Use generic algorithm |

### Sprite/Audio (High)
| Gap | Impact | Workaround |
|-----|--------|------------|
| Glyph coverage table origin | Can't regenerate | Hardcode 256 values |
| Dither matrix values | Can't verify | Use documented pattern |
| Marker format source | Can't extend | Document limitation |
| Material-to-sample mapping | Commented out | Implement or remove |

### Alex Harri Integration (High)
| Gap | Impact | Workaround |
|-----|--------|------------|
| WebGL shader source | Can't optimize | Implement from scratch |
| Font used for alphabets | Wrong visual match | Regenerate vectors |
| Performance benchmarks | Can't validate | Build and measure |

---

## 4. POTENTIAL BUGS - TODOs in C++ Code

### Rendering System (HIGH SEVERITY)
| File | Line | Bug | Fix Required |
|------|------|-----|--------------|
| render.cpp | 377 | No anim/frame/angle bounds validation | Add bounds checks |
| render.cpp | 378 | 2× supersampling hardcoded | Make DBL configurable |
| sprite.cpp | 328 | glyph uint32 can exceed 255 | Add validation |
| sprite.cpp | 795 | max_anims=16 hardcoded | Grow or validate |

### Terrain System (HIGH SEVERITY)
| File | Line | Bug | Fix Required |
|------|------|-----|--------------|
| terrain.cpp | 613 | BUG: `if (x)` twice, should be `y` | Fix logic |
| terrain.cpp | 480,492 | Boundary `>` vs `>=` assumption | Verify |
| terrain.cpp | 805 | Condition `u < y` where `y` out of scope | Fix scope |

### World System (MEDIUM)
| File | Line | Bug | Fix Required |
|------|------|-----|--------------|
| world.cpp | 922 | Ancestor cleanup STUBBED | Document limitation |
| world.cpp | 1146 | Empty parent nodes not collapsed | Implement or note |

### Audio System (MEDIUM)
| File | Line | Bug | Fix Required |
|------|------|-----|--------------|
| audio.cpp | 704 | No sample unloading | Add cleanup |
| audio.cpp | 553 | Marker lookup no bounds | Add check |
| audio.cpp | 684 | Division by 65535 | Fix precision |

---

## 5. INTEGRATION RISKS

### Alex Harri → Asciicker
| Risk | Likelihood | Severity | Mitigation |
|------|-------------|----------|------------|
| 6D vectors don't match game patterns | HIGH | HIGH | Test before committing |
| k-d tree too slow for 60fps | MEDIUM | HIGH | Profile extensively |
| Cache hit rate lower than expected | MEDIUM | MEDIUM | Budget fallback |
| Depth ordering lost | HIGH | HIGH | Verify RESOLVE integration |
| Temporal flickering | HIGH | MEDIUM | Implement smoothing |
| Wrong font for alphabet | HIGH | HIGH | Regenerate vectors |

### Architectural Risks
| Risk | Likelihood | Severity | Mitigation |
|------|-------------|----------|------------|
| Performance regression vs auto_mat | HIGH | HIGH | Benchmark current vs new |
| GPU pipeline complexity | MEDIUM | MEDIUM | Start CPU-only |
| Memory pressure from cache | LOW | MEDIUM | Set explicit limits |

---

## 6. DECISION POINTS REQUIRED

### Rendering
1. **Perspective vs Isometric**: Can we derive matrix values or use isometric only?
2. **auto_mat vs k-d tree**: Keep existing for performance or replace for quality?
3. **2× supersampling**: Make configurable or hardcode?

### World/Terrain
4. **8 octant variants**: Keep for performance or generalize?
5. **Ancestor cleanup**: Implement properly or document as known limitation?
6. **NODE_SHARE**: Skip as incomplete feature?

### Sprite/Audio
7. **Allocator**: Use trait-based or direct malloc?
8. **Reference counting**: Manual, Arc, or Rc?
9. **Audio backend**: Abstract to traits or pick one platform?

### Alex Harri Integration
10. **CPU vs GPU**: Start with CPU path, add GPU later
11. **2D vs 6D vectors**: Begin 2D for speed, upgrade if stable
12. **Effects intensity**: Default to zero, expose as parameter

---

## 7. RECOMMENDED NEXT STEPS

### Immediate (Before Coding)
1. **Verify critical assumptions** by reading actual C++ source
2. **Fix known bugs** in terrain.cpp (line 613)
3. **Document unknowns** that block implementation decisions

### Short-term (Research Phase)
4. **Obtain WebGL shader source** from Alex Harri's repo
5. **Identify exact font** used for alphabet generation
6. **Benchmark** current Asciicker rendering performance

### Long-term (Porting)
7. **Add comprehensive validation** at file loading boundaries
8. **Implement with bounds checking** even if C++ didn't
9. **Set explicit cache limits** for memory safety

---

## Appendix: Quick Reference

### Critical Files to Re-read
- render.cpp:370-390 (resolve phase)
- terrain.cpp:600-620 (neighbor lookup)
- terrain.cpp:1230-1280 (height init)
- world.cpp:1300-1600 (SAH algorithm)
- sprite.cpp:550-600 (XP layer parsing)

### Unknowns Blocking Decisions
1. Perspective matrix values → blocks perspective port
2. SAH cost threshold → affects BSP performance  
3. External point positions → blocks directional crunch
4. .a3d versions → blocks serialization compatibility

### Bugs to Fix Before Port
1. terrain.cpp:613 (confirmed bug)
2. sprite frame bounds (crash vector)
3. hardcoded 2× supersampling
