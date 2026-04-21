# Architecture Research

**Domain:** Rust/Bevy ASCII game engine (C++ port of 82K-line Asciicker engine)
**Researched:** 2026-02-20
**Confidence:** MEDIUM (Bevy 0.18 render pipeline details are partially verified; core ECS patterns are HIGH confidence; Mage Core integration approach is HIGH from source inspection)

## Standard Architecture

### System Overview

```
+===================================================================+
|                       BEVY APP (Main World)                       |
+===================================================================+
|                                                                   |
|  +------------------+  +------------------+  +-----------------+  |
|  | AssetLoaderPlugin|  | GamePlugin       |  | InputPlugin     |  |
|  | (.xp, .a3d)     |  | (state machine,  |  | (keyboard,      |  |
|  |                  |  |  game loop)      |  |  gamepad, mouse)|  |
|  +--------+---------+  +--------+---------+  +--------+--------+  |
|           |                     |                     |           |
|  +--------v---------+  +-------v----------+  +-------v---------+  |
|  | WorldPlugin      |  | PhysicsPlugin    |  | CharacterPlugin |  |
|  | (BSP tree,       |  | (sphere sweep,   |  | (state machine, |  |
|  |  terrain quad,   |  |  TOI collision,  |  |  equipment,     |  |
|  |  instances)      |  |  gravity)        |  |  animation)     |  |
|  +--------+---------+  +--------+---------+  +--------+--------+  |
|           |                     |                     |           |
+===========|=====================|=====================|===========+
|           v                     v                     v           |
|  +---------------------------------------------------------------+|
|  |              CPU RASTERIZER PLUGIN (Main World)               ||
|  |  +----------+  +---------+  +--------+  +-------+  +-------+ ||
|  |  | CLEAR    |->| TERRAIN |->| WORLD  |->| SHADOW|->| REFL  | ||
|  |  +----------+  +---------+  +--------+  +-------+  +-------+ ||
|  |                                                     |         ||
|  |                              SampleBuffer (2x)      v         ||
|  |  +----------------------------------------------------+      ||
|  |  | RESOLVE: SampleBuffer -> AsciiCell grid            |      ||
|  |  | (auto_mat glyph/color, later: k-d tree shape match)|      ||
|  |  +----------------------------+-----------------------+      ||
|  +-------------------------------|-------------------------------+|
|                                  | AsciiCellGrid (Resource)      |
+==================================|================================+
                                   | Extract
+==================================v================================+
|                     BEVY RENDER WORLD                             |
|  +---------------------------------------------------------------+|
|  |            ASCII OUTPUT PLUGIN (Render World)                 ||
|  |                                                               ||
|  |  Prepare: upload 3 textures (char_idx, fg_rgba, bg_rgba)     ||
|  |  Queue:   bind group + fullscreen quad pipeline               ||
|  |  Render:  single fullscreen pass, WGSL shader samples         ||
|  |           font atlas to composite final pixels                ||
|  +---------------------------------------------------------------+|
+===================================================================+
```

### Component Responsibilities

| Component | Responsibility | Communicates With |
|-----------|----------------|-------------------|
| **AssetLoaderPlugin** | Parse .xp (gzip + CP437) and .a3d (binary BSP + terrain) into Bevy assets | WorldPlugin, CharacterPlugin (via Asset handles) |
| **WorldPlugin** | Manage BSP tree, mesh library, terrain quadtree, instance registry | CPU Rasterizer (provides scene data for queries), PhysicsPlugin |
| **TerrainSystem** (within WorldPlugin) | Quadtree heightmap patches (HEIGHT_CELLS=4, 5x5 vertex grid), frustum culling | CPU Rasterizer (QueryTerrain callback), PhysicsPlugin (height sampling) |
| **PhysicsPlugin** | Sphere-based collision (TOI sweep), gravity, face/edge/vertex tests | WorldPlugin (BSP queries), CharacterPlugin (position updates) |
| **CharacterPlugin** | State machine (idle/walk/run/attack/block/dead), 5D equipment sprite lookup, animation | PhysicsPlugin, CPU Rasterizer (sprite blit after RESOLVE) |
| **InputPlugin** | Keyboard/gamepad/mouse -> action mapping, camera Q/E rotation toggle | CharacterPlugin, CameraSystem |
| **CameraSystem** | Perspective camera with yaw, zoom, focal; scene_shift for UI sliding | CPU Rasterizer (provides view/projection matrices) |
| **CPU Rasterizer Plugin** | 6-stage pipeline (CLEAR->TERRAIN->WORLD->SHADOW->REFLECTION->RESOLVE), writes AsciiCellGrid | WorldPlugin (reads scene), ASCII Output Plugin (writes cell grid) |
| **ASCII Output Plugin** | GPU fullscreen pass: 4 textures (char index, fg, bg, font atlas) via WGSL shader | CPU Rasterizer (reads AsciiCellGrid), Bevy Render World |
| **AudioPlugin** | 16-track mixer via bevy_kira_audio, positional audio | GamePlugin (trigger events) |
| **NetworkPlugin** | Client-server multiplayer sync | GamePlugin (state sync), WorldPlugin (entity replication) |
| **GamePlugin** | Top-level state machine (MainMenu, Loading, Playing, Paused), orchestrates frame | All other plugins |

