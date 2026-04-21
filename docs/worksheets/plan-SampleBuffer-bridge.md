> **STATUS: CONTINGENT ON D010** — This implementation plan is for the k-d tree shape-matching integration. Per DECISION_LOG D010 (Final): "Keep auto_mat initially, add k-d later." This plan should NOT be implemented until after Phase 2, when performance data justifies the transition from auto_mat to k-d tree (see D030). The technical analysis remains valid as reference for the future integration.

# Implementation Plan: SampleBuffer to Shape-Matching Bridge

## Executive Summary

This document provides a detailed implementation plan for bridging Asciicker's SampleBuffer to Alex Harri's k-d tree shape-matching system. The bridge converts RGB555 color data, depth values, and diffuse lighting coefficients from the SampleBuffer into 6D sampling vectors for nearest-neighbor character matching. This integration point represents the RESOLVE phase of Asciicker's rendering pipeline, where visual samples are converted to final ASCII output.

The implementation plan addresses four key areas: the data transformation pipeline from SampleBuffer format to 6D vectors, the timing of sampling relative to depth testing, handling of the 2x supersampling factor, and performance optimization strategies for maintaining real-time frame rates.

---

## 1. Data Transformation Pipeline

### 1.1 Overview

The transformation pipeline converts Asciicker's SampleBuffer format into the 6D sampling vectors that Alex Harri's k-d tree expects. This pipeline must handle three distinct data types: RGB555 color values, depth values, and diffuse lighting coefficients. The following diagram illustrates the overall flow:

```
SampleBuffer (2× supersampled)
         │
         ▼
┌─────────────────┐
│ RGB555 Unpack   │  Extract R, G, B from 15-bit packed format
└────────┬────────┘
         ▼
┌─────────────────┐
│ Luminance Calc  │  Convert RGB to grayscale via Rec. 709 weights
└────────┬────────┘
         ▼
┌─────────────────┐
│ 6D Vector Build │  Sample at 6 staggered positions per cell
└────────┬────────┘
         ▼
┌─────────────────┐
│ Optional: Add   │  Integrate depth and/or diffuse as extra dims
│ Depth/Diffuse   │
└────────┬────────┘
         ▼
   6D Sampling Vector
         │
         ▼
┌─────────────────┐
│ Quantized Cache │  30-bit key → O(1) lookup
└────────┬────────┘
         ▼
┌─────────────────┐
│   K-D Tree      │  Nearest neighbor search
└────────┬────────┘
         ▼
    Selected Character
```

### 1.2 RGB555 Unpacking

The SampleBuffer stores colors in RGB555 format, a 15-bit packed representation with 5 bits per color channel. The bit layout places red in bits 14-10, green in bits 9-5, and blue in bits 4-0. The unpacking process extracts these individual components and expands them to 8-bit values for further processing.

The RGB555 to RGB888 expansion follows a simple left-shift strategy that replicates the 5-bit value into the upper bits of an 8-bit byte:

```rust
fn unpack_rgb555(packed: u16) -> (u8, u8, u8) {
    let r = ((packed >> 10) & 0x1F) as u8;
    let g = ((packed >> 5) & 0x1F) as u8;
    let b = (packed & 0x1F) as u8;
    
    // Expand 5-bit to 8-bit: left-shift by 3 bits
    let r_expanded = r << 3;
    let g_expanded = g << 3;
    let b_expanded = b << 3;
    
    (r_expanded, g_expanded, b_expanded)
}
```

This expansion maps the range 0-31 to 0-248, effectively scaling by a factor of approximately 8. While more sophisticated expansion methods exist (such as bit replication for smoother gradients), the simple left-shift provides sufficient quality for ASCII rendering where character selection depends on overall structure rather than precise color accuracy.

### 1.3 Luminance Calculation

The Alex Harri system operates on luminance (lightness) values rather than raw color channels. The standard approach uses Rec. 709 luma coefficients, which provide perceptually accurate grayscale conversion that matches human vision's sensitivity to different wavelengths. The coefficients are: red contributes 0.2126, green contributes 0.7152, and blue contributes 0.0722.

