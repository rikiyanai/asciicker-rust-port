# Phase 2: Asset Parsers - Research

**Researched:** 2026-02-20
**Domain:** Binary asset format parsing, Bevy asset system integration
**Confidence:** HIGH

## Summary

Phase 2 ports four binary asset formats (.xp sprites, .a3d terrain, .a3d world, .akm meshes) from the C++ Asciicker engine to Rust, integrating them with Bevy 0.18's `AssetLoader` trait for async loading. The formats are well-documented in the C++ source and have been verified by direct binary inspection of test files.

The .a3d format is a **composite sequential binary** file containing three sections read in order from a single file handle: terrain patches (with AS3D magic header), a 131,072-byte material table (256 entries x 512 bytes each), and world instances (format-versioned, 3 variant types). The .xp format is gzip-compressed REXPaint with column-major cell data and game-specific layer semantics. The .akm format is standard ASCII PLY with Asciicker-specific conventions (collision alpha, freestyle marks).

**Primary recommendation:** Implement four separate `AssetLoader` implementations (`XpSpriteLoader`, `A3dTerrainLoader`, `A3dWorldLoader`, `AkmMeshLoader`) each producing a typed `Asset`. The .a3d composite file should be split into separate terrain/material/world assets during loading via `LoadContext::labeled_asset`. Use `flate2` for gzip decompression and hand-rolled binary parsing (no serde/binrw -- the formats are simple fixed-layout structs).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| ASSET-01 | XP sprite files load correctly (gzip, CP437, column-major, 3+ layers) | Full .xp binary format documented from sprite.cpp:293-1191. Gzip header parsing, XPCell struct (10 bytes packed), column-major layout, layer pointer arithmetic all traced. Use `flate2::read::GzDecoder` for decompression. |
| ASSET-02 | XP layer semantics preserved (L0=colorkey, L1=height, L2=visual, swoosh merge) | Layer semantics fully traced: L0 background color = transparency key, L1 glyph encodes height ('0'-'9'=0-9, 'A'-'Z'=10-35), L2 = visual data, L3+ swoosh merge with half-block glyphs (220-223) and cyan marker. All constants in sprite_constants.h. |
| ASSET-03 | A3D terrain loads correctly (AS3D magic 0x44335341, 188-byte FilePatch, HEIGHT_SCALE=16) | FileHeader (16 bytes) and FilePatch (188 bytes) structs verified by parsing real files. minimal_1x1.a3d: 1 patch at (0,0). minimal_2x2.a3d: 4 patches. game_map_y8.a3d: 9207 patches. All parse correctly. |
| ASSET-04 | A3D world loads correctly (format version detection, 3 instance variants, LoadWorld/UpdateMesh/RebuildWorld order) | Format version detection: first int32 negative = versioned format (negate for version, read second int for count). Three variants via mesh_id_len discriminant: >=0=mesh, -1=sprite, -2=item. format_version=1 adds story_id. game_map_y8_original: 1083 mesh + 48 sprite + 154 item instances confirmed. |
| ASSET-05 | AKM mesh files load correctly (Blender export format) | .akm = ASCII PLY format ("ply\nformat ascii 1.0\n..."). Flexible property parser (x/y/z/red/green/blue/alpha with unknown properties skipped). Face format: "3 v0 v1 v2 [visual]". Negative vertex count = freestyle (wireframe). 2-vertex faces = freestyle edges. Verified with Cube.akm (24 vertices, 12 faces). |
| ASSET-06 | Asset loaders integrate with Bevy AssetServer (async loading, Handle-based references) | Bevy 0.18 `AssetLoader` trait requires: `type Asset`, `type Settings`, `type Error`, `fn load(&self, reader, settings, load_context)`, `fn extensions()`. Register via `app.register_asset_loader(loader)`. Access via `asset_server.load::<MyAsset>("path")` returning `Handle<MyAsset>`. |
| ASSET-07 | Golden-file tests validate parser output against known C++ reference data | Test assets available: minimal_1x1.a3d (1 patch), minimal_2x2.a3d (4 patches), test_map_no_terrain.a3d (19 mesh instances), item-apple.xp (69 bytes), grid-water.xp (141 bytes), Cube.akm (24 verts/12 faces). Generate reference snapshots from Python parsing scripts. |
</phase_requirements>

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bevy | 0.18.0 | Asset system, `AssetLoader` trait, `Handle<T>`, `AssetServer` | Project foundation (D001) |
| flate2 | 1.x | Gzip decompression for .xp files | De facto Rust gzip/deflate crate. Pure Rust backend (miniz_oxide). High reputation. |
| thiserror | 2.0 | Error type derivation for loader errors | Already in Cargo.toml. Standard for error types. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| bytemuck | 1.x | Zero-copy casting of byte slices to packed structs (FilePatch, FileHeader) | For fixed-layout binary structs (terrain patches, .a3d headers). Avoids manual byte-by-byte parsing. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| bytemuck | Manual `from_le_bytes()` calls | bytemuck is cleaner for fixed-layout structs; manual is fine for variable-length fields. Use both as appropriate. |
| Hand-rolled PLY parser | `ply-rs` crate | .akm is a restricted PLY subset (ASCII only, specific properties). Hand-rolling is ~100 lines and avoids pulling in a full PLY library with features we do not need. |
| `flate2` | `miniz_oxide` directly | flate2 wraps miniz_oxide with `std::io::Read` interface which is more ergonomic. No reason to go lower-level. |
| `binrw` / `nom` | Hand-rolled parsing | The formats are simple enough that a parsing framework adds more complexity than it removes. The variable-length world instance format with discriminant-based dispatch is naturally expressed as match arms. |

