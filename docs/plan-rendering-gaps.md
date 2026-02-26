# Implementation Plan: MEDIUM Severity Rendering Gaps

**Date:** 2026-02-20  
**Status:** Implementation Decisions  
**Category:** Rendering  

---

## Executive Summary

This document provides implementation plans for five MEDIUM severity gaps identified in the rendering system analysis. Each gap represents a feature or optimization from the original C++ implementation that was not previously documented in the research materials. The plan addresses how each gap should be handled in the Rust port and provides clear recommendations on implementation timing.

The five gaps covered are: edge function mathematical derivation, BC_P pixel center sampling, double-sided rendering logic, RGB555 to RGB888 conversion optimization, and glyph coverage for alpha blending. After thorough analysis, four of these gaps are recommended for immediate implementation in the Rust port, while one is recommended for deferral until specifically needed.

---

## Gap 1: Edge Function Mathematical Derivation

### 1.1 Current State Analysis

The original C++ implementation contains detailed mathematical comments explaining the edge function algorithm used for triangle rasterization. The edge function computes the signed area of a parallelogram formed by two vectors, which determines which side of an edge a point lies on. This mathematical foundation is critical for understanding how the barycentric coordinate system works in the rasterizer.

The formula documented in render.cpp (lines 414-421) shows that the edge function e(a,b,c) equals (b.x - a.x)*(c.y - a.y) - (b.y - a.y)*(c.x - a.x). This computes the cross product of vectors (a->b) and (a->c), yielding a signed value that indicates the position of point c relative to the directed edge from a to b. The sign of this value determines whether the point is on the left or right side of the edge, which is fundamental to the inside/outside test in triangle rasterization.

### 1.2 Rust Port Implementation Approach

For the Rust port, this mathematical derivation should be captured as documentation within the rasterizer module rather than as executable code. The implementation should include inline comments explaining the mathematical significance of each computation. The actual edge function computation is straightforward to implement in Rust using the same formula, but the documentation explaining why this formula works is the primary gap to address.

The Rust implementation should create a constants module or dedicated documentation file that explains the edge function mathematics. This serves as a technical reference for future maintainers and ensures the reasoning behind the rasterization algorithm is preserved. The implementation should include the mathematical derivation in comments above the relevant function, following the pattern established in the original C++ code.

### 1.3 Recommendation

**IMPLEMENT IMMEDIATELY** - This gap requires documentation only, not new algorithmic implementation. The edge function computation itself is straightforward and will be ported as part of the rasterizer. The priority is MEDIUM because the mathematical reasoning provides important context for understanding and maintaining the rasterization code. Without this documentation, future developers may modify the edge function without understanding its mathematical foundations, potentially introducing subtle bugs.

---

## Gap 2: BC_P Pixel Center Sampling

### 2.1 Current State Analysis

The BC_P macro is a critical component of the rasterization algorithm that samples the edge function at the center of each pixel cell rather than at its corner. This design choice is explicitly made to avoid sampling bias that would cause triangles sharing an edge to either double-draw or miss boundary pixels entirely. The implementation uses 2*c+1 terms to compute the center position of cell c, where c is the cell index.

The importance of center sampling becomes clear when considering adjacent triangles that share an edge. If both triangles sampled at the corner of cells, floating-point imprecisions could cause one triangle to claim a pixel while its neighbor also claims the same pixel, resulting in double-draw artifacts. Alternatively, both triangles might miss the pixel entirely, creating gaps along shared edges. Center sampling provides a more robust boundary condition that naturally distributes shared pixels to exactly one triangle.

The source code in render.cpp (lines 426-431) explicitly documents this reasoning, making it clear that the 2*c+1 terms are not arbitrary but are specifically chosen to sample at cell centers. This is a subtle but important implementation detail that affects the visual quality of rendered output, particularly along triangle edges.

### 2.2 Rust Port Implementation Approach

The Rust implementation should replicate the center sampling behavior exactly, using integer arithmetic equivalent to the 2*c+1 computation. The module should include documentation explaining why center sampling is used and how it prevents the double-draw and gap artifacts that would occur with corner sampling. This documentation is essential for maintaining visual fidelity during the port.

The implementation should be structured as part of the rasterizer trait or struct, with clear comments explaining the coordinate transformation from cell indices to sample positions. The Rust code can use the same integer arithmetic approach, though care should be taken to ensure equivalent behavior on the Rust side. Performance testing should verify that the Rust implementation produces identical output to the C++ original.