The luminance calculation takes the expanded RGB888 values and computes a weighted sum:

```rust
fn calculate_luminance(r: u8, g: u8, b: u8) -> f32 {
    let r_float = r as f32 / 255.0;
    let g_float = g as f32 / 255.0;
    let b_float = b as f32 / 255.0;
    
    let luminance = 0.2126 * r_float + 0.7152 * g_float + 0.0722 * b_float;
    luminance
}
```

The output is normalized to the range [0.0, 1.0], which is required for the quantization scheme used in the cache (RANGE=8 multiplier). If performance is critical, an alternative approximation using integer arithmetic could avoid floating-point operations:

```rust
fn calculate_luminance_fast(r: u8, g: u8, b: u8) -> u8 {
    // Approximate Rec. 709 using integer weights: 54, 183, 18 (sum = 255)
    ((54u32 * r as u32 + 183u32 * g as u32 + 18u32 * b as u32) / 255) as u8
}
```

This approximation uses integer weights that sum to 255, providing a fast path that avoids floating-point division while maintaining reasonable accuracy.

### 1.4 6D Vector Construction

Alex Harri's 6D vector system samples the source content at six staggered positions within each character cell. These positions capture directional structure (stems, corners, edges) that 2D sampling cannot distinguish. The six sampling points are arranged as follows:

- Upper-left (UL): upper portion, left side
- Upper-right (UR): upper portion, right side
- Middle-left (ML): middle portion, left side
- Middle-right (MR): middle portion, right side
- Lower-left (LL): lower portion, left side
- Lower-right (LR): lower portion, right side

For each output cell, the system must sample the corresponding region in the SampleBuffer and compute the average luminance within each sampling circle. The sampling circles have a normalized radius of 0.28125 in the character cell's coordinate space.

The 6D vector construction process for a single output cell:

```rust
struct SamplingVector {
    components: [f32; 6],  // [UL, UR, ML, MR, LL, LR]
}

fn build_sampling_vector(
    sample_buffer: &SampleBuffer,
    cell_x: usize,
    cell_y: usize,
    cell_width: usize,
    cell_height: usize,
) -> SamplingVector {
    // Sampling circle radius: 0.28125 of cell size
    let radius = 0.28125;
    
    // Normalized sampling positions within cell [0, 1]
    let positions: [(f32, f32); 6] = [
        (0.25, 0.25),   // Upper-left
        (0.75, 0.25),   // Upper-right
        (0.25, 0.5),    // Middle-left
        (0.75, 0.5),    // Middle-right
        (0.25, 0.75),   // Lower-left
        (0.75, 0.75),   // Lower-right
    ];
    
    let mut components = [0.0f32; 6];
    
    for (i, (nx, ny)) in positions.iter().enumerate() {
        // Convert normalized position to buffer coordinates
        let center_x = cell_x as f32 + nx * cell_width as f32;
        let center_y = cell_y as f32 + ny * cell_height as f32;
        
        // Sample circular region (accounting for aspect ratio)
        let radius_x = radius * cell_width as f32;
        let radius_y = radius * cell_height as f32;
        
        components[i] = sample_circular_region(
            sample_buffer,
            center_x,
            center_y,
            radius_x,
            radius_y,
        );
    }
    
    SamplingVector { components }
}

fn sample_circular_region(
    buffer: &SampleBuffer,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
) -> f32 {
    // Determine bounds of circular region in buffer coordinates
    let min_x = (cx - rx).ceil() as i32;
    let max_x = (cx + rx).floor() as i32;
    let min_y = (cy - ry).ceil() as i32;
    let max_y = (cy + ry).floor() as i32;
    
    let mut sum = 0.0f32;
    let mut count = 0u32;
    
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            // Check if point is within ellipse
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry) <= 1.0 {
                if let Some(sample) = buffer.get(x, y) {
                    let (r, g, b) = unpack_rgb555(sample.visual);
                    sum += calculate_luminance(r, g, b);
                    count += 1;
                }
            }
        }
    }
    
    if count > 0 {
        sum / count as f32
    } else {
        0.0  // Return black for out-of-bounds regions
    }
}
```