**Installation:**
```toml
[dependencies]
flate2 = "1.0"
bytemuck = { version = "1", features = ["derive"] }
```

## Architecture Patterns

### Recommended Project Structure

```
src/
  asset_loader/
    mod.rs              # AssetLoaderPlugin, re-exports
    xp_sprite.rs        # XpSprite asset type + XpSpriteLoader
    a3d_terrain.rs      # A3dTerrain asset type + A3dTerrainLoader
    a3d_world.rs        # A3dWorld asset type + A3dWorldLoader
    akm_mesh.rs         # AkmMesh asset type + AkmMeshLoader
    constants.rs        # Shared constants (HEIGHT_SCALE, VISUAL_CELLS, etc.)
    error.rs            # AssetError enum (shared across loaders)
tests/
  golden/               # Golden-file test data (small reference files)
    xp/                 # .xp test files + expected output snapshots
    a3d/                # .a3d test files + expected output snapshots
    akm/                # .akm test files + expected output snapshots
  asset_parsers.rs      # Golden-file integration tests
```

### Pattern 1: Bevy AssetLoader Implementation

**What:** Each binary format gets its own `Asset` type (with `#[derive(Asset)]`) and a corresponding `AssetLoader` implementation.

**When to use:** Every asset format.

**Example:**
```rust
// Source: Bevy 0.18 AssetLoader trait docs
use bevy::asset::{Asset, AssetLoader, LoadContext};
use bevy::asset::io::Reader;

#[derive(Asset, TypePath, Debug)]
pub struct XpSprite {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<XpLayer>,
    // ... parsed sprite data
}

#[derive(Default)]
pub struct XpSpriteLoader;

impl AssetLoader for XpSpriteLoader {
    type Asset = XpSprite;
    type Settings = ();
    type Error = AssetError;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> impl ConditionalSendFuture {
        async {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            parse_xp(&bytes)
        }
    }

    fn extensions(&self) -> &[&str] {
        &["xp"]
    }
}
```

### Pattern 2: Composite A3D File as Labeled Sub-Assets

**What:** The .a3d file contains terrain + materials + world sequentially. The loader produces a primary asset and labeled sub-assets accessible via `asset_server.load("map.a3d#terrain")`, `asset_server.load("map.a3d#world")`.

**When to use:** For the composite .a3d format that contains multiple logical assets.

**Example:**
```rust
// Primary asset holds references to sub-assets
#[derive(Asset, TypePath, Debug)]
pub struct A3dFile {
    pub terrain: Handle<A3dTerrain>,
    pub materials: Handle<MaterialTable>,
    pub world: Handle<A3dWorld>,
}

impl AssetLoader for A3dFileLoader {
    type Asset = A3dFile;
    // ...
    fn load(&self, reader: &mut dyn Reader, _: &(), ctx: &mut LoadContext<'_>)
        -> impl ConditionalSendFuture
    {
        async {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            // P2-R6 FIX: Corrected to use consumed-offset slicing pattern (see 02-04-PLAN.md Task 1)
            let (terrain_data, terrain_consumed) = parse_terrain_section(&bytes)?;
            let (materials, mat_consumed) = parse_material_section(&bytes[terrain_consumed..])?;
            let world_data = parse_world_section(&bytes[terrain_consumed + mat_consumed..])?;

            // Bevy 0.18: add_labeled_asset requires String, not &str
            let terrain_handle = ctx.add_labeled_asset("terrain".to_string(), terrain_data);
            let mat_handle = ctx.add_labeled_asset("materials".to_string(), materials);
            let world_handle = ctx.add_labeled_asset("world".to_string(), world_data);

            Ok(A3dFile {
                terrain: terrain_handle,
                materials: mat_handle,
                world: world_handle,
            })
        }
    }
}
```

