# asciiid.cpp Analysis: Part 2 (Lines 4001-8000)
# Generated: 2026-02-12
# Agent: Handoff/Fix

## Overview

This document provides complete function-level analysis of `asciiid.cpp` covering lines 4001-8000. This section of the editor codebase handles terrain automation, material management, mesh baking, editing brushes, directory scanning, world persistence, and the MCP (Model Context Protocol) command processor.

---

## Function Analyses

### `ApplyAutoMatElev` (asciiid.cpp:4055-4064)

**Signature:** `static void ApplyAutoMatElev(int mode, double slope_threshold, int height_threshold, bool overwrite)`
**Purpose:** Apply automatic material elevation bit assignment to all terrain patches based on slope or height thresholds.
**Called by:** `asciiid.cpp:8232` (ImGui UI command handler for Auto Material Elevation feature)
**Calls:** `QueryTerrain()`, `URDO_Open()`, `URDO_Close()`
**Globals read:** `terrain`
**Globals mutated:** None directly; writes terrain visual maps through callback
**Side effects:** Opens undo system, queries all terrain patches, modifies persistent world state by setting elevation bit (0x8000).
**Notes:** The elevation bit (bit 15) indicates which cells should be painted with specific "elevated" materials (e.g., snow on peaks).

### `ApplyAutoTexture` (asciiid.cpp:4154-4161)

**Signature:** `static void ApplyAutoTexture(int mode, double slope_th, int h_min, int h_max, int mat_id, bool overwrite)`
**Purpose:** Paint specific material ID on terrain cells matching slope or height band criteria.
**Called by:** `asciiid.cpp:8262` (ImGui "AUTO TEXTURE" panel Apply button)
**Calls:** `QueryTerrain()`, `URDO_Open()`, `URDO_Close()`
**Globals read:** `terrain`
**Globals mutated:** None directly; writes terrain visual maps through callback
**Side effects:** Modifies material IDs (lower 8 bits of visual cells) on terrain matching criteria.
**Notes:** Similar to `ApplyAutoMatElev` but assigns specific material IDs instead of elevation bits.

### `ClearMatElev` (asciiid.cpp:4163-4194)

**Signature:** `static void ClearMatElev()`
**Purpose:** Remove all elevation bit markings (bit 15) from terrain visual map.
**Called by:** `asciiid.cpp:8235` (ImGui "AUTO MAT ELEV" panel Clear button)
**Calls:** `QueryTerrain()`, `URDO_Open()`, `URDO_Close()`
**Globals read:** `terrain`
**Globals mutated:** None directly; clears bits in terrain memory
**Side effects:** Batch modification of all terrain patches; adds entry to undo stack.
**Notes:** Inverse of `ApplyAutoMatElev`.

### `RefreshMaterialUsage` (asciiid.cpp:4199-4275)

**Signature:** `static void RefreshMaterialUsage()`
**Purpose:** Scan all terrain and world meshes to determine which material IDs are currently in use.
**Called by:** `asciiid.cpp:4290` (in `GetOrAllocateMaterialID`), `asciiid.cpp:4766` (in `BakeMeshesToTerrain`)
**Calls:** `memset()`, `QueryTerrain()`, `CollectMeshInsts()`, `GetInstMesh()`, `QueryMesh()`, `malloc()`, `free()`, `printf()`
**Globals read:** `terrain`, `world`, `g_material_used` (array)
**Globals mutated:** `g_material_used[256]`, `g_material_used_ready`
**Side effects:** Performs full world scan; outputs used material list to stdout.
**Notes:** Expensive operation; result is cached in `g_material_used_ready`.

### `GetOrAllocateMaterialID` (asciiid.cpp:4277-4363)