## Recommended Project Structure

```
src/
+-- main.rs                  # App::new(), plugin registration, window config
+-- lib.rs                   # Crate root, re-exports
+-- plugins/
|   +-- mod.rs               # Plugin module declarations
|   +-- game.rs              # GamePlugin: states, transitions, frame orchestration
|   +-- input.rs             # InputPlugin: action mapping, camera control
|   +-- audio.rs             # AudioPlugin: bevy_kira_audio 16-track wrapper
|   +-- network.rs           # NetworkPlugin: client-server sync (deferred)
+-- assets/
|   +-- mod.rs               # Asset type declarations
|   +-- xp_loader.rs         # .xp format: gzip decompress, CP437 parse, layer semantics
|   +-- a3d_loader.rs        # .a3d format: header, terrain patches, world instances, BSP
|   +-- font_loader.rs       # Font atlas (16x16 CP437 grid) -> GPU texture
+-- world/
|   +-- mod.rs               # WorldPlugin registration
|   +-- bsp.rs               # BSP tree: SAH construction, frustum query, ray intersection
|   +-- terrain.rs           # Quadtree: patch CRUD, heightmap, visual map, frustum query
|   +-- instance.rs          # Instance types: mesh, sprite, item (with flags)
|   +-- mesh.rs              # Mesh library: vertex data, face lists, bounding boxes
+-- physics/
|   +-- mod.rs               # PhysicsPlugin
|   +-- collision.rs         # Sphere sweep TOI, face/edge/vertex tests
|   +-- gravity.rs           # Gravity, terrain height sampling
+-- character/
|   +-- mod.rs               # CharacterPlugin
|   +-- state_machine.rs     # Character states + transitions
|   +-- equipment.rs         # 5D sprite lookup (weapon, armor, helmet, shield, color)
|   +-- animation.rs         # Animation timing, frame selection
+-- rendering/
|   +-- mod.rs               # CPU Rasterizer Plugin registration + system set config
|   +-- sample_buffer.rs     # SampleBuffer: 2x supersampled, Sample struct (height/visual/diffuse/spare)
|   +-- camera.rs            # View/projection matrix computation, frustum planes
|   +-- rasterize.rs         # Barycentric triangle rasterization, Bresenham lines
|   +-- terrain_shader.rs    # TerrainShader: height interpolation, material ID, diffuse
|   +-- mesh_renderer.rs     # Mesh transform + RenderFace dispatch
|   +-- sprite_renderer.rs   # Billboard sprite rendering into SampleBuffer
|   +-- shadow.rs            # Player blob shadow (darkens diffuse, converts terrain to auto-mat)
|   +-- reflection.rs        # Mirrored Z transform, reflected clip planes, water surface
|   +-- resolve.rs           # 2x2 downsample -> AsciiCell (glyph selection, auto_mat, silhouette)
|   +-- color.rs             # RGB555 pack/unpack, RGB888->xterm256 quantization, auto_mat cube
|   +-- material.rs          # Material/MatCell: shade[4][16] lookup tables
+-- gpu_output/
|   +-- mod.rs               # ASCII Output Plugin (Bevy render plugin)
|   +-- extract.rs           # Extract AsciiCellGrid from Main World -> Render World
|   +-- prepare.rs           # Create/update GPU textures (char_idx, fg, bg)
|   +-- node.rs              # Render graph node: fullscreen quad, bind groups, draw call
|   +-- shader.wgsl          # WGSL: sample font atlas by char index, mix fg/bg
|   +-- pipeline.rs          # Render pipeline descriptor, bind group layouts
```

### Structure Rationale