### Pattern 3: Golden-File Snapshot Testing

**What:** Parse a known binary file, serialize the parsed result to a deterministic text format (JSON or custom), compare against a checked-in snapshot file.

**When to use:** All parser tests. This is how ASSET-07 is satisfied.

**Example:**
```rust
#[test]
fn test_parse_minimal_terrain() {
    let bytes = include_bytes!("../tests/golden/a3d/minimal_1x1.a3d");
    let (terrain, _consumed) = parse_terrain_section(bytes).unwrap();

    assert_eq!(terrain.patches.len(), 1);
    assert_eq!(terrain.patches[0].x, 0);
    assert_eq!(terrain.patches[0].y, 0);

    // Snapshot of first few height values
    let expected_heights: &[u16] = &[/* known values from C++ */];
    assert_eq!(&terrain.patches[0].height[..], expected_heights);
}
```

### Anti-Patterns to Avoid

- **Parsing in systems instead of loaders:** All parsing must happen in the `AssetLoader::load()` method, not in Bevy systems. Systems should only access already-parsed `Res<Assets<XpSprite>>` data.
- **Shared mutable state during loading:** The C++ engine uses global linked lists for sprites and meshes. The Rust port must NOT replicate this. Bevy's `Assets<T>` resource is the single owner.
- **Blocking I/O in async load:** Bevy's asset loading is async. Do NOT use `std::fs::File` inside loaders. Read all bytes from the provided `Reader`, then parse synchronously from the byte buffer.
- **Trying to load .akm files inside the .a3d world loader:** The C++ engine's LoadWorld creates empty mesh stubs, then the caller loads .akm geometry separately. The Rust port should follow the same pattern -- the A3dWorld asset stores mesh names as strings, and mesh loading happens separately.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Gzip decompression | Custom deflate implementation | `flate2::read::GzDecoder` | RFC 1952 compliance, edge cases in optional header fields (FEXTRA, FNAME, FCOMMENT, FHCRC), CRC validation |
| Byte-to-struct casting | Unsafe pointer arithmetic | `bytemuck::from_bytes::<FilePatch>()` | Alignment, endianness, padding safety. The C++ code does raw pointer casts that would be UB in Rust. |

**Key insight:** The binary format parsing itself IS the deliverable here -- there is no existing crate for .xp, .a3d, or .akm. But the decompression layer (gzip) and the byte-to-struct layer (bytemuck) have well-tested solutions.

## Common Pitfalls

### Pitfall 1: Column-Major vs Row-Major Cell Order in .xp
**What goes wrong:** Parsing cells in row-major order (x varies fastest within a row) when the .xp format stores cells in column-major order (iterating down each column before moving to the next).
**Why it happens:** Most raster formats are row-major. REXPaint is column-major.
**How to avoid:** The cell index formula is `cell_index = column * height + row`. In the C++ code: `layer0[x * height + y]` where x is column and y is row.
**Warning signs:** Sprite appears rotated 90 degrees or transposed in golden-file tests.

### Pitfall 2: Per-Layer Width/Height Gap in .xp
**What goes wrong:** Failing to skip the 8-byte (2 x int32) per-layer header between layer data blocks.
**Why it happens:** The decompressed XP payload has a global header (16 bytes: version + layers + width + height), then each layer's data is preceded by its own width+height pair (8 bytes). The C++ code navigates this with pointer arithmetic: `layer1 = (XPCell*)((int*)(layer0 + cells) + 2)`.
**How to avoid:** After reading `width * height * 10` bytes of cell data for layer N, skip 8 bytes before the next layer's cell data.
**Warning signs:** Layer 1 data looks like garbage; heights are all wrong.

### Pitfall 3: Format Version Ambiguity in World Section
**What goes wrong:** Interpreting the first int32 of the world section as an instance count when it is actually a negative format version.
**Why it happens:** Legacy files (pre-Y4) store the instance count directly. Newer files store a negative format version, then the count.
**How to avoid:** `if first_int < 0 { format_version = -first_int; count = read_next_i32(); } else { count = first_int; format_version = 0; }`. The format_version controls whether story_id is present per instance.
**Warning signs:** Reading negative number of instances, or misaligned reads after the header.

### Pitfall 4: .ply to .akm Extension Conversion
**What goes wrong:** Mesh names stored as "foo.ply" in old .a3d files but the actual file on disk is "foo.akm".
**Why it happens:** The C++ code has a hardcoded `strcpy(mesh_id+mesh_id_len-4,".akm")` conversion for ".ply" suffixes.
**How to avoid:** After reading mesh_id, check if it ends with ".ply" and replace with ".akm".
**Warning signs:** Mesh files not found during asset loading despite existing on disk.