### 1.5 Depth Integration Strategy

The SampleBuffer contains depth information in the `height` field, where negative values indicate surfaces closer to the viewer. This depth information could influence character selection in several ways, but the recommended approach is to ignore depth during shape-matching and rely solely on visual content.

The rationale for this recommendation is threefold. First, Alex Harri's system has no inherent concept of depth and was designed for image-to-ASCII conversion where depth is unavailable. Second, the depth buffer already determines visibility: the content at each sample position is whatever surface is visible at that pixel, so sampling the resolved visual content captures exactly what should be rendered. Third, integrating depth as an additional dimension would expand the k-d tree to 7D, increasing tree depth and potentially reducing cache hit rates.

If depth influence is desired for special effects (such as preferring denser characters for nearby objects), this could be implemented as a post-processing step after character selection rather than as part of the sampling vector itself.

### 1.6 Diffuse Lighting Integration

Asciicker stores diffuse lighting values (0-255) in the Sample::diffuse field, which represents per-vertex or per-face illumination intensity. The current auto_mat system uses an 11-level lighting scale (values 0-10) to select between different glyphs for the same base color, creating the perception of shading.

Integrating diffuse lighting with shape-matching can proceed in two ways. The recommended approach is a hybrid pipeline: shape-matching selects a character based on structural information (the 6D visual vector), then the existing lighting system applies shading to the selected character. This preserves both the improved edge quality from shape-matching and the shading behavior that players expect.

An alternative approach would extend the k-d tree to 7D by appending normalized diffuse as a seventh dimension. This would require regenerating character vectors to include lighting variations, significantly increasing alphabet size. A 95-character alphabet with 11 lighting levels would become 1,045 vectors, increasing tree depth from approximately 7 to approximately 10 levels.

The recommended hybrid pipeline:

```rust
fn resolve_cell_shape_matching(
    sample_buffer: &SampleBuffer,
    kd_tree: &KdTree<char>,
    cache: &mut QuantizedCache<char>,
    cell_x: usize,
    cell_y: usize,
) -> (char, u8) {  // Returns (glyph, diffuse)
    // Build 6D sampling vector from visual content
    let vector = build_sampling_vector(sample_buffer, cell_x, cell_y);
    
    // Check quantized cache
    let key = quantize_key(&vector);
    if let Some(&cached_char) = cache.get(&key) {
        // Retrieve diffuse from resolved sample
        let diffuse = sample_buffer.get(cell_x, cell_y).map(|s| s.diffuse).unwrap_or(0);
        return (cached_char, diffuse);
    }
    
    // K-D tree lookup on miss
    let best_char = kd_tree.nearest_neighbor(&vector);
    
    // Cache the result
    cache.insert(key, best_char);
    
    // Return with diffuse lighting
    let diffuse = sample_buffer.get(cell_x, cell_y).map(|s| s.diffuse).unwrap_or(0);
    (best_char, diffuse)
}
```

---

## 2. When to Sample

### 2.1 Analysis of Options

The sampling timing determines which visual content feeds into the shape-matching algorithm. Three options exist: sampling before depth testing, sampling after depth testing, or sampling the final resolved color. Each option produces different results and has distinct performance characteristics.

**Option A: Sample Before Depth Test**

This approach would sample the raw color data before the depth buffer determines visibility. At each sample position, multiple surfaces might contribute color, and the sampling would capture all contributions rather than just the visible one.

This option is not recommended for two reasons. First, the depth buffer exists precisely to determine which surface is visible at each pixel; sampling before this determination ignores that information. Second, surfaces that are occluded but still contribute to the sample would create misleading visual data that does not represent what the player actually sees.