- **plugins/:** Top-level game orchestration. Thin wrappers that register systems and resources. Keeps `main.rs` clean.
- **assets/:** Isolated binary format parsers. These have zero dependencies on ECS logic; they transform bytes into typed Rust structs. Bevy's `AssetLoader` trait makes these testable independently.
- **world/:** Scene graph data structures (BSP, quadtree, instances). These are the "database" of the game. Heavy data, queried by both physics and rendering. Kept separate because they outlive any single frame.
- **physics/:** Collision and movement. Reads from world, writes to entity transforms. No rendering dependencies.
- **character/:** Game logic layer. State machines and animation are gameplay, not rendering.
- **rendering/:** The heart of the port: CPU rasterizer. This is the biggest module (~4400 lines in C++) and maps most directly to `render.cpp`. Split by pipeline stage for testability. Each stage reads SampleBuffer and writes SampleBuffer.
- **gpu_output/:** Bevy render plugin that lives in the Render World. Completely decoupled from CPU rasterization. It only needs the final AsciiCellGrid. This separation means the GPU output can be developed and tested independently.

## Architectural Patterns

### Pattern 1: Plugin-Per-Subsystem

**What:** Each major subsystem (world, physics, character, rendering, GPU output) is a Bevy Plugin that registers its own systems, resources, and system sets.

**When to use:** Always. This is Bevy's standard composition pattern.

**Trade-offs:** Adds boilerplate (one `impl Plugin for XPlugin` per subsystem) but provides clear boundaries, testability (add only the plugins you need in tests), and explicit dependency ordering via system sets.

**Example:**
```rust
pub struct CpuRasterizerPlugin;

impl Plugin for CpuRasterizerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SampleBuffer>()
            .init_resource::<AsciiCellGrid>()
            .configure_sets(
                Update,
                (
                    RasterizeSet::Clear,
                    RasterizeSet::Terrain,
                    RasterizeSet::World,
                    RasterizeSet::Shadow,
                    RasterizeSet::Reflection,
                    RasterizeSet::Resolve,
                )
                    .chain(),
            )
            .add_systems(Update, clear_sample_buffer.in_set(RasterizeSet::Clear))
            .add_systems(Update, rasterize_terrain.in_set(RasterizeSet::Terrain))
            .add_systems(Update, rasterize_meshes.in_set(RasterizeSet::World))
            .add_systems(Update, rasterize_sprites.in_set(RasterizeSet::World))
            .add_systems(Update, render_shadow.in_set(RasterizeSet::Shadow))
            .add_systems(Update, render_reflection.in_set(RasterizeSet::Reflection))
            .add_systems(Update, resolve_to_ascii.in_set(RasterizeSet::Resolve));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum RasterizeSet {
    Clear,
    Terrain,
    World,
    Shadow,
    Reflection,
    Resolve,
}
```

### Pattern 2: Resource as Frame Buffer (SampleBuffer + AsciiCellGrid)

**What:** The SampleBuffer and AsciiCellGrid are Bevy `Resource`s, not components on entities. They are singleton buffers that the CPU rasterizer writes and the GPU output reads.

**When to use:** For large, per-frame data that does not belong to any entity. The SampleBuffer is `(2*width+4) * (2*height+4)` Samples; the AsciiCellGrid is `width * height` AsciiCells. These are frame-scoped working buffers.

**Trade-offs:** Resources are globally accessible (any system can read/write them), so ordering via SystemSets is essential to prevent data races. The upside is zero entity query overhead and a single contiguous allocation.

**Example:**
```rust
#[derive(Resource)]
pub struct SampleBuffer {
    pub width: u32,
    pub height: u32,
    pub samples: Vec<Sample>,
}

#[derive(Resource)]
pub struct AsciiCellGrid {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<AsciiCell>,
}

// RESOLVE system reads SampleBuffer, writes AsciiCellGrid
fn resolve_to_ascii(
    sample_buf: Res<SampleBuffer>,
    mut cell_grid: ResMut<AsciiCellGrid>,
    materials: Res<MaterialTable>,
) {
    // Iterate 2x2 blocks, downsample, glyph select, color quantize
}
```

### Pattern 3: Extract-Prepare-Render for GPU Output

**What:** The ASCII Output Plugin follows Bevy's render world pattern: Extract copies the AsciiCellGrid from Main World into the Render World; Prepare uploads the 3 textures (char_idx, fg_rgba, bg_rgba) to the GPU via `queue.write_texture`; Render draws a fullscreen quad that samples all 4 textures (+ font atlas) in WGSL.

