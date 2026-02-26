# Batch Small B — Architecture Analysis
# Generated: 2026-02-12
# Agent: Handoff

Analysis of: `texheap.cpp`, `texheap.h`, `water.cpp`, `world_patch.cpp`, `input.cpp`, `fast_rand.h`, `stdafx.h`

---

## texheap.cpp — Texture Heap Allocator

### `TexHeap::Create` (texheap.cpp:31-44)

**Signature:** `void TexHeap::Create(int page_cap_x, int page_cap_y, int numtex, const TexDesc* texdesc, int page_user_bytes)`

**Purpose:** Creates texture heap with page dimensions, texture descriptions, and per-page user data bytes.

**Called by:** `InitSprite` (sprite.cpp) — sprite atlas initialization

**Calls:** `malloc` — allocates TexHeap structure and pages

**Globals read:** None

**Globals mutated:** None (allocates new heap, does not modify globals)

**Side effects:** Memory allocation (malloc)

**Notes:** Stores page_cap_x, page_cap_y, numtex, texdesc, page_user_bytes. Initializes cur_page to -1. Allocates this member and page pointers.

---

### `TexHeap::Destroy` (texheap.cpp:48-71)

**Signature:** `void TexHeap::Destroy()`

**Purpose:** Destroys texture heap and all allocated pages.

**Called by:** `FreeSprite` (sprite.cpp) — sprite atlas cleanup

**Calls:** `free` — deallocates pages and TexHeap structure

**Globals read:** None

**Globals mutated:** None

**Side effects:** Memory deallocation (free)

**Notes:** Loops through pages 0..MAX_PAGES, frees each, frees this pointer. Clears member variables.

---

### `TexAlloc::Update` (texheap.cpp:77-130)

**Signature:** `TexAlloc* TexHeap::Alloc(const TexData data[])`

**Purpose:** Allocates texture regions for multiple sprites using sequential bin-packing.

**Called by:** `LoadSprite` (sprite.cpp) — sprite allocation

**Calls:** None

**Globals read:** None

**Globals mutated:** `cur_page` — advances to next page when current page full, `page[page_idx]` — sets page fields

**Side effects:** Allocates TexAlloc from malloc. May cause page allocation.

**Notes:** Bin-packing algorithm: for each sprite in data[], check if fits on current page (sequential scan). If not, advance cur_page. Uses width/height from TexData. Returns TexAlloc struct with page index, region rect.

---

## texheap.h — Texture Heap Header

Header defines TexHeap class and TexAlloc struct. Implemented in texheap.cpp.

---

## water.cpp — Water Rendering

### `WaterRender` (water.cpp:9-69)

**Signature:** `void WaterRender(float* pos, float r, int level)`

**Purpose:** Renders water effect at position with radius and wave intensity level.

**Called by:** `Game::Render` (game.cpp) — during terrain rendering phase

**Calls:** None

**Globals read:** None

**Globals mutated:** None

**Side effects:** Draws to framebuffer

**Notes:** Creates multiple concentric circles with varying character and color based on level. Uses ASCII wave chars.

---

## world_patch.cpp — World Patch Operations

### `UpdatePatch` (world_patch.cpp:9-58)

**Signature:** `void UpdatePatch(Patch* p)`

**Purpose:** Updates patch visual state (material ID, alpha, light) from terrain data.

**Called by:** `Terrain::Update` (terrain.cpp) — terrain mesh update

**Calls:** `QueryTerrain` — reads terrain material, light, alpha values for patch region

**Globals read:** `terrain` — global terrain quadtree

**Globals mutated:** `p->matid`, `p->alpha`, `p->light` — patch visual attributes

**Side effects:** None

**Notes:** Samples terrain at patch vertices (4 corners + center). Converts material ID to visual representation. Updates patch for rendering.

---

## input.cpp — Input Handling Header

Header defines GAME_KEYB structure (key, shift, alt, ctrl, cap state). Functions declared in input.h.

---

## fast_rand.h — Fast Random Number Generator

### `fast_srand` (fast_rand.h:14)

**Signature:** `void fast_srand(unsigned int seed)`

**Purpose:** Seeds global fast_rand state.

**Called by:** `InitGame` (game.cpp) — game initialization

**Calls:** None

**Globals read:** None

**Globals mutated:** `fast_rand` — global RNG state

**Side effects:** None

---

### `fast_rand` (fast_rand.h:16)

**Signature:** `unsigned int fast_rand()`

**Purpose:** Returns next random number from fast LCG generator.

**Called by:** Throughout game code (random variations, procedural generation)

**Calls:** None

**Globals read:** `fast_rand` — global RNG state

**Globals mutated:** `fast_rand` — updates global state

**Side effects:** None

**Notes:** LCG: state = state * 214013 + 2531011. Return state >> 16.

---

## stdafx.h — Precompiled Header

Standard includes: stdio.h, stdlib.h, string.h, math.h, time.h, etc.
