# Phase 5: Pipeline Integration - Research

**Researched:** 2026-02-20
**Domain:** Terrain quadtree, BSP world traversal, camera system, deferred sprite blit, terrain shadows, full 6-stage pipeline integration, golden-file CI
**Confidence:** HIGH

## Summary

Phase 5 is the critical convergence point where all prior subsystems (Phase 2 asset parsers, Phase 3 GPU output, Phase 4 CPU rasterizer core) connect to render a real Asciicker .a3d world file. This phase adds four new subsystems (terrain quadtree, BSP world tree, camera, terrain shadows) and two pipeline features (deferred sprite blit, golden-file CI), then wires everything into the 6-stage rendering pipeline orchestrator.

The C++ codebase has been thoroughly documented through skill packs and architecture docs. The terrain system (~3300 lines in terrain.cpp) implements a quadtree with frustum-culled traversal, 5x5 vertex grids per patch (HEIGHT_CELLS=4), 8x8 material grids (VISUAL_CELLS=8), and 64-bit shadow bitmasks. The world system (~5000 lines in world.cpp) implements SAH-based BSP tree construction with 4 node types and frustum-culled traversal that dispatches mesh and sprite callbacks. The camera system is embedded in Render() (render.cpp:2838-4412) with both isometric and perspective modes, where perspective uses an "architectural" projection (commented-out sin30/cos30 for non-tilted view). The deferred sprite blit sorts queued sprites far-to-near and composites them after the RESOLVE stage using painter's algorithm with per-sample depth testing.