**When to use:** For the GPU output stage. This is the standard Bevy pattern for custom rendering.

**Trade-offs:** The Render World runs on a separate schedule, potentially pipelined with the next frame's Main World update. This means Extract must copy (not reference) the cell grid data. For a 1080p terminal at 8x16 font, that is ~135x67 = ~9K cells = ~36KB -- trivial copy cost.

**Example:**
```rust
pub struct AsciiOutputPlugin;

impl Plugin for AsciiOutputPlugin {
    fn build(&self, app: &mut App) {
        app.get_sub_app_mut(RenderApp)
            .expect("RenderApp required")
            .add_systems(ExtractSchedule, extract_ascii_grid)
            .add_systems(
                Render,
                prepare_ascii_textures.in_set(RenderSet::Prepare),
            );
        // Register render graph node
    }
}

fn extract_ascii_grid(
    mut commands: Commands,
    grid: Extract<Res<AsciiCellGrid>>,
) {
    commands.insert_resource(ExtractedAsciiGrid {
        width: grid.width,
        height: grid.height,
        cells: grid.cells.clone(),
    });
}
```

### Pattern 4: ECS Components for Scene Entities, Not Pipeline Data

**What:** Terrain patches, mesh instances, sprite instances, characters, and items are ECS entities with components. Pipeline-internal data (SampleBuffer, projection matrices, material tables) are Resources.

**When to use:** Always. This is the natural ECS split: things that exist in the game world are entities; things that exist per-frame or per-pipeline are resources.

**Trade-offs:** Terrain patches as entities means Bevy's change detection works for them (only re-query patches that moved or were added). Downside: many terrain patches (potentially thousands) mean entity iteration must be efficient -- use Bevy's query filters.

**Example:**
```rust
// Entity: exists in game world, has position, can be queried
#[derive(Component)]
pub struct TerrainPatch {
    pub x: i32,
    pub y: i32,
    pub visual: [[u16; 8]; 8],
    pub height: [[u16; 5]; 5],
    pub diag: u16,
}

// Entity: mesh instance in the BSP tree
#[derive(Component)]
pub struct MeshInstance {
    pub mesh: Handle<MeshAsset>,
    pub transform: Mat4,
    pub flags: u32,
}

// Resource: per-frame pipeline state, not an entity
#[derive(Resource)]
pub struct RenderContext {
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub clip_planes: [Vec4; 6],
    pub water_level: f32,
}
```

### Pattern 5: Custom Asset Types with AssetLoader

**What:** Implement Bevy's `AssetLoader` trait for .xp and .a3d formats. This integrates with Bevy's async asset loading, hot-reloading, and handle system.

**When to use:** For all binary format loading. Do not manually read files; use Bevy's asset server.

**Trade-offs:** Bevy's asset loader is async and reference-counted. Handles prevent use-after-free. The downside is that assets are not immediately available after `asset_server.load()` -- you must check `AssetEvent` or use states to wait for loading completion.

**Example:**
```rust
#[derive(Asset, TypePath)]
pub struct XpSprite {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<XpLayer>,
}

#[derive(Default)]
pub struct XpLoader;

impl AssetLoader for XpLoader {
    type Asset = XpSprite;
    type Settings = ();
    type Error = XpLoadError;

    async fn load(
        &self,
        reader: &mut dyn AssetReader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let decompressed = flate2::read::GzDecoder::new(&bytes[..]);
        // Parse header: version, num_layers, width, height
        // Parse layers column-major: glyph(u32) + fg(3xu8) + bg(3xu8) per cell
        Ok(parse_xp(&decompressed)?)
    }

    fn extensions(&self) -> &[&str] {
        &["xp"]
    }
}
```

## Data Flow

### Primary Render Pipeline Data Flow