**Signature:** `static uint8_t GetOrAllocateMaterialID(uint8_t rgb[3])`
**Purpose:** Find or allocate a material ID matching the requested RGB color using Euclidean distance.
**Called by:** `asciiid.cpp:4638` (in `MeshBake::Apply` callback)
**Calls:** `GetMaterialArr()`, `RefreshMaterialUsage()`, `std::min()`, `printf()`, `m[i].Update()`
**Globals read:** `g_material_used[256]`, `g_material_used_ready`
**Globals mutated:** `g_material_used[i]`, global material array state
**Side effects:** May allocate new material entry and generate lighting shade ramps.
**Notes:** Uses a distance squared threshold of 25.0 to determine if existing material is a match.

### `BakeMeshesToTerrain` (asciiid.cpp:4759-4782)

**Signature:** `static void BakeMeshesToTerrain(bool bake_height, bool bake_material, bool bake_vertex_colors, bool overwrite_height, bool overwrite_material, bool solid_only, double ray_top, uint8_t material_id)`
**Purpose:** Rasterize 3D mesh geometry into 2D terrain height map and material grid.
**Called by:** `asciiid.cpp:8421` (ImGui "MESH BAKE" panel Bake button), `asciiid.cpp:11361` (test call)
**Calls:** `RefreshMaterialUsage()`, `CollectMeshInsts()`, `QueryTerrain()`, `URDO_Open()`, `URDO_Close()`, `free()`
**Globals read:** `terrain`, `world`
**Globals mutated:** None directly; modifies terrain patches via callback
**Side effects:** Modifies terrain height and visual maps across all patches; creates undo record.
**Notes:** Converts Blender-exported .akm meshes to efficient 2.5D terrain representation.

### `ClearSelection` (asciiid.cpp:4784-4791)

**Signature:** `static void ClearSelection()`
**Purpose:** Remove `INST_SELECTED` flag from all mesh instances in world.
**Called by:** `asciiid.cpp:8433` (UI button), `asciiid.cpp:10030` (deselection on click)
**Calls:** `CollectMeshInsts()`, `GetInstFlags()`, `SetInstFlags()`, `free()`
**Globals read:** `world`
**Globals mutated:** None directly; modifies instance flags
**Side effects:** Deselects all meshes in the editor.
**Notes:** Simple bitwise flag clearing on all instances.

### `SelectArea` (asciiid.cpp:4800-4856)

**Signature:** `static void SelectArea(const double tm[16], ImVec2 p1, ImVec2 p2)`
**Purpose:** Select mesh instances within a 2D screen-space rectangle.
**Called by:** `asciiid.cpp:10031` (Marquee select drag complete)
**Calls:** `std::min()`, `std::max()`, `CollectMeshInsts()`, `ImGui::GetIO()`, `GetInstBBox()`, `Product()`, `SetInstFlags()`, `free()`
**Globals read:** `world`
**Globals mutated:** None directly; modifies instance flags
**Side effects:** Sets `INST_SELECTED` flag on multiple instances.
**Notes:** Performs 3D bbox projection to screen-space for selection test.

### `DeleteSelected` (asciiid.cpp:4858-4875)

**Signature:** `static void DeleteSelected()`
**Purpose:** Remove all mesh instances marked with `INST_SELECTED` flag.
**Called by:** `asciiid.cpp:8429` (ImGui "MESH INST" panel Delete Selected button)
**Calls:** `CollectMeshInsts()`, `GetInstFlags()`, `URDO_Open()`, `URDO_Delete()`, `URDO_Close()`, `free()`
**Globals read:** `world`, `selected_inst`, `drag_inst`
**Globals mutated:** `selected_inst`, `drag_inst`
**Side effects:** Removes objects from world; updates undo stack.
**Notes:** Clears global pointers if they refer to deleted objects.

### `DeleteAllMeshInsts` (asciiid.cpp:4877-4894)