**Option B: Sample After Depth Test**

This approach samples the color data at positions where the depth buffer has already determined visibility. Each sample contains the color of the visible surface at that position, which accurately represents the rendered output.

This is the recommended approach and aligns with how Alex Harri's system expects input: a resolved image where each position contains exactly what should be displayed. The SampleBuffer after the RESOLVE phase (or after sprite rendering) contains precisely this data.

**Option C: Sample the Final Resolved Color**

This approach would sample after all post-processing is complete, including color quantization to xterm-256 and any other transformations applied before writing to the output buffer.

This option is not recommended because the xterm-256 quantization discards significant color information. The shape-matching algorithm benefits from the full RGB555 color range (32,768 combinations) rather than the reduced 256-color palette. Additionally, sampling at this stage would require modifying the existing resolve pipeline rather than inserting the shape-matching bridge as a drop-in replacement.

### 2.2 Recommended Approach: Sample After Depth Test

The recommended integration point is immediately after the existing depth testing completes but before the auto_mat lookup in the RESOLVE phase. Specifically, the shape-matching bridge should replace or augment the auto_mat lookup while using the same sample data that auto_mat currently consumes.

The integration replaces the auto_mat function call:

```rust
// Current pipeline (simplified)
for each cell (x, y):
    sample = resolve_2x2_to_single(sample_buffer, x, y)  // Average RGB555
    ansi_cell = auto_mat(sample.visual, sample.diffuse)

// New pipeline with shape-matching
for each cell (x, y):
    sample = resolve_2x2_to_single(sample_buffer, x, y)
    
    // Shape-matching path
    vector = build_sampling_vector(sample_buffer, x, y)
    glyph = kd_tree_lookup(vector)  // Or cached result
    
    // Preserve existing color pipeline
    ansi_cell = AnsiCell {
        glyph: glyph,  // From shape-matching
        fg: auto_mat_fg(sample.visual, sample.diffuse),
        bk: auto_mat_bg(sample.visual, sample.diffuse),
    }
```

This approach preserves the existing color handling while replacing only the glyph selection logic. The existing auto_mat system continues to handle foreground/background color selection based on RGB555 and diffuse values.

---

## 3. Handling 2x Supersampling

### 3.1 Supersampling in Asciicker

Asciicker renders to a SampleBuffer that is 2x supersampled relative to the output AnsiCell resolution. For an output of W×H cells, the SampleBuffer contains 2W×2H samples. This supersampling serves two purposes: it provides data for averaging to produce smoother final output, and it enables proper sampling positions for the 6D vectors which need sub-cell resolution.

The RESOLVE phase currently handles supersampling by averaging each 2×2 block of samples into a single representative value:

```rust
fn resolve_2x2_block(buffer: &SampleBuffer, cell_x: usize, cell_y: usize) -> Sample {
    let mut sum_visual_r = 0u32;
    let mut sum_visual_g = 0u32;
    let mut sum_visual_b = 0u32;
    let mut sum_diffuse = 0u32;
    let mut min_height = f32::MAX;
    
    // Sample the 2×2 block (4 samples per cell)
    for dy in 0..2 {
        for dx in 0..2 {
            let sample = buffer.get(cell_x * 2 + dx, cell_y * 2 + dy);
            if let Some(s) = sample {
                let (r, g, b) = unpack_rgb555(s.visual);
                sum_visual_r += r as u32;
                sum_visual_g += g as u32;
                sum_visual_b += b as u32;
                sum_diffuse += s.diffuse as u32;
                min_height = min_height.min(s.height);
            }
        }
    }
    
    // Average and repack to RGB555
    let r_avg = (sum_visual_r / 4) as u8;
    let g_avg = (sum_visual_g / 4) as u8;
    let b_avg = (sum_visual_b / 4) as u8;
    let visual_packed = pack_rgb555(r_avg, g_avg, b_avg);
    
    Sample {
        height: min_height,  // Use closest depth
        visual: visual_packed,
        diffuse: (sum_diffuse / 4) as u8,
        spare: 0,
    }
}
```