### 2.3 Recommendation

**IMPLEMENT IMMEDIATELY** - This is a critical implementation detail that must be preserved for visual correctness. The priority is MEDIUM relative to other gaps because the effect is subtle (only visible along triangle edges) but the impact on visual quality is significant. Incorrect sampling would cause visible artifacts in rendered output, particularly noticeable along mesh boundaries and terrain triangle edges.

---

## Gap 3: Double-Sided Rendering Logic

### 3.1 Current State Analysis

The Rasterize function in the original implementation supports both single-sided and double-sided rendering modes, controlled by the dblsided parameter. When dblsided is true, the rasterizer handles triangles regardless of their winding order (counter-clockwise or clockwise). This is essential for rendering objects that are visible from both sides, such as walls, membranes, or transparent surfaces.

The implementation works by computing the triangle area to determine winding order. For counter-clockwise triangles (area greater than zero), the standard edge function test applies: a point is inside the triangle if all three edge function results are non-negative. For clockwise triangles (area less than zero), the test is inverted: a point is inside if all three edge function results are non-positive. This simple sign inversion allows the same rasterization logic to handle both winding orders.

The source code in render.cpp (lines 501-554) shows the conditional logic that branches based on the area sign. When dblsided is false, clockwise triangles are rejected entirely, implementing standard back-face culling. When dblsided is true, both winding orders are accepted, allowing the triangle to be rendered from either side.

### 3.2 Rust Port Implementation Approach

The Rust implementation should include a dblsided parameter in the rasterizer interface, with clear documentation explaining its purpose and behavior. The implementation should perform the area calculation and apply the appropriate sign test based on the winding order. The conditional logic can be implemented using pattern matching or simple if-else branches, with the sign test inverted based on whether the area is positive or negative.

This feature is particularly important for the terrain and mesh rendering pipelines, where double-sided surfaces are common. The Rust implementation should include unit tests that verify correct rendering of both clockwise and counter-clockwise triangles in double-sided mode, ensuring the sign inversion produces correct results.

### 3.3 Recommendation

**IMPLEMENT IMMEDIATELY** - This is a required feature for correct rendering of meshes and terrain. The priority is MEDIUM because most visible objects require double-sided rendering, though single-sided mode may be appropriate for some closed surfaces. The implementation is straightforward (simple sign inversion) but critical for visual correctness.

---

## Gap 4: RGB555 to RGB888 Conversion Optimization

### 4.1 Current State Analysis

The original implementation uses a specific formula for converting 15-bit RGB555 color values to 24-bit RGB888 format. Rather than using simple bit shifting (which would produce lower quality output), the implementation uses an optimized integer formula that produces better perceptual results: ((value * 527) + 23) >> 6. This formula approximates the ideal conversion of value * 255 / 31 while using only integer operations.

The mathematical basis for this formula is that 527/64 equals approximately 8.234, which is close to 8.225 (255/31). The addition of 23 before shifting provides rounding that improves accuracy across the entire value range. This optimization was likely chosen for performance reasons, as the integer formula is faster than floating-point division while producing visually similar results.

The source code appears in multiple locations: render.cpp (lines 3528-3530) for general color conversion and lines 865-869 for the reverse conversion (RGB888 to RGB555) used in shaders. The reverse formula uses (value * 249 + 1014) >> 11, which provides the inverse mapping with similar precision characteristics.

### 4.2 Rust Port Implementation Approach

The Rust implementation should create a dedicated color conversion module with both forward and reverse conversion functions. The implementation should preserve the optimized integer arithmetic approach, as this provides a good balance between performance and visual quality. The module should include unit tests verifying that the conversion produces values within expected ranges.

For the Rust port, there are several implementation options to consider. The first option is to replicate the exact integer formula for bit-exact compatibility with the original. The second option is to use the more straightforward (value << 3) | (value >> 2) formula, which provides a simpler but slightly less accurate conversion. A third option is to use floating-point division (value * 255 / 31), which is most readable but may have performance implications. The recommendation is to use the exact formula for compatibility unless profiling shows it to be a bottleneck.

### 4.3 Recommendation

**IMPLEMENT IMMEDIATELY** - This conversion is used extensively throughout the rendering pipeline whenever colors are displayed or processed. The priority is MEDIUM because while the simple shift alternative produces acceptable results, the optimized formula provides better visual quality with negligible performance cost. The implementation should be well-tested to ensure color fidelity is maintained.