**Signature:** `static void DeleteAllMeshInsts()`
**Purpose:** Remove all mesh instances from the world.
**Called by:** `asciiid.cpp:8426` (ImGui "MESH INST" panel Delete All button)
**Calls:** `CollectMeshInsts()`, `URDO_Open()`, `URDO_Delete()`, `URDO_Close()`, `free()`, `RebuildWorld()`
**Globals read:** `world`
**Globals mutated:** None directly; world state modified
**Side effects:** Complete removal of all meshes; rebuilds world BSP tree.
**Notes:** Used when starting fresh or after baking all meshes to terrain.

### `GatherCB` (asciiid.cpp:4942-4946)

**Signature:** `static void GatherCB(Patch* p, int x, int y, int view_flags, void* cookie)`
**Purpose:** Callback to populate `gather->patch[]` grid for Gaussian smoothing.
**Called by:** `QueryTerrain()` in `Stamp()` mode 2 (line 5102)
**Calls:** `gather->GetPatchIdx()`
**Globals read:** `gather`
**Globals mutated:** `gather->count`, `gather->patch[]`
**Side effects:** Populates spatial grid for multi-patch smoothing.
**Notes:** Part of the two-pass Gaussian terrain brush.

### `StampCB` (asciiid.cpp:4948-5036)

**Signature:** `static void StampCB(Patch* p, int x, int y, int view_flags, void* cookie)`
**Purpose:** Apply terrain height brush stroke with falloff (Gaussian/Square/Noise).
**Called by:** `QueryTerrain()` in `Stamp()` mode 1 (line 5068)
**Calls:** `GetTerrainLimits()`, `URDO_Patch()`, `GetTerrainHeightMap()`, `UpdateTerrainHeightMap()`, `std::max()`, `std::min()`, `sqrt()`, `cos()`, `fast_rand()`, `fmax()`, `printf()`
**Globals read:** `br_alpha`, `br_radius`, `HEIGHT_SCALE`, `br_limit`, `probe_z`, `brush_shape`
**Globals mutated:** None directly; modifies patch height data
**Side effects:** Records undo data; updates patch height values.
**Notes:** Core height editing logic with three different brush shapes.

### `Stamp` (asciiid.cpp:5053-5229)

**Signature:** `void Stamp(double x, double y)`
**Purpose:** Apply terrain height brush stroke at world position (x, y).
**Called by:** `asciiid.cpp:9508`, `9528`, `9816` (Mouse/LMB handlers)
**Calls:** `ImGui::GetIO()`, `URDO_Open()`, `QueryTerrain()`, `URDO_Close()`, `malloc()`, `memset()`, `free()`, `ceil()`, `floor()`, `sqrt()`, `exp()`, `round()`
**Globals read:** `terrain`, `br_alpha`, `br_radius`, `gather`, `brush_shape`, `br_limit`, `probe_z`
**Globals mutated:** `gather` (may reallocate)
**Side effects:** Modifies terrain elevation; handles both direct and smoothed brush modes.
**Notes:** Holding Shift triggers Mode 2 (true Gaussian smoothing via separable convolution).

### `Palettize` (asciiid.cpp:5237-5398)

**Signature:** `void Palettize(const uint8_t p[768])`
**Purpose:** Build RGB-to-palette mapping via GPU 3D texture rendering.
**Called by:** `asciiid.cpp:7498` (UI button), `asciiid.cpp:10954` (Initialization)
**Calls:** `free()`, `malloc()`, `a3dGetTime()`, various OpenGL functions, `printf()`
**Globals read:** `ipal`, `pal_tex`
**Globals mutated:** `ipal`, `pal_tex`
**Side effects:** Allocates large palette buffer; creates and executes GPU shader program.
**Notes:** Uses fragment shader to find nearest colors in a 16MB 3D lookup table.

### `FreeDir` (asciiid.cpp:5408-5417)