### 3.2 Supersampling Strategy for Shape-Matching

For shape-matching, the 6D sampling vectors need more granular data than the simple 2×2 average provides. The recommended strategy is to sample directly from the supersampled buffer at the precise positions needed for the 6D vector, rather than using the pre-averaged resolved samples.

This approach provides maximum flexibility and accuracy: each of the six sampling circles can span multiple samples in the supersampled buffer, computing an accurate average luminance that captures the local structure. The 2× supersampling provides sufficient resolution for this purpose.

### 3.3 Implementation Strategy

The sampling vector computation should access the raw 2× supersampled buffer directly, using the cell dimensions (2× the output cell size) to position the six sampling circles:

```rust
fn build_sampling_vector_from_supersampled(
    buffer: &SampleBuffer,
    cell_x: usize,
    cell_y: usize,
) -> SamplingVector {
    // Each cell is 2×2 samples in the buffer
    let buffer_cell_x = cell_x * 2;
    let buffer_cell_y = cell_y * 2;
    let cell_width = 2;
    let cell_height = 2;
    
    // Use the same 6D positioning logic, but with buffer coordinates
    build_sampling_vector(
        buffer,
        buffer_cell_x,
        buffer_cell_y,
        cell_width,
        cell_height,
    )
}
```

This approach leverages the existing supersampling without modification to the rendering pipeline. The shape-matching bridge operates on the raw supersampled data, computing sampling vectors with the benefit of 2× resolution.

---

## 4. Performance Considerations

### 4.1 Memory Access Patterns

The SampleBuffer layout significantly impacts cache performance during sampling vector computation. The current structure stores samples in row-major order (contiguous X within each row), which provides good locality when processing cells sequentially. However, the 6D vector sampling requires accessing samples at non-sequential positions within each cell, which may cause cache misses.

For a 240×135 output resolution (the example from the gap analysis), the SampleBuffer contains 480×270 samples (129,600 samples total). Processing cells row-by-row provides reasonable cache utilization since adjacent cells share sample positions.

The recommended approach processes cells in scan order (row-by-row, left-to-right), which naturally aligns with the buffer layout:

```rust
fn resolve_frame(
    sample_buffer: &SampleBuffer,
    kd_tree: &KdTree<char>,
    cache: &mut QuantizedCache<char>,
    output: &mut [AnsiCell],
    width: usize,
    height: usize,
) {
    for y in 0..height {
        for x in 0..width {
            // Build sampling vector from supersampled buffer
            let vector = build_sampling_vector_from_supersampled(
                sample_buffer,
                x,
                y,
            );
            
            // Quantized cache lookup
            let key = quantize_key(&vector);
            let glyph = *cache.get_or_insert_with(key, || {
                kd_tree.nearest_neighbor(&vector)
            });
            
            // Get resolved sample for color
            let sample = resolve_2x2_block(sample_buffer, x, y);
            let (fg, bg) = auto_mat_color(sample.visual, sample.diffuse);
            
            output[y * width + x] = AnsiCell { glyph, fg, bk: bg };
        }
    }
}
```

### 4.2 Quantized Cache Implementation

The quantized cache is critical for real-time performance. Alex Harri's implementation uses 5-bit quantization per component with a RANGE of 8, producing a practical key space of 262,144 entries. This cache provides O(1) lookup for repeated sampling vectors.

The cache key generation:

```rust
fn quantize_key(vector: &[f32; 6]) -> u32 {
    const BITS: u32 = 5;
    const RANGE: u32 = 8;
    
    let mut key = 0u32;
    for &component in vector.iter() {
        // Clamp to [0, 1] range
        let clamped = component.max(0.0).min(1.0);
        // Quantize to 0-7 (RANGE - 1)
        let quantized = (clamped * RANGE as f32).floor() as u32;
        let quantized = quantized.min(RANGE - 1);
        // Pack into key
        key = (key << BITS) | quantized;
    }
    key
}
```