### Pitfall 5: Material Table Size is Fixed 131,072 Bytes
**What goes wrong:** Assuming the material table size is variable or skipping it entirely.
**Why it happens:** The material table is read between terrain and world in the .a3d file. It is always 256 materials x 4 elevations x 16 diffuse levels x 8 bytes per MatCell = 131,072 bytes.
**How to avoid:** After reading terrain, read exactly 131,072 bytes for the material table before parsing the world section.
**Warning signs:** World parsing reads garbage because the file position is wrong.

### Pitfall 6: Swoosh Merge Only on Last Layer
**What goes wrong:** Applying swoosh merge logic (cyan fg + half-block glyphs) to all layers >= 3.
**Why it happens:** The code checks `m == layers - 1` for swoosh activation. Only the LAST layer triggers swoosh; intermediate layers (3 to N-2) are simple overwrites.
**How to avoid:** Track layer index during merge loop. Only apply swoosh logic when `layer_index == num_layers - 1`.
**Warning signs:** Extra swoosh effects on sprites, visual artifacts in golden-file comparison.

### Pitfall 7: Two Different Palette Quantization Formulas
**What goes wrong:** Using `RGB2PAL` formula `(c + 25) / 51` for sprite palette when `LoadSprite` uses `(c * 5 + 128) / 255` (projection) or `(c * 5 + 128) / 400` (reflection).
**Why it happens:** Two divergent quantization paths exist in the C++ code (TRAP-R03).
**How to avoid:** For sprite loading, use the LoadSprite inline formula: `level = (component * 5 + 128) / divisor` where divisor = 255 for projection frames and 400 for reflection frames. The RGB2PAL formula is for the RESOLVE stage only.
**Warning signs:** Colors slightly off in golden-file comparison; reflection frames too bright.

### Pitfall 8: Endianness Assumption
**What goes wrong:** Parsing .a3d on a big-endian platform produces garbage.
**Why it happens:** The C++ code assumes little-endian (TRAP-W12). Files are written with native byte order on x86.
**How to avoid:** Use `u32::from_le_bytes()`, `i32::from_le_bytes()`, etc. for all binary reads. Or use bytemuck with `#[repr(C)]` structs and validate on LE platforms only.
**Warning signs:** Magic number 0x44335341 does not match, patch coordinates are absurdly large.

## Code Examples

### XP File Decompression and Header Parsing

```rust
// Based on sprite.cpp:293-510
use flate2::read::GzDecoder;
use std::io::Read;

fn parse_xp(bytes: &[u8]) -> Result<XpSprite, AssetError> {
    // Decompress gzip
    let mut decoder = GzDecoder::new(bytes);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    // Parse header (16 bytes)
    let _version = i32::from_le_bytes(decompressed[0..4].try_into()?);
    let num_layers = i32::from_le_bytes(decompressed[4..8].try_into()?) as usize;
    let width = i32::from_le_bytes(decompressed[8..12].try_into()?) as usize;
    let height = i32::from_le_bytes(decompressed[12..16].try_into()?) as usize;

    if num_layers < 3 { return Err(AssetError::TooFewLayers(num_layers)); }
    if width < 1 || height < 1 { return Err(AssetError::InvalidDimensions(width, height)); }

    let cells = width * height;

    // Each layer: 8 bytes header (width+height, skipped) + cells * 10 bytes
    // Layer 0 starts at offset 16 (global header)
    // Layer N+1 starts at layer_N_start + 8 + cells * 10
    let layer_size = 8 + cells * 10; // 8 = per-layer width+height
    let layer0_offset = 16;  // after global 16-byte header
    // NOTE: layer0 data starts at offset 16, NOT 16+8, because the global
    // header already consumed the first width+height pair
    // Actually the C++ does: layer0 = (XPCell*)((int*)out + 4)
    // That's out + 16 bytes (4 ints), which skips the global header.
    // Then layer1 = (XPCell*)((int*)(layer0 + cells) + 2)
    // That's layer0_data_end + 8 bytes (2 ints for per-layer w/h)

    // Parse cells at each layer offset...
    // XPCell: 4 bytes glyph (u32) + 3 bytes fg + 3 bytes bk = 10 bytes
    // Column-major: for col in 0..width, for row in 0..height
    // cell_index = col * height + row

    Ok(XpSprite { /* ... */ })
}
```

### A3D Terrain Parsing