**Signature:** `void FreeDir(DirItem** dir)`
**Purpose:** Release memory for directory listing array and nodes.
**Called by:** Multiple locations in file dialog code (e.g., `asciiid.cpp:7523`)
**Calls:** `free()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Deallocates memory.
**Notes:** Safe cleanup for `AllocDir()` results.

### `AllocDir` (asciiid.cpp:5419-5480)

**Signature:** `int AllocDir(DirItem*** dir, DirItem** list = 0)`
**Purpose:** Scan current directory and allocate sorted array of entries.
**Called by:** `asciiid.cpp:7527`, `7541`, `7555`, `7964` (File dialog init)
**Calls:** `a3dListDir()`, `malloc()`, `strcmp()`, `qsort()`
**Globals read:** None
**Globals mutated:** None
**Side effects:** Allocates memory; performs filesystem I/O.
**Notes:** Sorts directories before files, then alphabetically.

### `SpriteScan` (asciiid.cpp:5485-5506)

**Signature:** `static bool SpriteScan(A3D_DirItem item, const char* name, void* cookie)`
**Purpose:** Load sprite callback for directory scanning.
**Called by:** `asciiid.cpp:5583` (New map), `asciiid.cpp:6743` (Sprite reload)
**Calls:** `snprintf()`, `LoadSprite()`, `InitSpritePrefs()`
**Globals read:** None
**Globals mutated:** None directly; global sprite list updated
**Side effects:** Loads .xp files from disk into memory.
**Notes:** Skips directories; only loads files with .xp extension.

### `MeshScan` (asciiid.cpp:5513-5550)

**Signature:** `static bool MeshScan(A3D_DirItem item, const char* name, void* cookie)`
**Purpose:** Load mesh callback for directory scanning.
**Called by:** `asciiid.cpp:5583`, `5984`
**Calls:** `snprintf()`, `GetFirstMesh()`, `GetNextMesh()`, `GetMeshName()`, `strcmp()`, `LoadMesh()`, `malloc()`, `memset()`, `SetMeshCookie()`
**Globals read:** `world`
**Globals mutated:** None directly; world mesh library updated
**Side effects:** Loads .akm files from meshes/ directory.
**Notes:** Deduplicates by mesh name to avoid redundant loads.

### `New` (asciiid.cpp:5558-5817)

**Signature:** `void New()`
**Purpose:** Create empty map with default or image-based terrain.
**Called by:** `asciiid.cpp:7563` (ImGui "VIEW" panel New button)
**Calls:** `GetFirstMesh()`, `GetNextMesh()`, `GetMeshCookie()`, `free()`, `URDO_Purge()`, `DeleteTerrain()`, `DeleteWorld()`, `CreateTerrain()`, `CreateWorld()`, `a3dListDir()`, `MeshScan()`, `RebuildWorld()`, `a3dLoadImage()`
**Globals read:** `world`, `terrain`, `base_path`
**Globals mutated:** `world`, `terrain`, `active_mesh`
**Side effects:** Resets editor state; clears undo stack; rescans mesh assets.
**Notes:** Optionally loads heightmap from `maps/new.png` if it exists.

### `TranslateMap` (asciiid.cpp:5818-5887)

**Signature:** `void TranslateMap(int delta_z, bool water_limit)`
**Purpose:** Translate all terrain heights and mesh instances by delta_z.
**Called by:** `asciiid.cpp:5995` (commented call), UI sliders (indirect)
**Calls:** `QueryTerrain()`, `QueryWorld()`, `RebuildWorld()`, `UpdateTerrainHeightMap()`
**Globals read:** `terrain`, `world`, `probe_z`
**Globals mutated:** None directly; world/terrain data updated
**Side effects:** Batch modification of world geometry; rebuilds spatial index.
**Notes:** `water_limit` allows selective translation of land vs water.

### `Load` (asciiid.cpp:5895-5996)

**Signature:** `void Load(const char* path)`
**Purpose:** Load .a3d map file (terrain, materials, world, enemies).
**Called by:** `asciiid.cpp:6670` (MCP), `7818`, `7917` (Dialogs), `11136` (Init)
**Calls:** `TermCloseAll()`, `URDO_Purge()`, `DeleteTerrain()`, `DeleteWorld()`, `fopen()`, `LoadTerrain()`, `LoadWorld()`, `UpdateMesh()`, `a3dListDir()`, `MeshScan()`, `RebuildWorld()`, `fclose()`
**Globals read:** `world`, `terrain`, `base_path`, `g_enable_enemies`
**Globals mutated:** `world`, `terrain`, `active_mesh`, material array
**Side effects:** Complete replacement of current map data; network terminals closed.
**Notes:** Reloads .akm mesh geometry from disk after loading scene graph.

### `json_mesh_cb` (asciiid.cpp:6011-6040)

**Signature:** `void json_mesh_cb(Inst* i, Mesh* m, double tm[16], void* cookie)`
**Purpose:** Export mesh instance as JSON object for MCP DUMP_MATRIX.
**Called by:** `QueryWorld()` in `DumpWorldJSON()` (line 6060)
**Calls:** `GetMeshName()`, `printf()`
**Globals read:** None
**Globals mutated:** `ctx->count`, `ctx->first`
**Side effects:** Outputs JSON text to stdout.
**Notes:** Decomposes transform matrix into position and scale.

### `json_sprite_cb` (asciiid.cpp:6042-6044)

**Signature:** `void json_sprite_cb(Inst* inst, Sprite* s, float pos[3], float yaw, int anim, int frame, int reps[4], void* cookie)`
**Purpose:** Placeholder for sprite JSON export (currently a stub).
**Called by:** `QueryWorld()` in `DumpWorldJSON()` (line 6060)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** None (currently no-op)
**Notes:** Future implementation point for sprite persistence in MCP.

### `DumpWorldJSON` (asciiid.cpp:6046-6069)

**Signature:** `void DumpWorldJSON()`
**Purpose:** Export world scene graph as JSON to stdout for MCP protocol.
**Called by:** `asciiid.cpp:6204` (MCP DUMP_MATRIX command)
**Calls:** `QueryWorld()`, `printf()`, `fflush()`
**Globals read:** `world`
**Globals mutated:** None
**Side effects:** Prints JSON text to stdout; flushes buffer.
**Notes:** Wrapped in `[MATRIX_START]` and `[MATRIX_END]` markers.

### `Base64Encode` (asciiid.cpp:6071-6120)

**Signature:** `int Base64Encode(unsigned char* data, int len, char* base64)`
**Purpose:** Encode binary data to Base64 string.
**Called by:** `asciiid.cpp:6184` (MCP RENDER command)
**Calls:** None
**Globals read:** None
**Globals mutated:** None
**Side effects:** Writes to provided `base64` output buffer.
**Notes:** Used for sending software-rendered ANSI buffers over text-based MCP.

### `ProcessMCPCommand` (asciiid.cpp:6127-6720)

**Signature:** `void ProcessMCPCommand(char* line)`
**Purpose:** Parse and execute MCP (Model Context Protocol) text commands.
**Called by:** `asciiid.cpp:6726` (main render loop)
**Calls:** `CreateRenderer()`, `Render()`, `DeleteRenderer()`, `Base64Encode()`, `DumpWorldJSON()`, `LoadMesh()`, `CreateInst()`, `RebuildWorld()`, `LoadSprite()`, `Load()`, `exit()`
**Globals read:** `terrain`, `world`, `active_sprite`, `weather`
**Globals mutated:** `active_sprite`, `pos_x`, `pos_y`, `pos_z`, `rot_yaw`, `weather`
**Side effects:** Executes engine operations via text interface; potentially exits process.
**Notes:** Core of the headless/automated testing infrastructure.

### `my_render` (asciiid.cpp:6721-8000)

**Signature:** `void my_render(A3D_WND* wnd)`
**Purpose:** Main ImGui frame loop for UI panels, terrain editing, and 3D rendering.
**Called by:** Platform event loop (e.g., `sdl.cpp` or `x11.cpp`)
**Calls:** `IsStdinReady()`, `ProcessMCPCommand()`, `FreeSprites()`, `a3dListDir()`, `SpriteScan()`, `ImGui::NewFrame()`, various UI widget functions
**Globals read:** `g_mcp_mode`, `reload_sprites_requested`, `mouse_queue_len`
**Globals mutated:** `g_Time`, `reload_sprites_requested`
**Side effects:** Processes input; renders entire editor UI; triggers MCP command handling.
**Notes:** Handles the F5 sprite reload hotkey and batches mouse events.

---

## Data Structures

### Struct: `HeightRaster` (asciiid.cpp:4378-4387)

**Fields:**
- `int x, y` — Patch-aligned origin
- `int w, h` — Raster dimensions
- `uint16_t* hmap` — Height buffer
- `uint16_t* vmap` — Visual/material buffer
**Used by:** `BakeMeshesToTerrain` rasterization process.
**Size notes:** Temporary context used during mesh-to-terrain conversion.

### Struct: `JsonContext` (asciiid.cpp:6006-6009)

**Fields:**
- `bool first` — Track first object for comma placement
- `int count` — Total objects exported
**Used by:** `DumpWorldJSON()` and associated callbacks.
**Size notes:** Trivial state tracker.

### Struct: `Gather` (asciiid.cpp:4897-4936)

**Fields:**
- `int x, y` — Patch-aligned origin
- `int count` — Number of queried patches
- `int size` — Grid size in patches
- `int* tmp_x, *tmp_y` — Separable Gaussian filter buffers
- `Patch* patch[1]` — Flexible array of patch pointers
**Used by:** `Stamp()` mode 2 (Gaussian smoothing).
**Size notes:** Dynamically allocated based on brush radius.

### Struct: `DirItem` (asciiid.cpp:5401-5406)

**Fields:**
- `A3D_DirItem item` — File type enum
- `DirItem* next` — Linked list pointer
- `char name[1]` — Flexible array for filename
**Used by:** `AllocDir()` and file dialog UI.
**Size notes:** Node in directory listing linked list.

### Struct: `SpriteWidget` (asciiid.cpp:6874-7108)

**Fields:**
- `ImRect rect` — Viewport bounds
**Used by:** ImGui custom widget for sprite preview.
**Size notes:** Minimal state; logic is in `draw_cb` static method.

### Struct: `MeshWidget` (asciiid.cpp:7110-7389)

**Fields:**
- `ImRect rect` — Viewport bounds
**Used by:** ImGui custom widget for mesh preview.
**Size notes:** Minimal state; handles complex fitting of mesh BBox to viewport.

---

## Global Variables (lines 4001-8000)

### Global: `g_material_used_ready` (asciiid.cpp:4196)

**Type:** `bool`
**Purpose:** Tracks if the material usage scan is currently valid.
**Initialized by:** `RefreshMaterialUsage()`
**Read by:** `GetOrAllocateMaterialID()`
**Written by:** `RefreshMaterialUsage()`
**Thread safety:** Single-thread-only (main UI thread)

### Global: `g_material_used[256]` (asciiid.cpp:4197)

**Type:** `bool[256]`
**Purpose:** Bitset of which material IDs are referenced by world geometry.
**Initialized by:** `RefreshMaterialUsage()`
**Read by:** `GetOrAllocateMaterialID()`
**Written by:** `RefreshMaterialUsage()`
**Thread safety:** Single-thread-only

### Global: `gather` (asciiid.cpp:4940)

**Type:** `Gather*`
**Purpose:** Cached patch grid for Gaussian terrain brush smoothing.
**Initialized by:** `Stamp()`
**Read by:** `Stamp()`, `GatherCB()`
**Written by:** `Stamp()`
**Thread safety:** Single-thread-only