```
[Game State (ECS)]
    |
    | Query terrain patches (frustum cull via quadtree)
    | Query world instances (frustum cull via BSP)
    | Read camera position, yaw, zoom, perspective flag
    v
[CPU Rasterizer Plugin - Main World Update]
    |
    | CLEAR: memcpy cached clear state -> SampleBuffer lower half
    | TERRAIN: TerrainShader writes Sample.height/visual(material_id)/diffuse
    | WORLD: RenderMesh/RenderSprite writes Sample.height/visual(RGB555)/diffuse
    | SHADOW: darkens diffuse around player position
    | REFLECTION: mirrored-Z re-query + rasterize, marks reflection samples
    | RESOLVE: 2x2 downsample -> AsciiCell (glyph from auto_mat/k-d tree,
    |          fg/bg from material table or auto_mat 32x32x32 cube)
    | SPRITES: deferred sprite blit onto AsciiCell grid (after RESOLVE)
    v
[AsciiCellGrid Resource]  -- fg(u8), bg(u8), glyph(u8), spare(u8) per cell
    |
    | Extract (copy to Render World)
    v
[Render World]
    |
    | Prepare: unpack AsciiCells into 3 GPU textures:
    |   - char_idx texture (R8, glyph code per cell)
    |   - fg_rgba texture  (RGBA8, palette-resolved foreground color)
    |   - bg_rgba texture  (RGBA8, palette-resolved background color)
    |   Upload via queue.write_texture()
    |
    | Render: fullscreen quad (4 vertices, triangle strip)
    |   WGSL shader:
    |     1. pixel position -> character cell coords (div by font_width/font_height)
    |     2. pixel position -> local coords within cell (mod by font_width/font_height)
    |     3. textureLoad(char_idx) -> glyph code -> font atlas coords (code%16, code/16)
    |     4. textureLoad(font_atlas, atlas_coords) -> alpha
    |     5. if alpha > 0.5: return fg_rgba, else: return bg_rgba
    v
[Screen Output]
```

### Asset Loading Data Flow

```
[.xp file on disk]
    |
    | AssetServer::load("sprites/player.xp")
    v
[XpLoader (async)]
    |
    | gzip decompress -> parse header (version, layers, w, h)
    | parse layers column-major: 10 bytes/cell (glyph u32 + fg RGB + bg RGB)
    | Layer semantics: L0=colorkey, L1=height, L2=visual, L3+=swoosh
    v
[XpSprite asset] -- Handle<XpSprite>
    |
    | Systems query Assets<XpSprite> to create sprite instances
    v
[SpriteInstance component on entity]


[.a3d file on disk]
    |
    | AssetServer::load("worlds/demo.a3d")
    v
[A3dLoader (async)]
    |
    | Terrain section: "AS3D" magic (0x44335341 LE), header(16B), patches(188B each)
    |   -> Vec<TerrainPatchData>
    | World section: format_version, instances
    |   mesh_id_len >= 0:  MeshInst (mesh name + double tm[16])
    |   mesh_id_len == -1: SpriteInst (sprite name + pos + yaw + anim)
    |   mesh_id_len == -2: ItemInst (item proto + count + pos + yaw)
    v
[A3dWorld asset] -- Handle<A3dWorld>
    |
    | Startup system spawns entities: TerrainPatch components, MeshInstance components
    | Builds BSP tree from mesh instances, quadtree from terrain patches
    v
[ECS entities in World]
```

### Key Data Flows

1. **Input -> Camera -> Rasterizer:** Keyboard/gamepad events update camera yaw/zoom/position. Camera system computes view/projection matrices and stores them in `RenderContext` resource. CPU rasterizer reads `RenderContext` for frustum culling and projection.

2. **Physics -> Character -> SpriteRenderer:** Physics updates character positions. Character state machine selects animation frame. Sprite renderer blits character sprites onto AsciiCellGrid after RESOLVE (deferred blit, not through SampleBuffer).

3. **SampleBuffer -> RESOLVE -> AsciiCellGrid:** The critical bridge. RESOLVE reads 2x2 sample blocks, computes elevation bucket (for glyph selection), diffuse level, decides material vs auto-mat path, applies silhouette detection, and writes final AsciiCell. This is where the Alex Harri k-d tree shape matching will later replace auto_mat glyph selection.

4. **AsciiCellGrid -> GPU textures -> Screen:** The Main World to Render World bridge. AsciiCellGrid is extracted (cloned) every frame. In the Render World, the 3 textures are updated via `queue.write_texture()`. The WGSL shader composites the final pixels using the font atlas.

## Scaling Considerations

| Concern | Current Target (1080p/60fps) | Stress Case (4K/60fps) | Mitigation |
|---------|------------------------------|------------------------|------------|
| SampleBuffer size | ~540K samples (270x135 cells, 2x = 540x270) | ~2M samples | Pre-allocated, reused across frames. memcpy clear from cached half. |
| Terrain patches | Hundreds visible | Thousands in world | Quadtree frustum culling limits visible set. Only visible patches enter rasterizer. |
| BSP query | ~100 visible instances | ~1000 in scene | SAH-constructed BSP. Frustum culling eliminates ~90% before rasterization. |
| GPU texture upload | 3 textures, ~36KB total per frame | 3 textures, ~500KB at 4K | `write_texture` is fast for these sizes. No bottleneck. |
| RESOLVE stage | ~9K cells | ~135K cells at 4K | CPU-bound. Profile first. Parallelizable per-cell with rayon if needed. |