---

## Gap 5: Glyph Coverage for Alpha Blending

### 5.1 Current State Analysis

The glyph_coverage[256] table is a precomputed mapping that provides 4-quadrant coverage information for all 256 CP437 glyphs. This table enables half-block transparency compositing by determining which color (foreground or background) dominates in each quadrant of a cell. The coverage data is encoded in a uint16 where each nibble (4 bits) represents the fill level (0-4) of one quadrant: bits 0-3 for bottom-left, bits 4-7 for bottom-right, bits 8-11 for top-left, and bits 12-15 for top-right.

This system is used primarily in sprite rendering for effects like swoosh transparency (smoke and magic effects) and distance-based dithering. When a sprite has partial transparency, the coverage table determines how to blend the sprite color with the underlying terrain. The AverageGlyph and AverageGlyphTransp functions use this table to determine which color should dominate in each cell quadrant based on the glyph shape.

The source code in sprite.cpp (lines 1944-1948) documents the encoding scheme, while the actual table definition appears in lines 1813-1840. The table appears to be hand-crafted or computed from the actual CP437 font bitmap data, with values representing the visual fill of each character in a 4-quadrant grid.

### 5.2 Rust Port Implementation Approach

The Rust implementation should port the complete glyph_coverage[256] table as a constant array. The table can be copied directly from the audit-unknown-glyph-coverage.md documentation, which already contains the full 256-value array. Additionally, the quadrant mask constants (SPRITE_MASK_LOWER, SPRITE_MASK_LEFT, SPRITE_MASK_RIGHT, SPRITE_MASK_UPPER, SPRITE_MASK_FULL) should be ported as Rust constants.

The AverageGlyph and AverageGlyphTransp functions should be implemented as methods on an appropriate type (likely part of a sprite or glyph utilities module). These functions extract the relevant nibbles based on the provided mask, compute the average coverage, and return either the foreground or background color based on whether coverage exceeds 50%. The implementation should include documentation explaining the purpose and behavior of each function.

### 5.3 Recommendation

**DEFER** - This feature is specific to sprite compositing effects that may not be required for initial rendering capability. The priority is MEDIUM because while it enables important visual effects (half-block transparency, dithering), the basic rendering pipeline can function without it. If the Rust port initially focuses on terrain and mesh rendering, this sprite-specific feature can be added later when sprite effects are needed.

---

## Consolidated Recommendations Summary

| Gap | Implementation Status | Rationale |
|-----|---------------------|-----------|
| Edge Function Mathematical Derivation | Implement Immediately | Documentation only; essential for maintainability |
| BC_P Pixel Center Sampling | Implement Immediately | Critical for visual correctness along triangle edges |
| Double-Sided Rendering Logic | Implement Immediately | Required for correct mesh and terrain rendering |
| RGB555 to RGB888 Conversion | Implement Immediately | Used throughout pipeline; optimized formula provides quality benefit |
| Glyph Coverage for Alpha Blending | Defer | Sprite-specific; not required for initial rendering capability |

---

## Implementation Dependencies

The rasterizer implementation has clear dependencies that should guide the development order. The edge function mathematics provides the foundation for all rasterization, so this should be documented early in the development process. The BC_P center sampling and double-sided logic are both integral parts of the Rasterize function and should be implemented together. The RGB555 conversion is used in multiple pipeline stages and should be available early. The glyph coverage can be added later when sprite rendering is implemented.

The recommended implementation order is: first, document the edge function mathematics in the rasterizer module; second, implement the Rasterize function with center sampling and double-sided support; third, create the color conversion utilities; fourth, add glyph coverage when sprite effects are needed.

---

## Risk Assessment

The primary risks in implementing these gaps relate to maintaining visual compatibility with the original C++ implementation. Edge function and center sampling errors would cause visible artifacts in rendered output, particularly along triangle boundaries. Double-sided rendering errors would cause incorrect or missing geometry. Color conversion errors would cause incorrect colors throughout the output. Glyph coverage errors would affect sprite blending effects.

Mitigation strategies include: comprehensive unit tests comparing Rust output to reference C++ screenshots; bit-exact integer arithmetic for color conversion; and systematic testing of both winding orders in double-sided mode. The Rust implementation should establish visual regression tests before and after each gap implementation to catch any regressions early.

---

**End of Plan**