```rust
// Based on terrain.cpp:3083-3266
use bytemuck::{Pod, Zeroable};

const HEIGHT_CELLS: usize = 4;
const HEIGHT_CELLS_PLUS_ONE: usize = HEIGHT_CELLS + 1;
const VISUAL_CELLS: usize = 8;
const HEIGHT_SCALE: u16 = 16;

#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct FileHeader {
    file_sign: u32,     // 0x44335341 = "AS3D"
    header_size: u32,   // 16
    num_patches: u32,
    reserved: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct FilePatch {
    x: i32,
    y: i32,
    visual: [[u16; VISUAL_CELLS]; VISUAL_CELLS],  // 8x8 = 128 bytes
    height: [[u16; HEIGHT_CELLS_PLUS_ONE]; HEIGHT_CELLS_PLUS_ONE],  // 5x5 = 50 bytes
    diag: u16,
}
// static_assert: size_of::<FilePatch>() == 188

fn parse_terrain_section(data: &[u8]) -> Result<(A3dTerrain, usize), AssetError> {
    let header: &FileHeader = bytemuck::from_bytes(&data[0..16]);

    if header.file_sign != 0x44335341 { return Err(AssetError::BadMagic(header.file_sign)); }
    if header.header_size != 16 { return Err(AssetError::BadHeaderSize(header.header_size)); }

    let mut patches = Vec::with_capacity(header.num_patches as usize);
    for i in 0..header.num_patches as usize {
        let offset = 16 + i * 188;
        let patch: &FilePatch = bytemuck::from_bytes(&data[offset..offset + 188]);
        patches.push(TerrainPatch {
            x: patch.x,
            y: patch.y,
            visual: patch.visual,
            height: patch.height,
            diag: patch.diag,
        });
    }

    let consumed = 16 + header.num_patches as usize * 188;
    Ok((A3dTerrain { patches }, consumed))
}
```

### World Instance Parsing (3 Variants)

```rust
// Based on world.cpp:5008-5239
fn parse_world(data: &[u8]) -> Result<A3dWorld, AssetError> {
    let mut cursor = 0;

    let first_int = i32::from_le_bytes(data[cursor..cursor+4].try_into()?);
    cursor += 4;

    let (format_version, num_instances) = if first_int < 0 {
        let version = (-first_int) as u32;
        let count = i32::from_le_bytes(data[cursor..cursor+4].try_into()?);
        cursor += 4;
        (version, count as usize)
    } else {
        (0u32, first_int as usize)
    };

    let mut instances = Vec::with_capacity(num_instances);

    for _ in 0..num_instances {
        let mesh_id_len = i32::from_le_bytes(data[cursor..cursor+4].try_into()?);
        cursor += 4;

        match mesh_id_len {
            len if len >= 0 => {
                // Mesh instance
                let mesh_id = read_string(data, &mut cursor, len as usize);
                let inst_name = read_len_prefixed_string(data, &mut cursor);
                let tm = read_f64x16(data, &mut cursor);  // 128 bytes
                let flags = read_i32(data, &mut cursor);
                let story_id = if format_version > 0 { read_i32(data, &mut cursor) } else { -1 };

                // .ply -> .akm conversion
                let mesh_id = if mesh_id.ends_with(".ply") {
                    format!("{}.akm", &mesh_id[..mesh_id.len()-4])
                } else { mesh_id };

                instances.push(WorldInstance::Mesh { mesh_id, inst_name, tm, flags, story_id });
            }
            -1 => {
                // Sprite instance
                let sprite_name = read_len_prefixed_string(data, &mut cursor);
                let pos = read_f32x3(data, &mut cursor);
                let yaw = read_f32(data, &mut cursor);
                let anim = read_i32(data, &mut cursor);
                let frame = read_i32(data, &mut cursor);
                let reps = read_i32x4(data, &mut cursor);
                let flags = read_i32(data, &mut cursor);
                let story_id = if format_version > 0 { read_i32(data, &mut cursor) } else { -1 };

                instances.push(WorldInstance::Sprite { sprite_name, pos, yaw, anim, frame, reps, flags, story_id });
            }
            -2 => {
                // Item instance
                let item_proto_index = read_i32(data, &mut cursor);
                let count = read_i32(data, &mut cursor);
                let pos = read_f32x3(data, &mut cursor);
                let yaw = read_f32(data, &mut cursor);
                let flags = read_i32(data, &mut cursor);
                let story_id = if format_version > 0 { read_i32(data, &mut cursor) } else { -1 };

                instances.push(WorldInstance::Item { item_proto_index, count, pos, yaw, flags, story_id });
            }
            _ => return Err(AssetError::UnknownInstanceType(mesh_id_len)),
        }
    }

    Ok(A3dWorld { format_version, instances })
}
```

### AKM Mesh Parsing (ASCII PLY)