### Scaling Priorities

1. **First bottleneck: CPU RESOLVE stage.** The 2x2 downsample + glyph selection + color quantization loop is the most compute-intensive per-frame operation. The C++ engine handles this in a tight loop. In Rust, use `itertools` or `rayon` for parallel cell processing if profiling shows this is the bottleneck.

2. **Second bottleneck: Terrain rasterization.** Many visible patches means many triangles through the barycentric rasterizer. The C++ engine uses template specialization for the inner loop. In Rust, use generic `Shader` trait with monomorphization (the compiler will specialize just like C++ templates).

## Anti-Patterns

### Anti-Pattern 1: Putting Pipeline Data in Components

**What people do:** Store SampleBuffer data on entities, or use components for per-pixel data.
**Why it is wrong:** SampleBuffer is a dense 2D array indexed by screen coordinates. Entity queries add overhead and destroy cache locality. There is no entity that "owns" a pixel.
**Do this instead:** Use `Resource` for SampleBuffer and AsciiCellGrid. Use components only for things that exist in the game world (terrain patches, characters, items).

### Anti-Pattern 2: Bypassing Bevy's Render World for GPU Operations

**What people do:** Access wgpu directly from Main World systems, call `queue.write_texture()` outside the Render schedule.
**Why it is wrong:** Bevy's Render World runs on a pipelined schedule. Accessing GPU resources from the Main World creates race conditions and breaks pipelining.
**Do this instead:** Use the Extract -> Prepare -> Queue -> Render pattern. Copy data in Extract, create GPU resources in Prepare, submit commands in Render.

### Anti-Pattern 3: One Giant Rasterizer System

**What people do:** Put the entire 6-stage pipeline in a single Bevy system function.
**Why it is wrong:** Bevy cannot parallelize within a single system. A monolithic rasterizer blocks all other systems for the entire frame. It is also untestable.
**Do this instead:** Split each pipeline stage into its own system, ordered via SystemSets with `.chain()`. Each stage is independently testable, and Bevy can schedule non-conflicting stages in parallel with other game systems.

### Anti-Pattern 4: Treating the Existing Skeleton as Foundation

**What people do:** Try to salvage the existing ~385 LOC skeleton that does not compile (4 missing modules, uses `Camera2dBundle::default()` which may be deprecated, has no tests).
**Why it is wrong:** The skeleton has structural issues: wildcard re-exports (`pub use *`), no plugin architecture, `Arc<Sprite>` where Bevy `Handle<XpSprite>` should be used, mutable statics for frame counting. Salvaging it means carrying forward misaligned patterns.
**Do this instead:** Start fresh with the plugin-per-subsystem architecture described here. Cherry-pick individual data structures (Sample, SampleBuffer basic layout, TerrainPatch fields) that are correct, but restructure the module graph.

### Anti-Pattern 5: Mixing Coordinate Systems

**What people do:** Forget that Asciicker uses Z-up (physics.h:41) while Bevy defaults to Y-up.
**Why it is wrong:** Mismatched coordinate systems cause inverted terrain, broken physics, and incorrect camera behavior. Every cross-system boundary becomes a bug source.
**Do this instead:** Define the project-wide convention as Z-up (matching the C++ engine). Apply a coordinate transform at Bevy integration boundaries (camera, Bevy Transform component). Document this transform once and enforce it.

## Integration Points

### Bevy Render Pipeline Integration

| Integration Point | Pattern | Notes |
|-------------------|---------|-------|
| **Main World -> Render World** | Extract system copies AsciiCellGrid | Runs in `ExtractSchedule`. Clones the grid data. ~36KB at 1080p, negligible cost. |
| **GPU Texture Upload** | `queue.write_texture()` in Prepare | 3 textures: char_idx (R8Unorm), fg (Rgba8Unorm), bg (Rgba8Unorm). Updated every frame. |
| **Fullscreen Quad** | Custom render graph node | No vertex buffer needed. Vertex shader generates 4 positions from `vertex_index`. Triangle strip. |
| **Font Atlas** | Loaded once as GPU texture | 16x16 grid of CP437 glyphs. Rgba8Unorm. Never changes after load. |
| **Shader Bindings** | 2 bind groups | Group 0: 4 textures (fg, bg, chars, font). Group 1: uniforms (font_width, font_height). |
| **Window Resize** | Recreate char_idx/fg/bg textures | On resize: recalculate grid dimensions (window_pixels / font_char_size), reallocate textures, update bind group. |