For the asciicker integration, the cache should be implemented as a frame-persistent structure that maintains entries across frames. The gap analysis notes that adjacent cells and consecutive frames tend to produce similar vectors, so initializing each frame's cache with the previous frame's entries provides cache warming:

```rust
struct ShapeMatchingContext {
    kd_tree: KdTree<char>,
    cache: FxHashMap<u32, char>,  // Or standard HashMap
    previous_frame_cache: FxHashMap<u32, char>,
}

impl ShapeMatchingContext {
    fn new(alphabet: &Alphabet) -> Self {
        let kd_tree = KdTree::build(&alphabet.vectors);
        
        // Pre-warm cache with previous frame data
        let cache = FxHashMap::with_capacity_and_hasher(
            1024,  // Initial capacity hint
            Default::default(),
        );
        
        ShapeMatchingContext {
            kd_tree,
            cache,
            previous_frame_cache: FxHashMap::default(),
        }
    }
    
    fn resolve_cell(&mut self, vector: &[f32; 6]) -> char {
        let key = quantize_key(vector);
        
        // Check current cache
        if let Some(&glyph) = self.cache.get(&key) {
            return glyph;
        }
        
        // Check previous frame cache (cache warming)
        if let Some(&glyph) = self.previous_frame_cache.get(&key) {
            self.cache.insert(key, glyph);
            return glyph;
        }
        
        // K-D tree lookup on miss
        let glyph = self.kd_tree.nearest_neighbor(vector);
        self.cache.insert(key, glyph);
        glyph
    }
    
    fn end_frame(&mut self) {
        // Promote current cache to previous frame for next frame's warming
        self.previous_frame_cache = std::mem::take(&mut self.cache);
        self.cache = FxHashMap::with_capacity_and_hasher(
            self.previous_frame_cache.len(),
            Default::default(),
        );
    }
}
```

### 4.3 K-D Tree Optimization

The k-d tree lookup is O(log n) in the number of characters, which for an 80-95 character alphabet yields approximately 6-7 comparisons per lookup. While this is already fast, several optimizations can improve real-world performance.

**Squared Distance Comparison**: The standard Euclidean distance computes a square root for each comparison. Comparing squared distances avoids this expensive operation:

```rust
fn distance_squared(v1: &[f32; 6], v2: &[f32; 6]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..6 {
        let diff = v1[i] - v2[i];
        sum += diff * diff;
    }
    sum
}

// In k-d tree search, compare squared distances
if best_squared > dist_squared {
    best_squared = dist_squared;
    best_char = node.data;
}
```

**Stack-Based Iterative Traversal**: The TypeScript implementation uses recursive function calls for k-d tree traversal. In Rust, an iterative implementation with an explicit stack avoids function call overhead and potential stack overflow:

```rust
fn nearest_neighbor(&self, target: &[f32; 6]) -> char {
    let mut best_char = self.root.data;
    let mut best_dist = distance_squared(target, &self.root.vector);
    
    // Explicit stack to avoid recursion
    let mut stack = vec![(self.root.left.as_ref(), self.root.right.as_ref())];
    
    while let Some((left, right)) = stack.pop() {
        // Process both children with proper pruning
        if let Some(node) = left {
            let dist = distance_squared(target, &node.vector);
            if dist < best_dist {
                best_dist = dist;
                best_char = node.data;
            }
            
            // Check if we need to explore the other side
            let diff = target[node.axis] - node.vector[node.axis];
            if diff * diff < best_dist {
                // Push the other child for exploration
                if let Some(other) = right {
                    stack.push((Some(other), None));
                }
            }
        }
        // Similar logic for right child...
    }
    
    best_char
}
```

### 4.4 SIMD Vectorization

The 6D distance calculation is a natural candidate for SIMD vectorization. The portable_simd crate provides architecture-agnostic SIMD support that can accelerate the distance calculations:

