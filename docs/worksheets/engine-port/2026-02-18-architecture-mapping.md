> **STATUS: REFERENCE MATERIAL** — Written 2026-02-18 before the Bevy engine decision (D001, 2026-02-19). The Mage Core rendering analysis and ASCII architecture patterns described here remain valid as technical reference. Integration approach has been updated: Mage Core's 4-texture GPU rendering approach will be implemented within Bevy's render pipeline rather than as a standalone engine. See DECISION_LOG.md D001 for engine decision.

---
title: "Architecture Mapping: C++ to Rust"
type: research
status: REFERENCE
date: 2026-02-18
# blocked_by: docs/worksheets/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md (not applicable to Rust port)
---

> **Note:** This document was created during the C++ project's pipeline closeout phase. The blocking dependency on pipeline closeout is not applicable to the Rust port. Content remains valid as reference material.

# Architecture Mapping: C++ to Rust

## Status: REFERENCE (originally deferred in C++ project)

---

## Module Mapping Overview

| C++ Module | Lines | Rust Equivalent | Reuse Strategy |
|------------|-------|-----------------|----------------|
| `render.cpp` | ~4400 | `rasterizer.rs` | Port template patterns as traits |
| `sprite.cpp` | ~1200 | `xp_loader.rs` | New implementation, reference DATA-CONTRACT |
| `game.cpp` | ~10600 | `game/` module | Partial port, ECS refactor |
| `world.cpp` | ~5000 | `world_bsp.rs` | New implementation |
| `terrain.cpp` | ~1500 | `terrain.rs` | New implementation |
| `font1.cpp` | ~390 | `font.rs` | Port, adapt for mage-core Font trait |

---

## Component Analysis

### 1. Rendering System

#### Asciicker C++ Structure

**File:** `render.cpp`

```
Renderer (opaque struct)
├── SampleBuffer (2x supersampled)
│   ├── Sample { visual, diffuse, spare, height }
│   └── Dimensions: (2*w+4) x (2*h+4)
├── SpriteRenderBuf (deferred sprite queue)
├── Item/NPC sorting lists
└── Perlin noise generator

Functions:
├── Render() - main entry point
├── RenderPatch() - terrain callback
├── RenderSprite() - sprite callback
├── RenderMesh() - mesh callback
├── RenderFace() - triangle rasterization
├── Rasterize<Sample, Shader>() - template
└── Bresenham<Sample>() - line template
```

**Evidence:** `render.cpp:567-700` (Renderer struct), `render.cpp:404-557` (Rasterize template)

#### Mage Core Structure

**File:** `render.rs`

```
RenderState (internal struct)
├── WGPU surface, device, queue
├── Textures: fg_texture, bg_texture, chars_texture, font_texture
├── RenderPipeline
└── BindGroups

Functions:
├── new() - async setup
├── resize() - handle window resize
├── render() - submit GPU commands
└── images() - get mutable buffer access
```

**Evidence:** `render.rs:24-70` (RenderState struct), `render.rs:330-375` (render function)

#### Mapping Strategy

| C++ Concept | Rust Implementation | Complexity |
|-------------|---------------------|------------|
| `SampleBuffer` | `struct SampleBuffer { data: Vec<Sample>, w: usize, h: usize }` | Low |
| `Rasterize<Sample, Shader>` | `fn rasterize<S: Sample, H: Shader>(buf: &mut [S], ...)` | Medium |
| `Bresenham<Sample>` | `fn bresenham<S: Sample>(...)` | Low |
| `Renderer` | `struct Renderer { sample_buffer: SampleBuffer, sprites: Vec<SpriteRender>, ... }` | Medium |

### 2. Sprite/XP Loading System

#### Asciicker C++ Structure

**File:** `sprite.cpp`

```
Sprite (struct)
├── Frame[] atlas
│   ├── width, height
│   ├── ref[3] (origin)
│   ├── meta_xy[2] (attachment)
│   └── AnsiCell[] cell
├── Anim[] anim
├── projs, anims, frames, angles
└── proj_bbox[6]

LoadSprite(path, name, recolor)
├── Parse gzip header
├── Decompress with tinfl
├── Parse XP header (layers, width, height)
├── Extract layer pointers
├── Merge swoosh layers
├── Quantize colors to palette
└── Assemble atlas grid
```

**Evidence:** `sprite.cpp:55-90` (Sprite struct), `sprite.cpp:293-1191` (LoadSprite)

#### Mage Core Equivalent

**File:** `image.rs`

```
Image (struct)
├── width, height: u32
├── fore_image: Vec<u32>
├── back_image: Vec<u32>
└── text_image: Vec<u32>
```

**Evidence:** `image.rs:3-18`

#### Mapping Strategy

**New module required:** `xp_loader.rs`

```rust
// Proposed structure
pub struct XpSprite {
    pub atlas: Vec<XpFrame>,
    pub anims: Vec<Anim>,
    pub angles: usize,
    pub projs: usize,
}

pub struct XpFrame {
    pub width: usize,
    pub height: usize,
    pub origin: [i32; 3],
    pub cells: Vec<AnsiCell>,  // palette-indexed
}

pub fn load_xp(path: &Path, recolor: Option<&[u8]>) -> Result<XpSprite, XpError> {
    // 1. Read gzip
    // 2. Decompress
    // 3. Parse layers
    // 4. Build atlas
}
```

**Key differences:**
- Mage Core uses ABGR u32 for colors; XP stores palette indices
- Need conversion layer or palette-aware rendering

### 3. World/BSP System

#### Asciicker C++ Structure

