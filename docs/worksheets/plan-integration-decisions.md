> **STATUS: PARTIALLY SUPERSEDED** — Section 1 (Alex Harri k-d tree) recommended integrating k-d tree first, but DECISION_LOG D010 (2026-02-19, Final) resolved: "Keep auto_mat initially, add k-d tree later." The recommendation in Section 1 is overridden by D010. Section 2 (Perspective mode) is also resolved by D004/D005 (Final). See DECISION_LOG.md for authoritative decisions.

# Integration Decision Plan
## HIGH Severity Integration Gaps

**Date:** 2026-02-20  
**Status:** Decision Required

---

## Decision 1: Alex Harri vs auto_mat

### Question
Should we integrate the k-d tree shape-matching approach or keep the existing auto_mat glyph selection?

### Option A: Integrate k-d tree (Alex Harri)

| Pros | Cons |
|------|------|
| Dramatically improves edge definition and visual quality | Significant implementation effort (~4 phases) |
| Preserves structural detail in rendered content | Computational overhead - may exceed 16.67ms frame budget at high resolutions |
| Maintains depth-buffering semantics | Temporal coherence challenges - characters may flicker during camera movement |
| k-d tree provides O(log n) lookup complexity | Cache warming strategies needed for optimal performance |
| 6D vector approach captures directional structure | RGB555 to luminance conversion pipeline must be documented |
| Effects system (crunch) enhances edge contrast | Font consistency required between generation and runtime |
| Hybrid approach can preserve lighting information | Requires SIMD optimization for 60fps at high resolution |

### Option B: Keep existing auto_mat

| Pros | Cons |
|------|------|
| Proven, stable implementation | Loses visual quality improvements |
| O(1) lookup complexity | Limited edge definition |
| No pipeline rearchitecture needed | Stair-stepping on diagonals/curves |
| Lower memory bandwidth requirements | No structural awareness in character selection |
| Simpler to maintain and debug | Dithering adds noise rather than preserving structure |

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|-------------|
| Frame time exceeds budget | HIGH | HIGH | Implement adaptive resolution, start with 2D alphabet |
| Temporal flickering | HIGH | MEDIUM | Cache warming, temporal smoothing |
| Cache thrashing | MEDIUM | MEDIUM | LRU cache with bounded size |
| Integration complexity | HIGH | HIGH | Phased implementation with validation |
| Memory bandwidth | MEDIUM | MEDIUM | Optimize SampleBuffer access patterns |

### Recommendation: **INTEGRATE k-d tree (Option A)**

The visual quality improvement is significant and the phased approach manages complexity. Start with 2D simple alphabet, add 6D support later, and implement GPU acceleration when needed.

**Confidence:** HIGH  
**Priority:** P1 (blocks visual quality improvement)

---

## Decision 2: Perspective Mode

### Question
Should we derive values from the existing renderer or use isometric-only rendering?

### Option A: Derive values from existing perspective renderer

| Pros | Cons |
|------|------|
| Preserves existing game content | Complex sampling across distorted cells |
| Maintains compatibility with existing assets | Non-uniform sampling requires per-cell computation |
| Camera system works unchanged | Depth-to-sampling integration unclear |
| Faster time-to-market | Patch boundary discontinuities |
| Existing pipeline remains intact | Grid-line handling becomes complex |

### Option B: Isometric-only rendering

| Pros | Cons |
|------|------|
| Uniform, predictable cell structure | Requires converting all content to isometric |
| Simpler k-d tree sampling (regular grid) | May lose existing game content compatibility |
| Consistent aspect ratio handling | Camera system requires modification |
| Cleaner integration with Alex Harri | Potential asset conversion effort |
| Better edge detection (predictable neighbors) | Requires new rendering pipeline |

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|-------------|
| Perspective distortion artifacts | HIGH | HIGH | Hybrid approach - sample after projection |
| Resolution scaling complexity | HIGH | MEDIUM | Adaptive resolution with hysteresis |
| Patch boundary discontinuities | MEDIUM | MEDIUM | Sample after terrain resolution |
| Camera system changes required | MEDIUM | HIGH | Preserve existing camera, add isometric mode |

### Recommendation: **DERIVE VALUES (Option A)**

Preserves existing content and faster to implement. The sampling pipeline already operates on the resolved 2D buffer, so the perspective distortion is already applied. Focus on making the sampling work well with existing pipeline.

**Confidence:** MEDIUM  
**Priority:** P1 (can be validated incrementally)

---

## Decision 3: Editor vs Game Runtime

### Question
Should we port the editor or focus exclusively on game runtime rendering?

### Option A: Port editor

| Pros | Cons |
|------|------|
| Full authoring workflow | Significant additional effort |
| Real-time preview of shape-matching | Complex UI integration |
| WYSIWYG editing | Multiple rendering paths to maintain |
| Aligns with Bevy ECS patterns | Editor-specific optimizations needed |
| Enables new visual features | Testing surface doubles |

### Option B: Focus on game runtime

| Pros | Cons |
|------|------|
| Faster time-to-market | No visual editing |
| Single rendering pipeline | Manual iteration for visual tweaks |
| Smaller codebase | Debugging requires runtime inspection |
| Lower maintenance burden | No immediate feedback loop |
| Can add editor later | Content creation workflow gaps |

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|-------------|
| Visual quality issues without editor | HIGH | MEDIUM | Runtime debugging tools |
| Iteration speed slow | HIGH | MEDIUM | Configurable parameters at runtime |
| Content creation bottleneck | MEDIUM | LOW | External config files |
| Missing editor features | MEDIUM | LOW | Document editor requirements |

### Recommendation: **FOCUS ON GAME RUNTIME (Option B)**

The primary goal is playable game rendering. Editor can be a Phase 2+ effort. Runtime debugging tools and configurable parameters will mitigate the lack of editor.

**Confidence:** HIGH  
**Priority:** P2 (after core rendering working)

---

## Summary Decision Matrix

| Decision | Option | Confidence | Priority |
|----------|--------|------------|----------|
| Alex Harri vs auto_mat | Integrate k-d tree | HIGH | P1 |
| Perspective mode | Derive values | MEDIUM | P1 |
| Editor vs Runtime | Focus on runtime | HIGH | P2 |

---

## Implementation Priority

### Phase 1 (Immediate)
1. Port k-d tree and quantized cache to Rust
2. Implement 2D simple alphabet shape-matching
3. Connect to RESOLVE phase
4. Create benchmark baseline

### Phase 2
1. Add 6D vector support
2. Implement cache warming
3. Add temporal smoothing

### Phase 3
1. Implement effects (global/directional crunch)
2. Add GPU acceleration for sampling

### Phase 4 (if needed)
1. Editor port
2. Advanced features

---

## Open Questions Requiring Resolution

1. **RGB555 to luminance formula**: Document specific conversion (Rec. 709 luma weights confirmed in source)
2. **Benchmark baseline**: Must establish before integration
3. **Cache size limits**: Need bounding strategy for long play sessions
4. **Alphabet selection**: Default to 2D simple, upgrade as performance allows

---

*Document Version: 1.0*
*Generated: 2026-02-20*