**Primary recommendation:** Build the phase in 5-6 plans: (1) terrain quadtree runtime with frustum query, (2) BSP tree construction and frustum-culled world query, (3) perspective camera with Q/E rotation, (4) terrain shadow computation, (5) pipeline orchestrator wiring all stages + deferred sprite blit, (6) golden-file CI comparison. Each plan produces testable output.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TERR-01 | Quadtree heightmap with HEIGHT_CELLS=4 (5x5 vertex grid per patch) | Terrain quadtree data structures, C++ terrain.cpp quadtree navigation documented in arch/terrain_cpp_part1.md. TerrainPatch already parsed in Phase 2. |
| TERR-02 | VISUAL_CELLS=8 material grid (8x8 cells per patch) | Material grid data already parsed. RenderPatch callback indexes visual map per triangle. |
| TERR-03 | Quadtree propagates height bounds for frustum culling | C++ UpdateNodes propagates lo/hi from patches to ancestors. QueryTerrain tests AABB against frustum planes with early-out and plane elimination optimization. |
| TERR-04 | Known C++ bugs fixed during port (TERRAIN-001 through TERRAIN-004) | TERRAIN-001: terrain.cpp:613 `if(x)` should be `if(y)`. TERRAIN-002: terrain.cpp:805 `u < y` should be `u < v`. TERRAIN-003: terrain.cpp:1671 same as TERRAIN-002. TERRAIN-004: terrain.cpp:480,492 verify `>` vs `>=`. |
| WRLD-01 | BSP tree with SAH-style construction | SplitBSP (world.cpp:1340-1603) documented: SAH tests 3 axes, sorts by centroid, partitions. Creates NODE/NODE_SHARE/LEAF/INST types. |
| WRLD-02 | 4 BSP node types supported (NODE, NODE_SHARE, LEAF, INST) | All 4 types documented. NODE: interior with 2 children. NODE_SHARE: interior + straddling instance list. LEAF: linked list of instances. INST: promoted single instance. |
| WRLD-03 | Frustum-culled BSP traversal for rendering | QueryWorld tests AABB 8 corners against frustum planes, eliminates fully-inside planes, early-outs fully-outside. Dispatches mesh_cb and sprite_cb per visible instance. |
| WRLD-04 | Instance flags functional (VISIBLE, USE_TREE, VOLATILE, SELECTED) | INST_VISIBLE=0x1 (rendered), INST_USE_TREE=0x2 (in BSP), INST_VOLATILE=0x4 (temp), INST_SELECTED=0x8 (editor). Only USE_TREE and VISIBLE needed for Phase 5. |
| REND-08 | Deferred sprite blit post-RESOLVE (painter's algorithm, far-to-near sort) | Sprites queued during world query (RenderSprite v1), sorted by dist via qsort, blitted via RenderSprite v2 after RESOLVE. Per-sample 2x2 depth testing against SampleBuffer. |
| REND-09 | Terrain shadow computation (64-bit bitmask per patch) | DarkUpdater callback raycasts from each visual cell center toward light direction, tests HitTerrain then HitWorld. 64 bits = 8x8 cells, 1 bit per cell. |
| CAM-01 | Perspective camera with configurable FOV | C++ Render() constructs "architectural" perspective: focal = max(dw,dh)*2, view_dir horizontal (sin/cos of yaw), no vertical tilt. FOV is implicit via focal length. |
| CAM-02 | Q/E rotation toggle (required by D004-D005) | Yaw parameter to Render(). Q/E key bindings toggle yaw by fixed increment. Camera constructs view matrix from yaw + position. |
| CAM-03 | Scene shift in sample-buffer space (multiplied by 2 per TRAP-R06) | scene_shift[2] multiplied by 2 in both isometric and perspective tm[12]/tm[13] offsets and view_ofs. TRAP-R06 documented in skill pack. |
| VIS-02 | Golden-file CI comparison of AnsiCell output against C++ reference (<1% cell difference threshold) | Requires capturing C++ reference output for a canonical scene, then comparing Rust AnsiCell grid cell-by-cell. Threshold: <1% cells different (fg, bk, gl comparison). |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bevy | 0.18.0 | ECS, windowing, input, asset system | Already pinned in Cargo.toml; provides all game framework needs |
| bytemuck | 1.x | Zero-copy struct casting | Already used in Phase 2 terrain parser for FilePatch |
| flate2 | 1.x | Gzip decompression | Already used in Phase 2 for .xp files |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| noise (perlin) | -- | Perlin noise for water ripple | Phase 6 water effects, not needed Phase 5 |

### No New Dependencies Needed
Phase 5 requires no new crate dependencies. All work is algorithmic (quadtree, BSP, frustum culling, camera math) using existing Bevy math types (`Vec3`, `Mat4`, `Quat`) and the asset types already defined in Phase 2.

## Architecture Patterns

### Recommended Project Structure (Phase 5 additions)
```
src/
  terrain/
    mod.rs              # TerrainPlugin, runtime terrain resource
    quadtree.rs         # Quadtree node types, traversal, frustum query
    patch_runtime.rs    # Runtime patch with shadow bitmask, height bounds
    shadow.rs           # UpdateTerrainDark, DarkUpdater equivalent
  world/
    mod.rs              # WorldPlugin, runtime world resource
    bsp.rs              # BSP tree types, SAH construction, frustum query
    instance.rs         # Runtime instance types (mesh, sprite, item)
  render/
    mod.rs              # CpuRasterizerPlugin + pipeline orchestrator system
    pipeline.rs         # 6-stage pipeline orchestrator (the Render() equivalent)
    camera.rs           # Perspective camera state, view matrix construction
    sprite_blit.rs      # Deferred sprite queue, far-to-near sort, BlitSprite
    # (existing from Phase 4:)
    sample_buffer.rs    # SampleBuffer with 2x supersampling
    rasterizer.rs       # Bresenham lines, barycentric triangles
    quantize.rs         # RGB555 -> xterm-256 color quantization
    material.rs         # auto_mat shade tables
  tests/
    golden_pipeline.rs  # Golden-file comparison of full-scene AnsiCell output
```

### Pattern 1: Quadtree with Enum Nodes
**What:** Rust enum for quadtree nodes instead of C++ void pointer + level tracking.
**When to use:** All terrain quadtree code.
**Example:**
```rust
// Quadtree node: either interior (4 children) or leaf (patch data)
pub enum QuadNode {
    Interior {
        children: [Option<Box<QuadNode>>; 4],
        lo: u16,  // min height bound (propagated from children)
        hi: u16,  // max height bound (propagated from children)
    },
    Leaf(RuntimePatch),
}

pub struct RuntimePatch {
    pub x: i32,
    pub y: i32,
    pub height: [[u16; 5]; 5],     // HEIGHT_CELLS+1 = 5
    pub visual: [[u16; 8]; 8],     // VISUAL_CELLS = 8
    pub diag: u16,
    pub dark: u64,                  // shadow bitmask (64 bits for 8x8 cells)
    pub lo: u16,                    // min height
    pub hi: u16,                    // max height
}
```

### Pattern 2: BSP Tree with Enum Nodes
**What:** Rust enum for BSP node types, replacing C++ type tag + casts.
**When to use:** All BSP tree code.
**Example:**
```rust
pub enum BspNode {
    Node {
        children: [Option<Box<BspNode>>; 2],
        bbox: [f64; 6],  // xmin, xmax, ymin, ymax, zmin, zmax
        split_plane: f64,
        split_axis: u8,
    },
    NodeShare {
        children: [Option<Box<BspNode>>; 2],
        bbox: [f64; 6],
        instances: Vec<InstanceId>,  // straddling instances
    },
    Leaf {
        bbox: [f64; 6],
        instances: Vec<InstanceId>,
    },
    Inst {
        bbox: [f64; 6],
        instance: InstanceId,
    },
}
```

### Pattern 3: Frustum Culling with Plane Elimination
**What:** Port the C++ frustum test that checks 8 AABB corners against each plane, eliminates planes where all corners pass, early-outs where all corners fail.
**When to use:** Both terrain QueryTerrain and world QueryWorld.
**Example:**
```rust
pub fn frustum_test_aabb(bbox: &[f64; 6], planes: &mut Vec<[f64; 4]>) -> FrustumResult {
    let corners = aabb_corners(bbox);
    let mut i = 0;
    while i < planes.len() {
        let mut neg = 0;
        let mut pos = 0;
        for corner in &corners {
            if dot_plane(&planes[i], corner) >= 0.0 {
                pos += 1;
            } else {
                neg += 1;
            }
        }
        if neg == 8 { return FrustumResult::Outside; }
        if pos == 8 {
            // All corners inside this plane -- remove plane from further tests
            planes.swap_remove(i);
            continue;
        }
        i += 1;
    }
    if planes.is_empty() { FrustumResult::Inside } else { FrustumResult::Partial }
}
```

### Pattern 4: Pipeline Orchestrator as Bevy System
**What:** The 6-stage pipeline runs as a single Bevy system that reads terrain/world resources and writes to SampleBuffer, then resolves to AsciiCellGrid.
**When to use:** Main render loop.
**Example:**
```rust
fn render_pipeline_system(
    terrain: Res<RuntimeTerrain>,
    world_data: Res<RuntimeWorld>,
    materials: Res<MaterialTable>,
    camera: Res<GameCamera>,
    mut sample_buffer: ResMut<SampleBuffer>,
    mut cell_grid: ResMut<AsciiCellGrid>,
    config: Res<RenderConfig>,
) {
    // Stage 1: Clear
    sample_buffer.clear_fast();  // memcpy from clean half

    // Stage 2: Terrain
    terrain.query_visible(&camera.frustum_planes(), |patch, x, y| {
        render_patch(&mut sample_buffer, patch, x, y, &camera, &materials);
    });

    // Stage 3: World
    let mut sprite_queue = Vec::new();
    world_data.query_visible(&camera.frustum_planes(), |inst| {
        match inst {
            VisibleInst::Mesh { mesh, tm } => render_mesh(&mut sample_buffer, mesh, tm, &camera),
            VisibleInst::Sprite { data } => sprite_queue.push(data),
        }
    });

    // Stage 4: Shadow (player shadow -- Phase 6)
    // Stage 5: Reflection (water -- Phase 6)

    // Stage 6: Resolve
    resolve_to_cells(&sample_buffer, &materials, &mut cell_grid);

    // Post-resolve: Deferred sprite blit
    sprite_queue.sort_by(|a, b| b.dist.partial_cmp(&a.dist).unwrap_or(Ordering::Equal));
    for sprite_data in &sprite_queue {
        blit_sprite(&mut cell_grid, &sample_buffer, sprite_data);
    }
}
```

### Pattern 5: Camera State Resource
**What:** Encapsulate all camera state (position, yaw, focal, view matrix, inverse matrix, frustum planes) in a Bevy Resource updated from input.
**When to use:** Camera system.
**Example:**
```rust
#[derive(Resource)]
pub struct GameCamera {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub zoom: f32,
    pub focal: f32,
    pub perspective: bool,
    // Derived (recomputed each frame):
    pub view_tm: [f64; 16],      // 4x4 view matrix
    pub inv_tm: [f64; 16],       // inverse for unprojection
    pub view_dir: [f32; 3],      // normalized view direction
    pub view_pos: [f32; 3],      // camera world position
    pub view_ofs: [f32; 2],      // screen center offset
    pub scene_shift: [i32; 2],   // screen shake (x2 internally)
}
```

### Anti-Patterns to Avoid
- **Global mutable state:** C++ uses `global_refl_mode`, `render_break_point[2]`, etc. In Rust, pass these as parameters or store in the Renderer resource.
- **Void pointer callbacks:** C++ uses `void* cookie` for callbacks. In Rust, use closures with captured references or pass struct context directly.
- **Unsafe pointer arithmetic for quadtree navigation:** C++ casts QuadItem* to Patch* at leaf level. Use Rust enums with match.
- **Lazy allocation in the render loop:** C++ lazily allocates SampleBuffer on first Render(). In Rust, initialize at resource creation time and handle resize via system.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Matrix math | Custom 4x4 matrix ops | `bevy::math::Mat4`, `Vec3`, `Quat` | Bevy's glam-based math is optimized and correct |
| Frustum plane extraction | Manual plane computation | Adapt from C++ Render() view matrix construction | The C++ code constructs frustum planes inline; port the exact math |
| BSP SAH cost function | Novel BSP heuristic | Port C++ SplitBSP surface area heuristic directly | Proven to work for Asciicker's instance distribution |
| Perlin noise | Custom implementation | `noise` crate or port C++ PerlinNoise class | Deferred to Phase 6 (water ripple); not needed for Phase 5 |
| Golden-file comparison | Custom diff tool | Simple cell-by-cell comparison function | `assert!((diff_count as f64 / total as f64) < 0.01)` |

**Key insight:** Phase 5 is primarily a PORTING exercise. The algorithms are well-documented in the C++ source. The challenge is correctly translating C++ patterns (pointer arithmetic, void* callbacks, global state) to Rust idioms (enums, closures, Resources) while preserving numerical equivalence.

## Common Pitfalls

### Pitfall 1: SampleBuffer Dimensions Mismatch
**What goes wrong:** The C++ SampleBuffer is `(2*width+4) * (2*height+4)` samples with a border. The current Rust SampleBuffer is `width * height` without border.
**Why it happens:** Phase 1 created a basic SampleBuffer stub. Phase 4 should have expanded it, but the exact dimensions must match C++ for correct rasterization.
**How to avoid:** Phase 4 must implement the correct buffer dimensions with +4 border before Phase 5 can use it. Verify in Phase 5 integration tests.
**Warning signs:** Off-by-one errors in resolve stage, edge pixels rendering incorrectly.

### Pitfall 2: Sample.visual Overloading (TRAP-R01)
**What goes wrong:** The `Sample.visual` field stores material indices for terrain (spare bit 3 = 0) but RGB555 direct color for meshes (spare bit 3 = 1). The resolve pass branches on `spare & 0x8`.
**Why it happens:** Dual-purpose field saves memory but requires careful bit checking.
**How to avoid:** Encode the terrain/mesh distinction in the Rust `Sample` type. Either use an enum for visual data or carefully port the spare bit layout.
**Warning signs:** Terrain renders with mesh colors or vice versa.

### Pitfall 3: Camera View Matrix Construction
**What goes wrong:** The C++ camera uses "architectural" perspective where sin30/cos30 are commented out, resulting in a horizontal-only view direction. The focal length is `max(dw,dh)*2`. Getting any of these wrong produces wildly different output.
**Why it happens:** The perspective mode is unusual -- it's not a standard game camera. It's a specialized architectural projection.
**How to avoid:** Port the exact C++ math from render.cpp:2966-3034. Verify with known camera positions that project to expected screen coordinates.
**Warning signs:** Scene appears at wrong scale, rotated incorrectly, or objects at wrong depth.

### Pitfall 4: HEIGHT_CELLS vs VISUAL_CELLS Confusion
**What goes wrong:** HEIGHT_CELLS=4 (5x5 vertices, 4x4 quads) but VISUAL_CELLS=8 (8x8 material grid). The scale factor is VISUAL_CELLS/HEIGHT_CELLS = 2. Mixing these up produces wrong vertex positions or wrong material lookups.
**Why it happens:** Two different grids overlay the same patch. The height grid is coarser than the visual grid.
**How to avoid:** Define constants clearly. The terrain shader indexes `visual[u][v]` where u,v map from sample position, while height is indexed separately.
**Warning signs:** Terrain triangles are half-size or double-size; materials appear on wrong triangles.

### Pitfall 5: Frustum Plane Elimination Must Be Non-Destructive
**What goes wrong:** The C++ frustum culling modifies the plane array in-place (swap_remove). If the caller's plane array is shared across multiple query calls (terrain + world), the second call operates on a mutated plane set.
**Why it happens:** C++ copies plane pointers to a local array before calling recursive query. The local array is mutated but the original is preserved.
**How to avoid:** In Rust, clone the planes Vec before passing to each query function, or use a local copy inside the query.
**Warning signs:** World geometry culled that should be visible, or terrain visible that should be culled.

### Pitfall 6: Scene Shift x2 Factor (TRAP-R06)
**What goes wrong:** scene_shift values are multiplied by 2 in both the isometric transform (tm[12], tm[13]) and perspective (view_ofs). Forgetting the x2 produces half-amplitude screen shake.
**Why it happens:** Sample-buffer space is 2x supersampled relative to ASCII cell space.
**How to avoid:** Apply the x2 factor wherever scene_shift is used. The C++ code does `scene_shift[0]*2` and `scene_shift[1]*2`.
**Warning signs:** Screen shake appears too subtle compared to C++ reference.

### Pitfall 7: Deferred Sprite Sort Must Be Stable
**What goes wrong:** Sprites at equal distances render in unstable order, causing flickering.
**Why it happens:** C++ uses qsort (unstable). If two sprites have identical dist, their order is undefined.
**How to avoid:** Use Rust's `sort_by` (stable) or `sort_unstable_by`. For golden-file comparison, the exact sort order must match C++ for <1% diff.
**Warning signs:** Golden-file comparison fails on sprite-heavy scenes but passes on terrain-only scenes.

### Pitfall 8: Terrain Shadow Raycasting is Expensive
**What goes wrong:** UpdateTerrainDark raycasts from every visual cell center (64 per patch) toward the light direction, testing both terrain and world geometry. For large terrains, this is O(patches * 64 * raycast_cost).
**Why it happens:** Full shadow computation is a precomputation step, not per-frame.
**How to avoid:** Compute shadows once at load time (or when light changes), store in the 64-bit dark bitmask per patch. Do NOT recompute per frame.
**Warning signs:** Extremely slow scene loading if shadows computed per-frame by accident.

### Pitfall 9: Known C++ Bugs (TERRAIN-001 through TERRAIN-004)
**What goes wrong:** Porting C++ bugs into Rust code.
**Why it happens:** Copy-paste porting without reviewing known bug list.
**How to avoid:** Fix all 4 documented terrain bugs during porting:
  - TERRAIN-001 (terrain.cpp:613): `if(x)` should be `if(y)` in reverse patch lookup
  - TERRAIN-002 (terrain.cpp:805): `u < y` should be `u < v` in boundary check
  - TERRAIN-003 (terrain.cpp:1671): same fix as TERRAIN-002
  - TERRAIN-004 (terrain.cpp:480,492): verify `>` vs `>=` boundary intent
**Warning signs:** Terrain height interpolation errors at patch boundaries.

### Pitfall 10: Phase Dependencies Not Yet Implemented
**What goes wrong:** Phase 5 depends on Phase 3 (GPU output) and Phase 4 (CPU rasterizer) which are not yet started.
**Why it happens:** Phase 5 cannot be executed until Phases 3 and 4 are complete.
**How to avoid:** Phase 5 planning can proceed, but execution must wait. Plan should clearly document which Phase 4 outputs are consumed as inputs.
**Warning signs:** Attempting to implement Phase 5 code that references Phase 4 types that don't exist yet.

## Code Examples

### Terrain Quadtree Frustum Query
```rust
// Source: C++ terrain.cpp:1803-1904 (QueryTerrain with frustum planes)
pub fn query_terrain_frustum(
    node: &QuadNode,
    x: i32,
    y: i32,
    range: i32,
    planes: &mut Vec<[f64; 4]>,
    callback: &mut impl FnMut(&RuntimePatch, i32, i32),
) {
    let (lo, hi) = node.height_bounds();
    let bbox = [
        x as f64, (x + range) as f64,
        y as f64, (y + range) as f64,
        lo as f64, hi as f64,
    ];

    let mut i = 0;
    while i < planes.len() {
        let (neg, pos) = count_aabb_corners_vs_plane(&bbox, &planes[i]);
        if neg == 8 { return; }  // entirely outside
        if pos == 8 {
            planes.swap_remove(i);  // entirely inside this plane
            continue;
        }
        i += 1;
    }

    match node {
        QuadNode::Leaf(patch) => callback(patch, x, y),
        QuadNode::Interior { children, .. } => {
            let half = range / 2;
            let offsets = [(0, 0), (half, 0), (0, half), (half, half)];
            for (idx, &(dx, dy)) in offsets.iter().enumerate() {
                if let Some(child) = &children[idx] {
                    let mut child_planes = planes.clone();
                    query_terrain_frustum(child, x + dx, y + dy, half, &mut child_planes, callback);
                }
            }
        }
    }
}
```

### BSP Tree SAH Construction
```rust
// Source: C++ world.cpp:1340-1603 (SplitBSP)
pub fn build_bsp(instances: &mut [BspItem]) -> BspNode {
    if instances.len() == 1 {
        return BspNode::Inst {
            bbox: instances[0].bbox,
            instance: instances[0].id,
        };
    }

    // Try all 3 axes, pick best SAH cost
    let mut best_cost = f64::MAX;
    let mut best_axis = 0;
    let mut best_split = 0;

    for axis in 0..3 {
        instances.sort_by(|a, b| {
            let ca = (a.bbox[axis * 2] + a.bbox[axis * 2 + 1]) / 2.0;
            let cb = (b.bbox[axis * 2] + b.bbox[axis * 2 + 1]) / 2.0;
            ca.partial_cmp(&cb).unwrap_or(Ordering::Equal)
        });

        // Compute cumulative surface areas from left and right
        // Find split with minimum SAH cost
        for split in 1..instances.len() {
            let left_sa = compute_surface_area(&instances[..split]);
            let right_sa = compute_surface_area(&instances[split..]);
            let cost = left_sa * split as f64 + right_sa * (instances.len() - split) as f64;
            if cost < best_cost {
                best_cost = cost;
                best_axis = axis;
                best_split = split;
            }
        }
    }

    // Re-sort by best axis and split
    instances.sort_by(|a, b| {
        let ca = (a.bbox[best_axis * 2] + a.bbox[best_axis * 2 + 1]) / 2.0;
        let cb = (b.bbox[best_axis * 2] + b.bbox[best_axis * 2 + 1]) / 2.0;
        ca.partial_cmp(&cb).unwrap_or(Ordering::Equal)
    });

    let (left, right) = instances.split_at_mut(best_split);
    let left_node = build_bsp(left);
    let right_node = build_bsp(right);

    BspNode::Node {
        children: [Some(Box::new(left_node)), Some(Box::new(right_node))],
        bbox: combined_bbox(&instances),
        // ~~split_plane: 0.0~~  WRONG placeholder — see P5-074 FIX in 05-02-PLAN.md
        // Correct: split_plane = (items[best_split-1].centroid[best_axis] + items[best_split].centroid[best_axis]) / 2.0
        split_plane: (items[best_split - 1].centroid[best_axis] + items[best_split].centroid[best_axis]) / 2.0,
        split_axis: best_axis as u8,
    }
}
```

### Perspective Camera View Matrix
```rust
// Source: C++ render.cpp:2966-3034 (view matrix construction in Render())
pub fn update_camera(camera: &mut GameCamera, dw: f64, dh: f64) {
    let a = camera.yaw as f64 * std::f64::consts::PI / 180.0;
    let sinyaw = a.sin();
    let cosyaw = a.cos();

    let scale = dh / (dw.max(dh));
    let zoom = camera.zoom as f64 * scale;
    let sin30: f64 = 0.5;
    let ds = 2.0 * zoom / VISUAL_CELLS as f64;

    // Isometric-style view matrix (used for both modes)
    camera.view_tm[0] = cosyaw * ds;
    camera.view_tm[1] = -sinyaw * sin30 * ds;
    camera.view_tm[4] = sinyaw * ds;
    camera.view_tm[5] = cosyaw * sin30 * ds;
    camera.view_tm[8] = 0.0;
    camera.view_tm[9] = -ds;
    // ... translation incorporates pos * HEIGHT_CELLS + scene_shift * 2
    camera.view_tm[12] = dw / 2.0
        - (camera.pos[0] as f64 * camera.view_tm[0] * HEIGHT_CELLS as f64
        +  camera.pos[1] as f64 * camera.view_tm[4] * HEIGHT_CELLS as f64
        +  camera.pos[2] as f64 * camera.view_tm[8])
        + camera.scene_shift[0] as f64 * 2.0;
    camera.view_tm[13] = dh / 2.0
        - (camera.pos[0] as f64 * camera.view_tm[1] * HEIGHT_CELLS as f64
        +  camera.pos[1] as f64 * camera.view_tm[5] * HEIGHT_CELLS as f64
        +  camera.pos[2] as f64 * camera.view_tm[9])
        + camera.scene_shift[1] as f64 * 2.0;

    if camera.perspective {
        // "Architectural" perspective: horizontal-only view direction
        // sin30/cos30 are intentionally commented out in C++ for flat perspective
        camera.focal = dw.max(dh) as f32 * 2.0;
        camera.view_dir[0] = (-sinyaw * 1.0) as f32;  // * cos30
        camera.view_dir[1] = (cosyaw * 1.0) as f32;   // * cos30
        camera.view_dir[2] = 0.0;                      // -sin30

        camera.view_pos[0] = HEIGHT_CELLS as f32 * camera.pos[0] - camera.view_dir[0] * camera.focal;
        camera.view_pos[1] = HEIGHT_CELLS as f32 * camera.pos[1] - camera.view_dir[1] * camera.focal;
        camera.view_pos[2] = camera.pos[2];

        // Normalize view_dir by focal
        camera.view_dir[0] /= camera.focal;
        camera.view_dir[1] /= camera.focal;

        camera.view_ofs[0] = (dw / 2.0 + camera.scene_shift[0] as f64 * 2.0) as f32;
        camera.view_ofs[1] = (dh / 2.0 + camera.scene_shift[1] as f64 * 2.0) as f32;
    }
}
```

### Terrain Shadow (UpdateTerrainDark)
```rust
// ~~SUPERSEDED by Plan 05-06 Task 1~~
// Phase 5 signature: update_terrain_dark(terrain: &mut RuntimeTerrain, light_dir: [f64; 3])
// `world` parameter deferred to Phase 6. `for_each_patch_mut` approach
// replaced by two-pass index-based approach (P5-058 FIX borrow conflict).
//
// R6-002 FIX: THIS ENTIRE CODE BLOCK IS STALE. The function body below uses the OLD 3-parameter
// signature (with `world` parameter and raycast calls). See 05-06-PLAN.md Task 1 for the
// CORRECT 2-parameter signature: update_terrain_dark(terrain: &mut RuntimeTerrain, light_dir: [f64; 3])
//
// Source: C++ terrain.cpp:1714-1765 (DarkUpdater)
pub fn update_terrain_dark(
    terrain: &mut RuntimeTerrain,
    world: &RuntimeWorld,
    light_pos: [f32; 3],
) {
    let light_dir = [
        -light_pos[0] as f64,
        -light_pos[1] as f64,
        -light_pos[2] as f64 * HEIGHT_SCALE as f64,
    ];

    terrain.for_each_patch_mut(|patch, px, py| {
        let mut dark: u64 = 0;
        for v in 0..VISUAL_CELLS {
            for u in 0..VISUAL_CELLS {
                let coords = patch.sample_cell_center(u, v, px, py);
                let bit = 1u64 << (u + VISUAL_CELLS * v);

                // Test terrain self-shadowing
                if let Some(hit) = terrain.raycast(&coords, &light_dir) {
                    if hit[2] > coords[2] + HEIGHT_SCALE as f64 / 4.0 {
                        dark |= bit;
                        continue;
                    }
                }

                // Test world geometry shadowing
                if let Some(hit) = world.raycast(&coords, &light_dir) {
                    if hit[2] > coords[2] {
                        dark |= bit;
                        continue;
                    }
                }
                // Cell is lit (bit stays 0)
            }
        }
        patch.dark = dark;
    });
}
```

### Golden-File Comparison
```rust
// Compare two AnsiCell grids within tolerance
pub fn compare_ansi_grids(
    actual: &AsciiCellGrid,
    expected_fg: &[u8],
    expected_bk: &[u8],
    expected_gl: &[u8],
) -> (usize, usize, f64) {
    // Use `grid.width * grid.height` — `cells_count()` is not a confirmed method on AsciiCellGrid.
    let total = actual.width as usize * actual.height as usize;
    let mut diff = 0;

    for i in 0..total {
        let (gl, fg, bg) = (
            actual.char_indices[i] as u8,
            actual.fg_colors[i],
            actual.bg_colors[i],
        );
        // Compare fg palette index, bg palette index, glyph
        if gl != expected_gl[i]
            || fg_to_xterm256(fg) != expected_fg[i]
            || bg_to_xterm256(bg) != expected_bk[i]
        {
            diff += 1;
        }
    }

    let pct = diff as f64 / total as f64 * 100.0;
    (diff, total, pct)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Raw pointers for quadtree | Rust enum nodes with Option<Box> | New for port | Safe memory management, no null pointer bugs |
| Void* cookie callbacks | Closures with captured state | New for port | Type-safe, no casting errors |
| Global refl_mode flag | Parameter passed through pipeline | New for port | No hidden mutable global state |
| memset clear | Double-allocation memcpy (Phase 4) | Original C++ design | Faster clear; must be preserved in Rust |

**Deprecated/outdated:**
- The current Rust `SampleBuffer` (Phase 1 stub) lacks the C++ fields: `visual`, `diffuse`, `spare`, `height`. Phase 4 must expand it before Phase 5.
- The current `Sample` struct has `color_rgb555`, `glyph`, `material_id` -- but the C++ Sample has `visual` (overloaded), `diffuse`, `spare` (bitfield), `height` (depth). Phase 4 must reconcile these.

## Integration Dependencies (Critical)

Phase 5 consumes outputs from three prior phases. The planner MUST verify these are available:

### From Phase 2 (Complete)
- `A3dTerrain` with `Vec<TerrainPatch>` (patches with height, visual, diag data)
- `A3dWorld` with `Vec<WorldInstance>` (mesh/sprite/item instances)
- `MaterialTable` with `Vec<[[MatCell; 16]; 4]>` (256 materials)
- `AkmMesh` with vertices, faces, edges
- `A3dFileLoader` for Bevy async loading

### From Phase 3 (NOT YET STARTED)
- GPU render plugin that takes `AsciiCellGrid` and displays it in a Bevy window
- Extract/Prepare/Render pipeline with unconditional extraction
- Font atlas as PNG asset (CP437 16x16 glyph grid)

### From Phase 4 (NOT YET STARTED)
- `SampleBuffer` with correct C++ field layout (visual, diffuse, spare, height)
- `SampleBuffer` double-allocation clear
- `Rasterize<Sample, Shader>` triangle rasterizer with duck-typed shaders
- `Bresenham` line rasterizer
- Material system with auto_mat LUT (32KB)
- RGB555 to xterm-256 quantization
- RESOLVE stage (2x2 downsample to AnsiCell)

## ECS Design for Runtime Terrain and World

### Runtime Terrain Resource
The parsed `A3dTerrain` (flat list of patches) must be assembled into a quadtree at load time. This is a Bevy Resource, not individual entities, because the quadtree is a single spatial structure queried as a whole.

```rust
#[derive(Resource)]
pub struct RuntimeTerrain {
    root: Option<QuadNode>,
    level: i32,
    base_x: i32,
    base_y: i32,
    patch_count: usize,
}
```

### Runtime World Resource
The parsed `A3dWorld` instances must be:
1. Mesh instances: store transform matrices, link to loaded AkmMesh data
2. BSP tree built via RebuildWorld equivalent
3. Instance flags respected

```rust
#[derive(Resource)]
pub struct RuntimeWorld {
    bsp_root: Option<BspNode>,
    flat_list: Vec<RuntimeInstance>,  // non-tree instances
    meshes: Vec<LoadedMesh>,          // mesh geometry (from AKM files)
}
```

### Load Sequence (C++ game.cpp pattern)
```
1. Load A3D file (Bevy async) -> A3dTerrain + MaterialTable + A3dWorld
2. Build RuntimeTerrain quadtree from A3dTerrain patches
3. For each mesh referenced in A3dWorld instances:
   a. Load AKM file (Bevy async)
   b. Compute mesh bounding box from vertices
4. Create RuntimeInstance for each WorldInstance
5. Build BSP tree (RebuildWorld equivalent)
6. Compute terrain shadows (UpdateTerrainDark)
```

## Open Questions

1. **Phase 3/4 Interface Stability**
   - What we know: Phase 5 depends on types from Phase 3 (AsciiCellGrid GPU display) and Phase 4 (SampleBuffer expanded, rasterizer, materials, resolve).
   - What's unclear: Phase 3 and 4 plans don't exist yet. Their type signatures may change during implementation.
   - Recommendation: Phase 5 planning should document expected interfaces. Phase 3/4 plans should be constrained by Phase 5's needs.

2. **Golden-File Reference Data Source**
   - What we know: VIS-02 requires <1% cell difference against C++ reference output.
   - What's unclear: How to capture C++ reference output. The C++ engine would need to dump AnsiCell grid for a specific camera position/scene.
   - Recommendation: Build a small utility that compiles the C++ engine to dump AnsiCell output for `game_map_y8.a3d` at a fixed camera position. Store as golden file in test assets.

3. **Water Plane in Phase 5 vs Phase 6**
   - What we know: The water plane affects terrain clipping (HEIGHT_SCALE/8 tolerance) and the reflection stage. Phase 6 covers water rendering.
   - What's unclear: Should Phase 5 include basic water plane clipping without reflection, or defer entirely to Phase 6?
   - Recommendation: Include a `water: f32` parameter in the pipeline but skip the reflection stage (Stage 5). Terrain/world should clip to water plane for correctness, reflection rendering deferred to Phase 6.

4. **Sprite Loading for Deferred Blit**
   - What we know: REND-08 requires deferred sprite blit. Sprites referenced by world instances must be loaded (XP files).
   - What's unclear: The XP sprite atlas system (angles, frames, projections) needs runtime representation beyond the parsed XpSprite.
   - Recommendation: Create a RuntimeSprite type that pre-resolves the atlas indexing (frame * angles * 2 + angle * 2 + proj). XpSprite parser from Phase 2 provides the raw data.

## Sources

### Primary (HIGH confidence)
- `docs/skills/engine-render.md` - Rendering pipeline callgraph, data contracts, known traps
- `docs/skills/world-loading.md` - BSP/terrain data contracts, quadtree structure, instance management
- `docs/arch/render_cpp_part1.md` - Function-level render.cpp analysis (Rasterize, RenderFace, RenderPatch, RenderSprite)
- `docs/arch/render_cpp_part2.md` - Render() main function analysis, camera setup, projection/unprojection
- `docs/arch/terrain_cpp_part1.md` - Quadtree CRUD, height bounds propagation, frustum query
- `docs/arch/terrain_cpp_part2.md` - QueryTerrain frustum culling, UpdateTerrainDark, raycasting
- `docs/arch/world_cpp_part1.md` - BSP construction (SplitBSP), instance lifecycle, QueryWorld
- `docs/arch/world_cpp_part2.md` - HitWorld raycast dispatch, QueryWorld implementation

### Secondary (HIGH confidence)
- C++ source code cross-referenced: `render.cpp:2838-4412` (Render function), `terrain.cpp:1714-1765` (DarkUpdater), `world.cpp:3073-3192` (QueryWorld BSP traversal)
- Existing Rust codebase: `engine-port/src/` (Phase 1-2 output, defines types consumed by Phase 5)

### Tertiary (MEDIUM confidence)
- Known C++ bugs list from `PROJECT.md` (TERRAIN-001 through TERRAIN-004) - documented but exact fix intent needs verification against C++ code

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - No new dependencies needed; all algorithmic work
- Architecture: HIGH - C++ code thoroughly documented in skill packs and arch docs
- Terrain quadtree: HIGH - Full function-level analysis in terrain_cpp_part1.md and part2.md
- BSP tree: HIGH - Full analysis in world_cpp_part1.md; SplitBSP and QueryWorld documented
- Camera system: HIGH - render.cpp view matrix construction analyzed line-by-line
- Terrain shadows: HIGH - DarkUpdater callback fully documented with C++ source verification
- Pitfalls: HIGH - Comprehensive trap list from skill packs (TRAP-R01 through TRAP-R12, TRAP-W01 through TRAP-W12)
- Golden-file CI: MEDIUM - Approach clear but reference data capture method needs Phase 5 implementation

**Research date:** 2026-02-20
**Valid until:** 2026-03-20 (stable domain; C++ reference code is frozen)