**File:** `world.h`, `world.cpp`

```
World (opaque)
├── BSP tree nodes
│   ├── NODE (split plane)
│   ├── NODE_SHARE (shared child)
│   ├── LEAF (instance list)
│   └── INST (single instance)
├── Mesh library (linked list)
└── Instance lists

Inst variants:
├── MeshInst { mesh, tm[16], ... }
├── SpriteInst { sprite, pos[3], yaw, anim, frame, ... }
└── ItemInst { item, pos[3], yaw, ... }

QueryWorld(planes, cb) - BSP traversal with frustum culling
```

**Evidence:** `world.h:48-220`

#### Mapping Strategy

**New module required:** `world_bsp.rs`

```rust
pub enum BspNode {
    Split { plane: [f64; 4], front: Box<BspNode>, back: Box<BspNode> },
    Leaf { instances: Vec<InstId> },
}

pub struct World {
    bsp: Option<BspNode>,
    meshes: HashMap<MeshId, Mesh>,
    instances: HashMap<InstId, Instance>,
}

pub fn query_world<F>(world: &World, frustum: &[[f64; 4]], callback: F)
where F: FnMut(Instance)
{
    // BSP traversal with plane tests
}
```

### 4. Game Logic

#### Asciicker C++ Structure

**File:** `game.cpp` (~10600 lines)

```
Character system:
├── Human { pos[3], yaw, anim, frame, equipment, inventory, ... }
├── NPC_Human { Human + AI state }
└── Character { linked list node for multiplayer }

Equipment enums:
├── MOUNT: NONE, WOLF, BEE
├── ARMOR: NONE, REGULAR_ARMOR
├── HELMET, SHIELD, WEAPON

Game instance:
├── player, terrain, world pointers
├── Input handling (keyboard, mouse, gamepad)
├── UI rendering (HP bar, inventory, menus)
└── Networking integration
```

**Evidence:** `game.cpp:1-150` (header comment)

#### Mapping Strategy

**Recommended:** ECS architecture (Entity Component System)

```rust
// Use hecs or bevy_ecs
pub struct Position(pub [f32; 3]);
pub struct Yaw(pub f32);
pub struct Animation { pub anim: usize, pub frame: usize }
pub struct Equipment { pub mount: Mount, pub armor: Armor, ... }
pub struct Inventory { pub items: Vec<Item> }
pub struct AI { pub state: AiState }

// Systems
fn update_physics(query: Query<(&mut Position, &Yaw)>);
fn update_animations(query: Query<(&mut Animation, &Equipment)>);
fn process_input(input: Res<Input>, query: Query<&mut Position>);
```

---

## Reusable Mage Core Modules

### Direct Reuse (No Modification)

| Module | Purpose | Usage |
|--------|---------|-------|
| `app.rs` | App trait | Main game loop interface |
| `input.rs` | ShiftState | Keyboard modifier tracking |
| `error.rs` | MageError | Error types |
| `colour.rs` | Colour enum | Color constants |

### Adaptation Required

| Module | Changes Needed |
|--------|----------------|
| `config.rs` | Add XP font loading option |
| `image.rs` | Add palette-indexed variant |
| `present.rs` | Add depth-aware blit |

### Complete Rewrite

| Module | Reason |
|--------|--------|
| `render.rs` | GPU-only, need CPU rasterizer |

---

## New Rust Modules Required

| Module | Purpose | Estimated Lines |
|--------|---------|-----------------|
| `xp_loader.rs` | Parse .xp files | ~400 |
| `rasterizer.rs` | CPU triangle rasterizer | ~600 |
| `sample_buffer.rs` | 2x supersampled buffer | ~150 |
| `material.rs` | Shade tables | ~200 |
| `palette.rs` | xterm-256 handling | ~100 |
| `world_bsp.rs` | Spatial partitioning | ~500 |
| `terrain.rs` | Heightmap patches | ~300 |
| `font.rs` | CP437 subset support | ~200 |

**Total new code:** ~2450 lines (vs ~22500 lines in C++)

---

## Template to Trait Mapping

### C++ Template Pattern

```cpp
// render.cpp:404-557
template <typename Sample, typename Shader>
inline void Rasterize(Sample* buf, int w, int h, Shader* s, const int* v[3], bool dblsided)
{
    // Shader must implement: Blend(Sample*, float z, float bc[3])
    // Sample must implement: DepthTest_RO(float z)
}
```

### Rust Trait Equivalent

```rust
pub trait Sample {
    fn depth_test_ro(&self, z: f32) -> bool;
}

pub trait Shader {
    fn blend(&mut self, sample: &mut impl Sample, z: f32, bc: [f32; 3]);
}

pub fn rasterize<S: Sample, H: Shader>(
    buf: &mut [S],
    w: usize,
    h: usize,
    shader: &mut H,
    vertices: [[i32; 4]; 3],
    double_sided: bool,
) {
    // Same algorithm, trait-based polymorphism
}
```

---

## References

- Asciicker render.cpp: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/render.cpp`
- Asciicker sprite.cpp: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/sprite.cpp`
- Asciicker world.h: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/world.h`
- Asciicker game.cpp: `/Users/rikihernandez/Downloads/Aciicker-Y9-2/game.cpp`
- Mage Core render.rs: `/Users/r/Projects/ascii research/Mage-core/src/render.rs`
- Mage Core image.rs: `/Users/r/Projects/ascii research/Mage-core/src/image.rs`
- Mage Core app.rs: `/Users/r/Projects/ascii research/Mage-core/src/app.rs`