```rust
use std::simd::{f32x4, SimdFloat};

fn distance_squared_simd(v1: &[f32; 6], v2: &[f32; 6]) -> f32 {
    // Load first 4 components into SIMD register
    let a1 = f32x4::from_array([v1[0], v1[1], v1[2], v1[3]]);
    let b1 = f32x4::from_array([v2[0], v2[1], v2[2], v2[3]]);
    let diff1 = a1 - b1;
    let sum1 = diff1 * diff1;
    
    // Load remaining 2 components (plus padding)
    let a2 = f32x4::from_array([v1[4], v1[5], 0.0, 0.0]);
    let b2 = f32x4::from_array([v2[4], v2[5], 0.0, 0.0]);
    let diff2 = a2 - b2;
    let sum2 = diff2 * diff2;
    
    // Horizontal sum of all lanes
    (sum1.reduce_sum() + sum2.reduce_sum()) as f32
}
```

However, the gap analysis notes that k-d tree traversal involves significant conditional branching based on axis comparison, which traditionally does not vectorize well. The recommended strategy is to implement SIMD for the sampling vector computation (which processes multiple cells in parallel) while keeping the k-d tree traversal in scalar code.

### 4.5 Multi-Threading Strategy

The cell processing in the RESOLVE phase is embarrassingly parallel: each cell's sampling vector computation and k-d tree lookup is independent of other cells. This enables straightforward parallelization using Rayon or crossbeam:

```rust
use rayon::prelude::*;

fn resolve_frame_parallel(
    sample_buffer: &SampleBuffer,
    kd_tree: &KdTree<char>,
    cache: &Mutex<QuantizedCache<char>>,
    output: &mut [AnsiCell],
    width: usize,
    height: usize,
) {
    // Process rows in parallel
    output
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..width {
                let vector = build_sampling_vector_from_supersampled(
                    sample_buffer,
                    x,
                    y,
                );
                
                // Thread-safe cache access
                let glyph = cache.lock().unwrap().lookup_or_insert(
                    &vector,
                    || kd_tree.nearest_neighbor(&vector),
                );
                
                let sample = resolve_2x2_block(sample_buffer, x, y);
                let (fg, bg) = auto_mat_color(sample.visual, sample.diffuse);
                
                row[x] = AnsiCell { glyph: *glyph, fg, bk: bg };
            }
        });
}
```

The quantized cache requires synchronization when accessed from multiple threads. Options include a Mutex-protected shared cache, thread-local caches with periodic merging, or a lock-free concurrent hash map.

### 4.6 Performance Budget Analysis

At 60fps, each frame has a 16.67ms budget. The gap analysis estimates that k-d tree lookups require approximately 12,000 distance calculations for a standard 80×25 terminal (2000 cells × 6 comparisons). Adding sampling vector computation, the total budget allocation should be:

| Component | Estimated Time | Notes |
|-----------|---------------|-------|
| Sampling vector computation | 1-2ms | 6 circular region averages per cell |
| Quantized cache lookup | 0.1-0.2ms | Hash map O(1) |
| K-D tree traversal | 0.5-1ms | ~6 comparisons per cell |
| AnsiCell construction | 0.1-0.2ms | Color lookup, memory writes |
| **Total per frame** | **2-4ms** | Conservative estimate |

This leaves substantial headroom within the 16.67ms frame budget, even at standard terminal resolutions. At higher resolutions (240×135 cells = 32,400 cells), the cost scales linearly with cell count, bringing the estimate to approximately 10-15ms, which may require the parallel processing strategy or resolution scaling.

---

## 5. Implementation Roadmap

### 5.1 Phase 1: Core Bridge (Priority: HIGH)

The first phase implements the basic data transformation pipeline without optimizations:

1. Implement RGB555 unpacking and luminance calculation
2. Implement 6D sampling vector construction from supersampled buffer
3. Integrate k-d tree with basic character lookup
4. Replace auto_mat glyph selection with shape-matching in RESOLVE phase
5. Verify visual output matches expected behavior