### Internal Module Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| **AssetLoader -> WorldPlugin** | Bevy asset handles + `AssetEvent` | World systems listen for `AssetEvent::LoadedWithDependencies` to spawn entities from loaded .a3d |
| **WorldPlugin -> CPU Rasterizer** | Query<> on terrain/instance entities + `RenderContext` resource | Rasterizer queries visible entities each frame. No direct function calls between modules. |
| **CPU Rasterizer -> GPU Output** | `AsciiCellGrid` resource (write in Main, read in Render) | The only data crossing the Main/Render world boundary. Clean interface: width, height, Vec<AsciiCell>. |
| **PhysicsPlugin -> WorldPlugin** | Reads BSP/terrain for collision, writes entity transforms | Physics queries the BSP tree and terrain heightmap. Updates Position components on entities. |
| **CharacterPlugin -> CPU Rasterizer** | Character entities have SpriteInstance components | Rasterizer's sprite blit stage reads sprite data from character entities. Deferred blit after RESOLVE. |

### Mage Core 4-Texture Mapping to Bevy

The Mage Core reference implementation (`../reference/Mage-core`) uses standalone wgpu. The mapping to Bevy's render world:

| Mage Core Concept | Bevy Equivalent |
|-------------------|-----------------|
| `RenderState.fg_texture` | GPU texture created in `Prepare` system set, stored as render world Resource |
| `RenderState.bg_texture` | Same pattern, separate texture |
| `RenderState.chars_texture` | Same pattern, char index texture |
| `RenderState.font_texture` | Loaded once in plugin init, stored as render world Resource |
| `RenderState.render()` | Custom render graph Node implementing `Node::run()` |
| `RenderState.images()` -> `&mut [u32]` slices | CPU-side Vec<u32> in Main World, uploaded via `write_texture()` in Prepare |
| `shader.wgsl` `fs_main` | Port directly; same WGSL logic works in Bevy's render pipeline |
| `create_texture_bind_group()` | Created in Prepare, stored as render world Resource |
| `RenderUniforms` (font_width, font_height) | Uniform buffer, created once, updated on resize |

The Mage Core shader is directly portable. The pixel logic (char coords, local coords, font atlas lookup, fg/bg selection) is identical. The only change is how the textures and uniforms are bound, which follows Bevy's render world patterns instead of standalone wgpu.

## Build Order (Dependencies Between Components)

The following order respects dependency chains -- each phase can only begin after its dependencies are complete:

```
Phase 1: Foundation
  +-- Asset loaders (.xp, .a3d) -- no dependencies
  +-- SampleBuffer + Sample data structures -- no dependencies
  +-- RGB555/xterm256 color utilities -- no dependencies
  +-- Material/MatCell data structures -- no dependencies
  These are pure data + parsing. Testable in isolation.

Phase 2: CPU Rasterizer Core
  +-- Camera/projection math (view matrix, frustum planes)
      Depends on: nothing (pure math)
  +-- Barycentric triangle rasterizer
      Depends on: SampleBuffer
  +-- Bresenham line rasterizer
      Depends on: SampleBuffer
  +-- TerrainShader (height, material_id, diffuse)
      Depends on: SampleBuffer, Material structs
  First visual output possible: rasterize hard-coded triangles.

Phase 3: World Data Structures
  +-- Terrain quadtree (patch storage, frustum query)
      Depends on: Asset loaders (.a3d terrain section)
  +-- BSP tree (instance storage, frustum query)
      Depends on: Asset loaders (.a3d world section)
  +-- Mesh library (vertex/face data)
      Depends on: Asset loaders (.a3d mesh format)
  Can load and query real game data.

Phase 4: GPU Output
  +-- ASCII Output Plugin (Render World)
      Depends on: AsciiCellGrid resource definition (Phase 1)
      Does NOT depend on CPU rasterizer -- can test with synthetic grid data.
  +-- Font atlas loading
      Depends on: nothing
  +-- WGSL shader (port from Mage Core)
      Depends on: nothing (pure shader code)
  First pixels on screen: colored glyphs from synthetic data.

Phase 5: Pipeline Integration
  +-- CLEAR + TERRAIN stages connected to quadtree queries
      Depends on: Phase 2 (rasterizer) + Phase 3 (terrain quadtree)
  +-- WORLD stage connected to BSP queries
      Depends on: Phase 2 (rasterizer) + Phase 3 (BSP tree)
  +-- RESOLVE stage (auto_mat glyph/color selection)
      Depends on: Phase 2 (SampleBuffer) + color utilities
  +-- Full pipeline: real .a3d data -> SampleBuffer -> RESOLVE -> AsciiCellGrid -> GPU
      Depends on: all of Phase 2, 3, 4
  First real scene rendered.

Phase 6: Advanced Features (can be parallelized)
  +-- Shadow pass
      Depends on: SampleBuffer, player entity
  +-- Reflection pass
      Depends on: SampleBuffer, water level, mirrored camera math
  +-- Deferred sprite blit
      Depends on: AsciiCellGrid, loaded .xp sprites
  +-- Physics (sphere sweep, gravity)
      Depends on: BSP tree, terrain heightmap
  +-- Character state machine
      Depends on: Physics, sprite assets
  +-- Alex Harri k-d tree shape matching (replaces auto_mat glyph selection)
      Depends on: RESOLVE stage working with auto_mat first

Phase 7: Polish & Systems
  +-- Audio (bevy_kira_audio)
  +-- Networking (client-server)
  +-- Weather effects
  +-- Main menu / game states
```