```rust
// Based on world.cpp:3619-4100
fn parse_akm(text: &str) -> Result<AkmMesh, AssetError> {
    let mut lines = text.lines().peekable();

    // Validate header
    let first = lines.next().ok_or(AssetError::EmptyFile)?;
    if first.trim() != "ply" { return Err(AssetError::NotPly); }

    let format = lines.next().ok_or(AssetError::Parse("missing format line".to_string()))?;
    if format.trim() != "format ascii 1.0" { return Err(AssetError::UnsupportedPlyFormat); }

    // Parse element/property declarations
    // Map properties: x=1, y=2, z=3, red/diffuse_red=4, green/diffuse_green=5,
    //                 blue/diffuse_blue=6, alpha=7, else=0(skip)
    let mut prop_types: Vec<u8> = Vec::new();
    let mut num_verts = 0usize;
    let mut num_faces = 0usize;
    let mut element = ' ';

    for line in &mut lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with("comment") { continue; }
        if line == "end_header" { break; }

        if let Some(rest) = line.strip_prefix("element vertex ") {
            num_verts = rest.parse()?;
            element = 'V';
        } else if let Some(rest) = line.strip_prefix("element face ") {
            num_faces = rest.parse()?;
            element = 'F';
        } else if let Some(rest) = line.strip_prefix("property ") {
            if element == 'V' {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    let prop_type = match parts[1] {
                        "x" => 1, "y" => 2, "z" => 3,
                        "red" | "diffuse_red" => 4,
                        "green" | "diffuse_green" => 5,
                        "blue" | "diffuse_blue" => 6,
                        "alpha" => 7,
                        _ => 0, // skip (normals, UVs, etc.)
                    };
                    prop_types.push(prop_type);
                }
            }
        }
    }

    // Parse vertices using property mapping...
    // Parse faces: "3 v0 v1 v2 [visual]" or "-3 v0 v1 v2 [visual]" (freestyle)
    // 2-vertex faces = freestyle edges

    Ok(AkmMesh { vertices, faces, edges })
    <!-- R9-001 FIX (LOW): Removed nonexistent `bbox` field from AkmMesh constructor. Actual struct has only vertices, faces, edges (see akm_mesh.rs:41-44). -->
}
```

## Binary Format Reference

### .a3d Composite File Layout

```
Offset    Size     Content
--------- -------- -------------------------------------------
0         16       FileHeader (terrain section)
                     u32 magic = 0x44335341 ("AS3D")
                     u32 header_size = 16
                     u32 num_patches
                     u32 reserved = 0
16        N*188    FilePatch[num_patches] (terrain data)
                   Per patch (188 bytes):
                     i32 x, y           (8 bytes)
                     u16 visual[8][8]   (128 bytes)
                     u16 height[5][5]   (50 bytes)
                     u16 diag           (2 bytes)
16+N*188  131072   Material table (mat[256])
                   Per material (512 bytes):
                     MatCell shade[4][16]  (4 elevations x 16 diffuse levels)
                   Per MatCell (8 bytes):
                     u8 fg[3], u8 gl, u8 bg[3], u8 flags
+131072   4        World format_version (i32, negative = versioned)
+4        4        World num_instances (i32, only if format_version < 0)
+8        variable Instance records (3 variants by mesh_id_len discriminant)
...       variable Enemy generator data (optional, after instances)
```

### Instance Record Variants (World Section)

**Mesh Instance** (mesh_id_len >= 0):
```
i32  mesh_id_len          -- string length (0 = empty mesh)
[u8] mesh_id              -- mesh_id_len bytes (no null terminator)
i32  inst_name_len        -- string length
[u8] inst_name            -- inst_name_len bytes
f64  tm[16]               -- 4x4 transform matrix (128 bytes, column-major)
i32  flags                -- INST_VISIBLE|INST_USE_TREE|INST_VOLATILE|INST_SELECTED
i32  story_id             -- only if format_version > 0
```

**Sprite Instance** (mesh_id_len == -1):
```
i32  inst_name_len        -- actually sprite name length
[u8] inst_name            -- sprite name (used to find loaded Sprite)
f32  pos[3]               -- world position (12 bytes)
f32  yaw                  -- rotation
i32  anim                 -- animation index
i32  frame                -- current frame
i32  reps[4]              -- palette remap indices (16 bytes)
i32  flags
i32  story_id             -- only if format_version > 0
```

**Item Instance** (mesh_id_len == -2):
```
i32  item_proto_index     -- index into item prototype library
i32  count                -- item stack count
f32  pos[3]               -- world position (12 bytes)
f32  yaw                  -- rotation
i32  flags
i32  story_id             -- only if format_version > 0
```

### .xp Sprite File Layout

