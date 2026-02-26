# Sprite Animation Timing Audit

**Source:** `/Users/r/Downloads/asciicker-Y9-2/`
**Date:** 2026-02-20
**Purpose:** Document sprite animation frame timing implementation for Rust port

---

## Summary

The original C++ codebase uses a **time-based animation system** with a unique 4-parameter timing model (`reps[4]`) that controls animation frame progression. There is **no `ms_per_frame` constant** - instead, animation timing is determined by a bit-shift operation on a timestamp.

---

## Key Findings

### 1. Animation Frame Timing Mechanism

**Location:** `world.cpp:5759-5796` - `AnimateSpriteInst()`

```cpp
int AnimateSpriteInst(Inst* i, uint64_t stamp)
{
    // ...
    time = (stamp >> 14) /*61.035 FPS*/ % len;
    
    if (time < si->reps[0])
        frame = 0;
    else if (time < si->reps[0] + si->reps[1] * sp->anim[anim].length)
        frame = (time - si->reps[0]) / si->reps[1];
    else if (time < si->reps[0] + si->reps[1] * sp->anim[anim].length + si->reps[2])
        frame = sp->anim[anim].length - 1;
    else
        frame = sp->anim[anim].length - 1 - (time - si->reps[0] - si->reps[1] * sp->anim[anim].length - si->reps[2]) / si->reps[3];
}
```

**Key insight:** The animation runs at approximately **61.035 FPS** (determined by `stamp >> 14`).

### 2. The `reps[4]` Parameter Model

**Location:** `world.cpp:520` - `SpriteInst` struct

```cpp
struct SpriteInst : Inst
{
    Sprite* sprite;
    int anim;
    int frame;
    int reps[4];    // Animation timing parameters
    float yaw;
    float pos[3];
};
```

#### `reps[4]` Semantics:

| Index | Name       | Purpose                                    |
|-------|------------|--------------------------------------------|
| `reps[0]` | hold_start | Initial hold time before animation starts  |
| `reps[1]` | frame_step | Milliseconds per frame (at 61 FPS)        |
| `reps[2]` | hold_mid   | Hold time at middle/end of animation      |
| `reps[3]` | reverse_step | Reverse playback step time             |

#### Animation Length Calculation:

```cpp
int len = si->reps[0] + si->reps[1] * sp->anim[anim].length 
        + si->reps[2] + si->reps[3] * sp->anim[anim].length;
```

This creates a timing model with:
1. **Initial delay** (`reps[0]`) - frames to wait before starting
2. **Forward playback** (`reps[1]` per frame) - time per frame in forward direction
3. **Middle hold** (`reps[2]`) - frames to hold at middle position
4. **Reverse playback** (`reps[3]` per frame) - time per frame in reverse direction

### 3. Common Usage Patterns

**Most common pattern:** `{ 0, 0, 0, 0 }` - no delays, no reverse

Found in `game.cpp` at multiple locations:
- Line 1884: `int reps[4] = { 0,0,0,0 };` (human NPC creation)
- Line 3808: `int reps[4] = { 0,0,0,0 };` (enemy creation)
- Line 3948, 4073, 4176, 6645: Same pattern

This means **most sprites use simple linear playback** with:
- No initial delay
- ~16.36ms per frame (at 61 FPS = 1 frame)
- No middle hold
- No reverse playback

### 4. No Global ms_per_frame Constant

**Finding:** There is **no centralized `ms_per_frame` constant** in the sprite system.

The timing is derived from:
```cpp
stamp >> 14  // Bit-shift divides timestamp by 2^14 = 16384
```

This assumes `stamp` is in **milliseconds**, giving:
```
16384 ms / 1000 = 16.384 ms per frame
1000 / 16.384 = 61.035 FPS
```

### 5. Animation State Machine

**There is no explicit state machine** in sprite.cpp. Animation state is managed through:

1. **Sprite data** (`sprite.h`):
   - `anim` - current animation index
   - `length` - number of frames in animation
   - `frame_idx[]` - mapping from animation time to frame index

2. **Instance data** (`world.cpp`):
   - `SpriteInst.anim` - current animation index
   - `SpriteInst.frame` - current frame (updated each tick)
   - `SpriteInst.reps[4]` - timing parameters

3. **Runtime updates** via:
   - `UpdateSpriteInst()` - update animation state
   - `AnimateSpriteInst()` - compute current frame from timestamp

### 6. Sprite Structure (for reference)

**Location:** `sprite.h:55-90`

```cpp
struct Sprite
{
    int projs;           // 1=single, 2=projection+reflection
    int anims;           // Number of animation sequences (0 = still)
    int frames;          // Number of frames in atlas (1 = still)
    int angles;          // View angles (e.g., 8 for 8-directional)
    Frame* atlas;        // Frame atlas [frames][angles][2]
    
    struct Anim {
        int length;      // Number of frames in animation
        int* frame_idx;  // [angles * 2] - maps to atlas
    } anim[1];
};
```

---

## Implications for Rust Port

### 1. Timing Model

Replace `stamp >> 14` with explicit millisecond timing:
```rust
// Instead of bit-shift:
// time = (stamp >> 14) % len;

// Use explicit timing:
let frame_time_ms = 1000.0 / 61.035;
let time = ((stamp as f64) / frame_time_ms) as i32 % len;
```

### 2. reps[4] Model

Implement as a struct:
```rust
struct AnimationTiming {
    hold_start: i32,   // Initial delay in frames
    frame_step: i32,   // ms per frame during forward playback
    hold_mid: i32,     // Hold frames at middle
    reverse_step: i32, // ms per frame during reverse playback
}
```

### 3. Frame Calculation

The frame calculation logic in `AnimateSpriteInst` must be ported exactly:
- Handle the 4 phases: initial hold, forward, middle hold, reverse
- Support both forward-only and ping-pong (reverse) animations

### 4. Default Behavior

Most sprites use `{0, 0, 0, 0}` for `reps`, meaning:
- Immediate start (no initial hold)
- 1 frame per animation frame (at 61 FPS)
- No middle hold
- No reverse

---

## Files Reviewed

| File            | Purpose                              |
|-----------------|--------------------------------------|
| `sprite.cpp`    | Sprite loading, atlas creation      |
| `sprite.h`      | Sprite data structures              |
| `world.cpp`     | Animation timing (`AnimateSpriteInst`) |
| `world.h`       | Public API for sprite instances     |
| `game.cpp`      | Sprite instance creation with reps  |

---

## Conclusion

The animation system uses a **simple but flexible time-based model** driven by the `reps[4]` parameters. The lack of a global `ms_per_frame` constant is intentional - timing is derived from the global timestamp bit-shift. For the Rust port, implement the `reps[4]` model exactly as-is, or consider adding a configurable frame rate if higher precision is needed.
