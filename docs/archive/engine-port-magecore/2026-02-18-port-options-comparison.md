# Engine Port Research: Full Mage Engine Vision

> **Note:** This document was created during the C++ project's pipeline closeout phase. The blocking dependency on pipeline closeout is not applicable to the Rust port. Content remains valid as reference material.

**Date:** 2026-02-18
**Status:** REFERENCE (originally deferred in C++ project)
**Updated:** Added Alex Harri + MIX-Project research
**Originally aligned to:** `docs/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md` *(not applicable to Rust port)*

---

## Decision Rule (from original C++ Closeout Plan -- historical reference)

1. Run real verification FIRST (no redesign).
2. Only if failures are real, apply minimal targeted fix.
3. If gates pass as-is: ship, no redesign.
4. Architecture cleanup (including engine port) is frozen until gate evidence is complete.

---

## Executive Summary

**Updated Goal:** Build a **Full Mage Engine** in Rust that can:
1. Run Asciicker game mechanics (physics, networking, sprites)
2. Support 3D projects like MIX (ray-marching, non-Euclidean geometries)
3. Deliver high-quality ASCII rendering (Alex Harri's 6D shape-vector approach)

### Port Options (Updated)

| Option | Effort | ASCII Quality | 3D Support | Verdict |
|--------|--------|---------------|------------|---------|
| **Full Mage Engine (Rust)** | High (18-24 months) | Best (6D shape vectors) | Full (WGPU shaders) | **RECOMMENDED** |
| **Unreal + ASCII Plugin** | Medium (6-9 months) | Good (brightness-based) | Full | Faster but lower ASCII quality |
| **MIX-based (Three.js)** | Low (3-6 months) | Good (Alex Harri algo) | Full (WebGL) | Web-only |

**Recommendation:** Build **Full Mage Engine (Rust)** - combines best ASCII quality with native 3D capabilities, no runtime dependencies, and full control.

---

## Research: External ASCII Rendering Projects

### Alex Harri ASCII Renderer (TypeScript/WebGL)

**Source:** `/Users/r/Projects/ascii research/alexharri-ascii/`
**Article:** https://alexharri.com/blog/ascii-rendering

#### Key Innovation: 6D Shape Vectors

Most ASCII renderers map brightness to character density. Alex Harri's approach:
1. Model each character as a **shape vector** (ink in multiple cell regions)
2. Build 6D sampling vectors from source image (staggered circle positions)
3. **Nearest-neighbor matching** in vector space (not brightness space)

#### Technical Details

| Component | Implementation |
|-----------|---------------|
| **Sampling** | 6 circular regions per cell (staggered layout) |
| **Matching** | k-d tree for O(log n) nearest-neighbor in 6D |
| **Cache** | Quantized 5-bit keys for O(1) repeated lookups |
| **Contrast** | Global crunch + directional crunch (external samples) |
| **GPU Path** | WebGL2 multi-pass shaders for sampling + crunch |

#### Sampling Circle Positions (normalized [0,1] in cell)
```
(0.3, 0.23), (0.7, 0.18)   // Top row (staggered)
(0.3, 0.50), (0.7, 0.50)   // Middle row
(0.3, 0.82), (0.7, 0.77)   // Bottom row (staggered)
```

#### Contrast Enhancement (Two-Stage)

**Global Crunch:**
```
normalized = value / max_value
enhanced = pow(normalized, exponent)
result = enhanced * max_value
```

**Directional Crunch:**
- Uses **external sampling points** (outside cell boundary)
- Darkens components where nearby edges detected
- Reduces staircasing artifacts

#### Performance Optimizations

1. **k-d tree** for 6D nearest-neighbor (not brute force)
2. **Quantized cache** - 5 bits per component = 30-bit key
3. **GPU acceleration** - WebGL2 shaders move heavy work off CPU
4. **Observer pattern** - decouples sampling from rendering

#### Port to Rust (Mage)

| Feature | Complexity | Notes |
|---------|------------|-------|
| k-d tree in 6D | 1-2 days | Well-documented algorithm |
| Quantized cache | 1 day | Bit manipulation |
| Contrast shaders | 2-3 days | Port GLSL to WGSL |
| Shape vector generation | 1 day | Precompute alphabet |

---

### MIX-Project (Three.js / WebGL)

**Source:** `/Users/r/Projects/MIX-Project/`
**Website:** https://3-dimensional.space

#### Purpose: Non-Euclidean Geometry Rendering

Ray-marching engine for Thurston geometries:
- Hyperbolic (H³)
- Spherical (S³)
- Nil, Sol, SL(2,R), H²×E, S²×E

#### Architecture

| Module | Purpose |
|--------|---------|
| `core/renderers/` | BasicRenderer, VRRenderer, PathTracerRenderer |
| `core/cameras/` | VRCamera, FullDomCamera, PathTracerCamera |
| `core/shapes/` | BasicShape, AdvancedShape |
| `core/materials/` | Material, PTMaterial |
| `ascii/` | **AsciiPass** - post-processing effect |

#### AsciiPass Implementation

```javascript
// Two-step GPU pipeline:
// Step 1: Sample scene at 6 positions per cell, match to nearest character
// Step 2: Render glyphs from font atlas

class AsciiPass extends Pass {
  _samplingMaterial  // GLSL shader for sampling + matching
  _renderMaterial    // GLSL shader for glyph rendering
  _atlasTexture      // Runtime-generated font atlas (10×8 grid = 80 chars)
  _samplingTarget    // Low-res render target (1 pixel per cell)
}
```

#### AsciiSampling.glsl (Key Algorithm)

```glsl
// Sample lightness at 6 circle positions
float s[6];
for (int i = 0; i < 6; i++) {
    vec2 sampleUV = cellOrigin + uSamplePoints[i] * uCellSize / uSceneResolution;
    s[i] = dot(texture(tDiffuse, sampleUV).rgb, LUMA_WEIGHTS);
}

// Global crunch: normalize per-cell contrast
float maxVal = max(max(s[0], s[1]), max(max(s[2], s[3]), max(s[4], s[5])));
for (int i = 0; i < 6; i++) {
    s[i] = pow(s[i] / maxVal, uCrunchExponent) * maxVal;
}

// Brute-force match (80 chars) - could use k-d tree for more
vec4 svA = vec4(s[0], s[1], s[2], s[3]);
vec2 svB = vec2(s[4], s[5]);
float minDist = 1e10;
int bestIdx = 0;
for (int i = 0; i < 80; i++) {
    vec4 dA = svA - uCharVecA[i];
    vec2 dB = svB - uCharVecB[i];
    float dist = dot(dA, dA) + dot(dB, dB);
    if (dist < minDist) { minDist = dist; bestIdx = i; }
}
```

#### Key Features for Mage Port

| Feature | MIX Has | Mage Needs |
|---------|---------|------------|
| Ray-marching shaders | ✓ | Custom WGSL port |
| Multiple geometries | ✓ (8 Thurston) | Extensible system |
| VR support | ✓ (WebXR) | OpenXR via wgpu |
| Path tracer | ✓ | Optional future |
| ASCII post-effect | ✓ (Alex Harri algo) | **Core feature** |

---

### Rendering Quality Comparison

| Approach | Edge Quality | Motion | Complexity |
|----------|-------------|--------|------------|
| **Brightness mapping** (common) | Blurry | OK | Low |
| **Asciicker CPU rasterizer** | Sharp (2x supersample) | Smooth | High |
| **Alex Harri 6D vectors** | Very sharp | Smooth | Medium |
| **MIX AsciiPass** | Sharp (6D + crunch) | Smooth | Medium |

**Winner:** Alex Harri's 6D shape-vector approach provides the best edge quality while remaining GPU-accelerated.

---

## Asciicker Architecture Overview

### Subsystem Inventory

| Subsystem | Files | Lines | Complexity | Skill |
|-----------|-------|-------|------------|-------|
| **Rendering** | render.cpp, sprite.cpp, font1.cpp, rgba8.cpp | ~7,000 | High (CPU rasterizer) | engine-render |
| **World/Terrain** | world.cpp, terrain.cpp | ~9,100 | Medium (BSP+Quadtree) | world-loading |
| **Physics** | physics.cpp | ~2,350 | High (collision) | physics-system |
| **Game Logic** | game.cpp, inventory.cpp, enemygen.cpp | ~15,800 | High (tightly coupled) | game-mechanics |
| **Networking** | network.cpp, game_svr.cpp, game_web.cpp | ~6,700 | Medium (custom protocol) | networking |
| **Editor** | asciiid.cpp, urdo.cpp | ~12,300 | Medium (ImGui) | editor-asciiid |
| **Audio** | audio.cpp, stb_vorbis.cpp | ~42,000 | Low (mostly decoder) | — |
| **Input** | input.cpp, gamepad.cpp | ~2,700 | Medium (multi-platform) | — |
| **Platform** | mswin.cpp, sdl.cpp, x11.cpp | ~4,400 | Low (abstraction) | — |
| **Main Menu** | mainmenu.cpp | ~87,600 | Low (UI only) | — |
| **Game App** | game_app.cpp | ~117,300 | Low (glue) | — |

**Total:** ~308,000+ lines (including generated/large files)

### Core Dependencies

```
Game ─┬─ Physics ─┬─ Terrain (quadtree)
      │           └─ World (BSP)
      ├─ Renderer ──── Terrain + World + Sprites
      ├─ Network ───── Server/Client protocol
      ├─ Audio ─────── Ogg Vorbis
      └─ Input ─────── Keyboard/Mouse/Touch/Gamepad
```

### Unique Technical Features

1. **CPU Software Rasterizer** - 6-stage pipeline, 2x supersampling, no GPU dependency
2. **xterm-256 Palette** - RGB → 6x6x6 color cube quantization
3. **CP437 Glyphs** - Full DOS character set with block characters
4. **Material Shading** - 4×16 shade tables per material, precomputed
5. **Sprite Atlas** - Multi-angle, multi-frame, multi-projection packing
6. **BSP Tree** - Spatial partitioning for mesh queries
7. **Quadtree Terrain** - Patch-based heightfield with LOD
8. **PhysicsIO Pattern** - Decoupled input/output for physics

---

## Full Mage Engine Architecture (Target Design)

### Vision

Build a **comprehensive ASCII-capable 3D engine** in Rust that:
1. Supports Asciicker game mechanics
2. Supports MIX-style 3D ray-marching
3. Uses Alex Harri's high-quality ASCII rendering
4. Remains 100% safe Rust

### Module Architecture

```
mage-engine/
├── mage-core/           # Existing: GPU rendering, App trait
│   ├── render.rs        # 4-texture WGPU renderer
│   ├── app.rs           # tick()/present() interface
│   └── config.rs        # Font loading, window config
│
├── mage-ascii/          # NEW: High-quality ASCII rendering
│   ├── shape_vectors.rs # 6D character vectors
│   ├── kd_tree.rs       # Nearest-neighbor matching
│   ├── sampling.rs      # GPU sampling shaders (WGSL)
│   ├── contrast.rs      # Global + directional crunch
│   └── alphabet.rs      # Alphabet generation/loading
│
├── mage-3d/             # NEW: 3D rendering capabilities
│   ├── camera.rs        # 3D camera system
│   ├── raymarch.rs      # Ray-marching shaders
│   ├── geometry.rs      # Non-Euclidean support
│   └── scene.rs         # Scene graph
│
├── mage-physics/        # NEW: Physics system
│   ├── collision.rs     # Sphere-triangle collision
│   ├── toi_sweep.rs     # Time-of-impact algorithm
│   ├── bsp_tree.rs      # Spatial partitioning
│   └── quadtree.rs      # Terrain patches
│
├── mage-game/           # NEW: Game systems
│   ├── character.rs     # Entity state
│   ├── equipment.rs     # 5D sprite lookup
│   ├── inventory.rs     # Item management
│   └── ai.rs            # NPC behavior
│
├── mage-net/            # NEW: Networking
│   ├── protocol.rs      # Binary protocol
│   ├── tcp.rs           # TCP transport
│   └── websocket.rs     # WebSocket transport
│
└── mage-ui/             # NEW: User interface
    ├── egui_backend.rs  # egui integration
    └── editor.rs        # Map editor
```

### What Mage Core Has (Existing)

| Feature | Implementation | Quality |
|---------|---------------|---------|
| **GPU Rendering** | WGPU 4-texture shader | Production-ready |
| **App Trait** | tick()/present() interface | Clean design |
| **Font Loading** | PNG atlas 16×16 grid | Simple |
| **Input State** | Shift/Ctrl/Alt tracking | Minimal |
| **Window Management** | winit event loop | Cross-platform |
| **Safety** | 100% safe Rust | No unsafe blocks |

### What Full Mage Engine Needs (New)

| Feature | Source | Effort |
|---------|--------|--------|
| **6D Shape Vectors** | Alex Harri | 1 week |
| **k-d Tree Matching** | Alex Harri | 1 week |
| **Contrast Shaders (WGSL)** | Alex Harri | 1 week |
| **Ray-marching** | MIX-Project | 2-3 weeks |
| **Non-Euclidean Geometries** | MIX-Project | 3-4 weeks |
| **Physics (TOI sweep)** | Asciicker | 2-3 weeks |
| **BSP Tree** | Asciicker | 1-2 weeks |
| **Quadtree Terrain** | Asciicker | 1-2 weeks |
| **XP Sprite Loader** | Asciicker | 1 week |
| **Game Systems** | Asciicker | 4-6 weeks |
| **Networking** | Asciicker | 2-3 weeks |
| **UI/Editor** | egui | 3-4 weeks |

**Total:** 22-30 weeks (6-8 months) for full engine

---

## Unreal Engine Analysis

### What Unreal Has

| Feature | Implementation | Notes |
|---------|---------------|-------|
| **Rendering** | GPU (DX11/12, Vulkan, Metal) | Overkill for ASCII |
| **Physics** | PhysX/Chaos | Replace sphere collision |
| **Networking** | Replication, RPCs | Replace custom protocol |
| **Audio** | FMOD/Unreal Audio | Replace Ogg decoder |
| **Input** | Enhanced Input | Replace custom handling |
| **UI** | UMG/Slate | Replace ImGui |
| **ECS-lite** | Actors/Components | Replace struct-based |
| **Editor** | Full editor | Replace asciiid |
| **Cross-platform** | All platforms | Replace platform code |

### What Unreal Lacks (Custom)

| Feature | Workaround |
|---------|------------|
| **ASCII Rendering** | Custom shader + font texture |
| **xterm-256 Palette** | Texture LUT |
| **CP437 Glyphs** | Font texture atlas |
| **XP Format** | Custom importer plugin |
| **Material Shading** | Material system adaptation |
| **.a3d Format** | Custom import/export |

### Port Strategy (Unreal)

**Phase 1: ASCII Rendering Plugin (3 weeks)**
```
[ASCIIRenderer]    → Custom USceneProxy
[FontAtlas]        → 16×16 CP437 texture
[PaletteLUT]       → xterm-256 lookup
[CellBuffer]       → 2D array of char+color
```

**Phase 2: Asset Import (2 weeks)**
```
[XPImporter]       → Factory for .xp files
[A3DImporter]      → Factory for .a3d maps
[SpriteConverter]  → XP → UTexture2D
```

**Phase 3: Game Systems (4 weeks)**
```
[ACharacterBase]   → Pawn with ASCII sprite
[AEquipment]       → 5D lookup component
[AInventory]       → Item container
[AAIController]    → NPC behavior tree
```

**Phase 4: Networking (2 weeks)**
```
[Replication]      → Replace custom protocol
[GameState]        → Server state
[PlayerState]      → Client state
```

**Total:** 11 weeks (3 months) for core port

### Risks (Unreal)

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| ASCII rendering quality | Medium | High | Custom shader R&D |
| Performance overhead | Medium | Low | Unreal is heavily optimized |
| Learning curve | Low | Medium | Team experience varies |
| License cost | None | None | Royalty-free under $1M |
| Binary size | High | Low | Not a concern for desktop |

---

## Hybrid Approach (Not Recommended)

Keep C++ core, use Mage for rendering only via FFI.

**Problems:**
- FFI boundary complexity
- Memory ownership issues
- Two build systems
- Debugging nightmare

**Verdict:** Not worth the complexity.

---

## Comparison Matrix (Updated)

| Criterion | Full Mage (Rust) | Unreal + Plugin | MIX (Three.js) |
|-----------|------------------|-----------------|----------------|
| **ASCII quality** | Best (6D vectors + k-d tree) | Good (brightness) | Good (Alex Harri) |
| **3D ray-marching** | Full (WGSL shaders) | Full (HLSL) | Full (GLSL) |
| **Non-Euclidean** | Extensible | Custom shaders | Built-in |
| **Physics** | Port from Asciicker | PhysX/Chaos | External |
| **Networking** | Port from Asciicker | Replication | External |
| **VR support** | OpenXR via wgpu | Full | WebXR |
| **Binary size** | ~10-20 MB | ~500 MB+ | Web (no binary) |
| **Runtime deps** | None | Unreal runtime | Browser |
| **Port effort** | 6-8 months | 3-4 months | 2-3 months |
| **Long-term risk** | Medium (self-maintained) | Low (Epic) | Medium (web deps) |
| **Platform support** | Win/Mac/Linux/Web | All major | Browser only |
| **Safety guarantees** | 100% safe Rust | C++ core | JavaScript |

---

## Updated Verdict

### Recommendation: Build Full Mage Engine (Rust)

**Rationale:**
1. **Best ASCII quality** - Alex Harri's 6D shape-vector approach is proven superior
2. **3D capability** - WGPU supports full ray-marching and non-Euclidean geometries
3. **No runtime deps** - Single binary, no framework overhead
4. **Safety** - 100% safe Rust eliminates memory bugs
5. **Extensible** - Modular architecture allows selective feature adoption
6. **Cross-platform** - WGPU works on Windows/Mac/Linux/Web

### Phase Roadmap

**Phase 1: ASCII Core (4 weeks)**
- Port 6D shape vectors
- Implement k-d tree matching
- Port contrast shaders to WGSL
- Verify visual quality matches Alex Harri's output

**Phase 2: 3D Foundation (4 weeks)**
- Camera system
- Ray-marching shaders
- Basic scene graph
- Test with simple geometry

**Phase 3: Game Mechanics (8 weeks)**
- Physics (TOI sweep from Asciicker)
- BSP tree + quadtree
- Character/equipment systems
- Input handling

**Phase 4: Platform (4 weeks)**
- Networking (TCP/WebSocket)
- Audio (rodio)
- UI (egui)
- Editor basics

**Phase 5: Advanced (4+ weeks)**
- Non-Euclidean geometries
- VR support (OpenXR)
- Path tracing (optional)

**Total Core:** 20-24 weeks (5-6 months)
**Full Feature:** 28-32 weeks (7-8 months)

### Risk Mitigation

| Risk | Mitigation |
|------|------------|
| WGPU shader complexity | Start with Mage Core's working shaders |
| Physics accuracy | Port Asciicker's proven TOI algorithm |
| Performance | GPU-first design, benchmark early |
| Scope creep | Modular crates, optional features |

---

## Decision Framework (Updated)

### Choose Full Mage Engine If:
- You want the **best ASCII rendering quality**
- You need **3D + ray-marching** in the same engine
- Binary size and runtime deps matter
- You want 100% safe Rust guarantees
- You're building for desktop + potentially web

### Choose Unreal + ASCII Plugin If:
- Speed to market is critical
- Team knows Unreal well
- ASCII quality is "good enough"
- You need all of Unreal's features (physics, networking, VR)

### Choose MIX (Three.js) If:
- Web-only deployment is acceptable
- You want to leverage existing MIX codebase
- JavaScript/TypeScript team
- Non-Euclidean geometries are the main focus

---

## Next Steps

1. **Week 1:** Create proof-of-concept for each option
   - Rust: XP loader + single sprite render
   - Unreal: ASCII shader plugin

2. **Week 2:** Benchmark rendering quality
   - Compare visual output to original
   - Measure performance

3. **Week 3:** Decision checkpoint
   - Choose primary port target
   - Create detailed implementation plan

4. **Week 4+:** Full port execution

---

## References

### Skills (Asciicker)
- `engine-render.md` - CPU rasterizer pipeline
- `world-loading.md` - BSP + quadtree
- `physics-system/SKILL.md` - TOI collision
- `game-mechanics/SKILL.md` - Game logic
- `networking/SKILL.md` - Protocol
- `editor-asciiid.md` - Editor

### External Projects
| Project | Path | What to Study |
|---------|------|---------------|
| **Alex Harri ASCII** | `/Users/r/Projects/ascii research/alexharri-ascii/` | 6D vectors, k-d tree, contrast |
| **MIX-Project** | `/Users/r/Projects/MIX-Project/` | Ray-marching, non-Euclidean, AsciiPass |
| **Mage Core** | `/Users/r/Projects/ascii research/Mage-core/` | WGPU rendering, App trait |

### Key Files
```
# Alex Harri ASCII Renderer
alexharri-ascii/website_repo/website-master/src/components/AsciiScene/
├── AsciiScene.tsx              # Main component
├── sampling/gpu/shaders.ts     # WebGL2 shaders
├── sampling/cpu/               # CPU fallback
├── characterLookup/KdTree.ts   # k-d tree matching
└── alphabets/*.json            # Precomputed vectors

# MIX-Project
MIX-Project/src/
├── ascii/AsciiPass.js          # Post-processing effect
├── ascii/shaders/*.glsl        # Sampling + render shaders
├── core/renderers/             # VR, path tracer, etc.
└── geometries/                 # Non-Euclidean

# Mage Core
Mage-core/src/
├── app.rs                      # App trait
├── render.rs                   # 4-texture renderer
├── shader.wgsl                 # WGPU shader
└── config.rs                   # Font loading
```

### Plans
- `docs/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md` - Historical reference (original C++ project prerequisite, not applicable to Rust port)