```
[Standard gzip container (RFC 1952)]
  Decompressed payload (little-endian):
    i32  version           -- currently -1, skipped
    i32  num_layers        -- must be >= 3
    i32  width             -- applies to ALL layers
    i32  height            -- applies to ALL layers

    Per layer (num_layers times):
      i32  layer_width     -- per-layer (skipped, same as global)
      i32  layer_height    -- per-layer (skipped, same as global)
      Per cell (width * height, COLUMN-MAJOR order):
        u32  glyph         -- CP437 code point (0-255 valid range)
        u8   fg_r, fg_g, fg_b   -- foreground RGB888
        u8   bk_r, bk_g, bk_b   -- background RGB888
      Total: 10 bytes per cell
```

### .akm Mesh File Layout (ASCII PLY)

```
ply
format ascii 1.0
comment [optional comments]
element vertex N
property float x
property float y
property float z
[property float nx]        -- normals (skipped)
[property float ny]
[property float nz]
[property float s]         -- UVs (skipped)
[property float t]
property uchar red         -- or "diffuse_red"
property uchar green       -- or "diffuse_green"
property uchar blue        -- or "diffuse_blue"
property uchar alpha       -- collision weight (0=solid, 255=passthrough)
element face M
property list uchar uint vertex_indices
end_header
<x> <y> <z> [<nx> <ny> <nz>] [<s> <t>] <r> <g> <b> <a>    -- N vertex lines
<count> <v0> <v1> <v2> [<visual>]                            -- M face lines
  count = 3: normal triangle
  count = -3: freestyle (wireframe) triangle
  count = 2: freestyle edge (v0, v1 only)
```

## Sprite Constants Reference

From `sprite_constants.h` (C++ source of truth):

| Constant | Value | Purpose |
|----------|-------|---------|
| `SPRITE_MIN_LAYERS` | 3 | Minimum required layers for valid .xp |
| `SPRITE_SWOOSH_INDEX` | 254 | Palette index for swoosh marker |
| `SPRITE_TRANSPARENT_INDEX` | 255 | Palette index for transparency |
| `SPRITE_CYAN_R/G/B` | 0, 255, 255 | Swoosh marker color |
| `SPRITE_MAGENTA_R/G/B` | 255, 0, 255 | REXPaint transparency color |
| `SPRITE_GLYPH_FULL_BLOCK` | 219 | Full block character |
| `SPRITE_GLYPH_HALF_LOWER` | 220 | Lower half block |
| `SPRITE_GLYPH_HALF_LEFT` | 221 | Left half block |
| `SPRITE_GLYPH_HALF_RIGHT` | 222 | Right half block |
| `SPRITE_GLYPH_HALF_UPPER` | 223 | Upper half block |
| `SPRITE_MASK_LOWER` | 0x3 | Bottom two quadrants mask |
| `SPRITE_MASK_LEFT` | 0x5 | Left two quadrants mask |
| `SPRITE_MASK_RIGHT` | 0xA | Right two quadrants mask |
| `SPRITE_MASK_UPPER` | 0xC | Top two quadrants mask |
| `SPRITE_MASK_FULL` | 0xF | All four quadrants mask |
| `SPRITE_LIGHTEN_AMOUNT` | 51 | RGB increment for swoosh lightening |
| `SPRITE_HEIGHT_UNDEFINED` | 0xFF | Invalid height marker |

## Test Assets Available

| File | Size | Content | Golden Test Use |
|------|------|---------|-----------------|
| `a3d/minimal_1x1.a3d` | 131 KB | 1 terrain patch at (0,0), 0 instances | Basic terrain parser test |
| `a3d/minimal_2x2.a3d` | 132 KB | 4 terrain patches, 0 instances | Multi-patch terrain test |
| `a3d/test_map.a3d` | 132 KB | 0 terrain patches, 3 mesh instances | World parser with mesh instances |
| `a3d/test_map_no_terrain.a3d` | 131 KB | 0 terrain, 19 mesh instances (format v1) | Instance parsing with story_id |
| `a3d/game_map_y8_original_game_map.a3d` | large | 4876 patches, 1083 mesh + 48 sprite + 154 item instances | All 3 instance types |
| `sprites/item-apple.xp` | 69 B | Small item sprite | Minimal .xp test |
| `sprites/grid-water.xp` | 141 B | Small grid sprite | Basic .xp test |
| `sprites/wolfie.xp` | -- | Multi-angle animated sprite | Angles/animation test |
| `meshes/Cube.akm` | 2 KB | 24 verts, 12 faces, with normals+UVs | Property-skipping PLY test |

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Bevy `AssetLoader` with `BoxedFuture` | `AssetLoader` with `impl ConditionalSendFuture` | Bevy 0.18 | Return type changed from boxed future to impl trait. No Box allocation needed. |
| `AssetLoader::load` took `&'a [u8]` | Takes `&mut dyn Reader` | ~Bevy 0.12 | Must use async read, not direct slice access. Read all bytes first, then parse. |