**Critical path:** Phase 1 -> Phase 2 -> Phase 5 (integrate) gives the shortest path to first real render. Phase 4 can proceed in parallel with Phases 2-3 because the GPU output only needs a synthetic AsciiCellGrid to test.

## Sources

- [Bevy Render Architecture Overview - Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/gpu/intro.html) -- Main World vs Render World, Extract pattern (MEDIUM confidence, verified against multiple sources)
- [Bevy Render Stages - Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/gpu/stages.html) -- RenderSet ordering: ExtractCommands, Prepare, Queue, Sort, Render, Cleanup (MEDIUM confidence)
- [Bevy Plugins - Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/programming/plugins.html) -- Plugin registration order, system set configuration (MEDIUM confidence)
- [Bevy System Sets - Unofficial Bevy Cheat Book](https://bevy-cheatbook.github.io/programming/system-sets.html) -- configure_sets, .chain(), ordering dependencies (MEDIUM confidence)
- [Bevy Custom Post Processing Render Pass](https://bevy.org/examples/shaders/custom-post-processing/) -- Custom render graph node pattern (MEDIUM confidence)
- [Bevy Custom Render Phase](https://bevy.org/examples/shaders/custom-render-phase/) -- Phase item pattern (MEDIUM confidence)
- [Render Pipeline Architecture - DeepWiki](https://deepwiki.com/bevyengine/bevy/5.1-render-pipeline-architecture) -- Pipeline specialization, feature keys (LOW confidence, unverified)
- [Bevy Asset System - DeepWiki](https://deepwiki.com/bevyengine/bevy/4-asset-system) -- AssetLoader trait, async loading, handles (LOW confidence)
- [Bevy 0.17 to 0.18 Migration Guide](https://bevy.org/learn/migration-guides/0-17-to-0-18/) -- Breaking changes in recent version (LOW confidence, not fully read)
- Mage Core source: `../reference/Mage-core/src/render.rs` -- 4-texture GPU approach with wgpu directly (HIGH confidence, read source code)
- Mage Core shader: `../reference/Mage-core/src/shader.wgsl` -- WGSL fullscreen quad with font atlas compositing (HIGH confidence, read source code)
- C++ render pipeline: `(ORIGINAL GAME)asciicker-Y9-2-main/docs/worksheets/RENDER_PIPELINE_DETAILED.md` -- 6-stage CPU pipeline stages (HIGH confidence, project documentation)
- C++ render pipeline map: `(ORIGINAL GAME)asciicker-Y9-2-main/docs/worksheets/RENDER_PIPELINE_MAP.md` -- Data structures, fragile invariants (HIGH confidence, project documentation)
- C++ engine render skill: `(ORIGINAL GAME)asciicker-Y9-2-main/docs/worksheets/skills/engine-render.md` -- Sample/AnsiCell/Material structs, traps (HIGH confidence, project documentation)
- C++ world loading skill: `(ORIGINAL GAME)asciicker-Y9-2-main/docs/worksheets/skills/world-loading.md` -- .a3d format, BSP/quadtree APIs (HIGH confidence, project documentation)

---
*Architecture research for: Rust/Bevy ASCII game engine port of Asciicker C++ engine*
*Researched: 2026-02-20*