This phase establishes the fundamental bridge and validates the integration approach.

### 5.2 Phase 2: Cache Optimization (Priority: HIGH)

The second phase adds the quantized cache for performance:

1. Implement quantized cache key generation (5 bits × 6 components)
2. Implement cache lookup with fallback to k-d tree
3. Add frame-persistent cache with previous-frame warming
4. Benchmark cache hit rates with representative game content

This phase should dramatically improve performance for typical game scenes where large regions have uniform color.

### 5.3 Phase 3: Advanced Features (Priority: MEDIUM)

The third phase adds optional enhancements:

1. Implement temporal smoothing to reduce flicker during camera movement
2. Add support for the global and directional crunch effects from Alex Harri's system
3. Implement hybrid glyph + lighting selection
4. Add sprite animation handling (per-frame analysis)

This phase refines visual quality and handles edge cases.

### 5.4 Phase 4: Performance Tuning (Priority: MEDIUM)

The fourth phase optimizes for production use:

1. Implement SIMD vectorization for sampling computation
2. Add multi-threaded parallel processing
3. Implement resolution scaling for performance management
4. Add detailed performance profiling and tuning

This phase ensures the system meets 60fps targets at high resolutions.

---

## 6. Open Questions and Future Research

### 6.1 Font Consistency

The alphabet character vectors were generated using Fira Code monospace font. The gap analysis notes that using a different font would result in mismatched character shapes. The implementation should verify that Asciicker's runtime font matches Fira Code, or document any adjustments needed for different fonts.

### 6.2 Temporal Coherence During Rapid Camera Movement

The gap analysis identifies flickering as a potential issue during rapid camera movement. The recommended mitigation is temporal smoothing: storing a weighted average of recent character selections and only changing when the new selection is significantly better. This could be implemented as a post-processing step after k-d tree lookup.

### 6.3 Hybrid Color Handling

The current plan preserves the existing color pipeline (xterm-256 color mapping) while replacing only glyph selection. An alternative would be to include color in the shape-matching process, selecting characters based on both structure and color. This would require extending the 6D vectors or creating separate alphabets for different color ranges.

### 6.4 Sprite Animation Integration

Sprite animation frames are not currently handled by the shape-matching system. Each animation frame should produce appropriate sampling vectors, and the system should analyze each frame independently. Further research is needed to determine optimal handling of animated content.

---

## 7. Summary

The SampleBuffer to Shape-Matching Bridge converts Asciicker's RGB555 color data, depth values, and diffuse lighting into 6D sampling vectors for Alex Harri's k-d tree character selection. The implementation plan recommends:

1. **Data Transformation Pipeline**: Unpack RGB555 to RGB888, calculate luminance via Rec. 709 weights (0.2126, 0.7152, 0.0722), and construct 6D vectors by sampling at staggered positions within each cell. Depth should be ignored (sampling resolved visual content), and diffuse lighting should be handled via hybrid post-selection rather than as an extra dimension.

2. **When to Sample**: Sample after depth testing completes, using the resolved visual content that represents exactly what the player sees. This is the recommended integration point in the RESOLVE phase.

3. **Supersampling Handling**: Sample directly from the 2× supersampled buffer at the precise positions needed for the 6D vectors, leveraging the existing supersampling for accurate circular region sampling.

4. **Performance**: Implement the quantized cache (5 bits × 6 components, RANGE=8) with frame-persistent warming. K-D tree lookups at O(log n) are fast; the primary cost is sampling vector computation. SIMD and multi-threading are available as needed for high resolutions.

This implementation plan provides a concrete roadmap for bridging Asciicker's rendering output to the Alex Harri shape-matching system, enabling high-quality ASCII character selection that captures structural edges and visual features rather than relying solely on brightness-to-density mapping.

---

*Document Version: 1.0*
*Created: 2026-02-20*
*Scope: Integration between Asciicker SampleBuffer and Alex Harri k-d tree shape-matching*