**Deprecated/outdated:**
- Bevy pre-0.12 `AssetLoader` API used `&[u8]` directly -- current API uses `Reader` trait object.
- `TypeUuid` derive was replaced by `TypePath` and `Asset` derive macro.

## Open Questions

1. **Swoosh merge complexity vs. Phase 2 scope**
   - What we know: Full swoosh merge is complex (~200 lines of C++ with AverageGlyph, LightenColor, half-block mask logic). It requires porting helper functions (RGB2PAL, PAL2RGB, AverageGlyphTransp, LightenColor).
   - What's unclear: Should Phase 2 implement full swoosh merge, or defer to a later phase that needs rendered sprites?
   - Recommendation: Implement a simplified version that handles the common cases (simple overwrites, transparent cells) and mark full swoosh merge as a TODO. The golden-file tests for sprites should use simple sprites (item-apple, grid-water) that do not exercise swoosh paths. Full swoosh testing defers to Phase 5 integration.

2. **Material table as separate asset or embedded data?**
   - What we know: The material table is 131,072 bytes, always present between terrain and world in .a3d files. It is used by the RESOLVE stage (Phase 4+).
   - What's unclear: Whether to make it a separate `Asset` type or embed it in the terrain/world asset.
   - Recommendation: Make it a labeled sub-asset of the A3dFile (`"materials"` label). It has its own lifecycle and is consumed by the render system, not the world system.

3. **Enemy generator data at end of .a3d**
   - What we know: `LoadEnemyGens(f)` is called after `LoadWorld()` in the game loading sequence. There may be additional data after the world section.
   - What's unclear: The exact format of enemy generator data. It is not needed for Phase 2.
   - Recommendation: Ignore it. Read only terrain + materials + world. If extra bytes remain after world parsing, skip them silently.

4. **Atlas layout parsing scope for Phase 2**
   - What we know: The .xp loader in C++ not only parses raw cell data but also interprets layer 0 to determine angles, animations, frame grid layout, and reference points. This produces the final `Sprite` struct with its `atlas[]` array.
   - What's unclear: How much of the atlas assembly belongs in Phase 2 (asset parsing) vs. Phase 5 (rendering integration).
   - Recommendation: Phase 2 should parse raw layer data and store it in the `XpSprite` asset. Atlas assembly (frame subdivision, animation indexing, projection/reflection) should be a post-processing step that can be done in Phase 5 when the sprite rendering system needs it. The golden-file tests should verify raw layer data, not assembled atlas frames.

## Sources

### Primary (HIGH confidence)
- C++ source: `sprite.cpp:293-1191` -- Complete XP loading implementation
- C++ source: `sprite_constants.h` -- All sprite constants
- C++ source: `terrain.cpp:3083-3266` -- Complete terrain loading implementation (FileHeader, FilePatch)
- C++ source: `world.cpp:5008-5239` -- Complete world loading implementation
- C++ source: `world.cpp:3619-4100` -- Complete AKM/PLY mesh loading implementation
- C++ source: `world.cpp:4826-4917` -- SaveInst (instance serialization format)
- C++ source: `game_web.cpp:672-751` -- A3D composite loading sequence
- Binary verification: Parsed `minimal_1x1.a3d`, `minimal_2x2.a3d`, `test_map.a3d`, `test_map_no_terrain.a3d`, `game_map_y8_original_game_map.a3d` with Python struct module to confirm format
- Context7: `/websites/rs_bevy` -- Bevy 0.18 AssetLoader trait, AssetApp, Asset derive

### Secondary (MEDIUM confidence)
- C++ source: `io_asciicker/mesh/export_akm.py` -- AKM export conventions (collision alpha, freestyle marks, triangulation)
- `docs/codedoc-xp-terrain-format.md` -- Format documentation (verified against source)
- `docs/skills/engine-render.md` -- Render pipeline skill pack (cross-referenced)
- `docs/skills/world-loading.md` -- World/terrain skill pack (cross-referenced)
- Context7: `/rust-lang/flate2-rs` -- GzDecoder usage

### Tertiary (LOW confidence)
- None. All findings verified against C++ source code and binary file inspection.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- Bevy 0.18 asset system verified via Context7, flate2 is the standard Rust gzip crate
- Architecture: HIGH -- Four asset types with clear separation, composite file pattern well-understood from C++ source and binary verification
- Binary formats: HIGH -- All four formats traced through C++ source AND independently verified by parsing real binary files with Python
- Pitfalls: HIGH -- Every pitfall traced to specific C++ code lines and data contract documentation
- Golden tests: HIGH -- Test files identified and verified to parse correctly

**Research date:** 2026-02-20
**Valid until:** Indefinite (binary formats are stable, tied to existing C++ assets)
