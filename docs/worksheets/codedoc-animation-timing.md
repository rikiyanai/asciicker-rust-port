# Animation Timing - C++ Source Analysis

Document generated from analysis of the original C++ source code in `/Users/rikihernandez/Downloads/Aciicker-Y9-2/`

## Source Files Analyzed
- `game.cpp` - Main game logic and character state management
- `world.cpp` - Sprite instance management and animation frame calculation
- `physics.cpp` - Physics integration loop with animation sync
- `game.h` - Character struct and ACTION enum definitions
- `sprite.h` - Sprite and Anim struct definitions

---

## 1. Animation Frame Timing Constants

Located in `game.cpp` (lines 409-411):

```cpp
static const int stand_us_per_frame = 30000;  // 30ms per frame (~33.33 FPS)
static const int fall_us_per_frame = 30000;   // 30ms per frame (~33.33 FPS)
static const int attack_us_per_frame = 20000; // 20ms per frame (~50 FPS)
```

**Timing Summary:**
| State   | Microseconds/Frame | Milliseconds/Frame | Approx FPS |
|---------|-------------------|---------------------|------------|
| STAND   | 30000             | 30                  | ~33        |
| FALL    | 30000             | 30                  | ~33        |
| ATTACK  | 20000             | 20                  | ~50        |

**Note:** These are microseconds (us), not milliseconds (ms). The suffix `_us_per_frame` confirms this.

---

## 2. Frame Duration (ms_per_frame equivalent)

The C++ code uses microseconds (`uint64_t stamp` in microseconds), so the conversion is:

```cpp
// Frame calculation in game.cpp (lines 6253, 6362, 6590, etc.)
int frame_index = (int)((_stamp - h->action_stamp) / attack_us_per_frame);
int frame = (int)((_stamp - h->action_stamp) / stand_us_per_frame);
```

**Frame Rate Calculation:**
- `stamp >> 14` = 61.035 FPS (used in `AnimateSpriteInst` in world.cpp:5781)
- This corresponds to ~16.67ms per frame (16384 microseconds)

The 61.035 FPS comes from: `1 << 14 = 16384` microseconds per frame

---

## 3. The reps[4] Parameter Meanings

Located in `world.cpp` (lines 520, 771-774, 5773-5792):

```cpp
struct SpriteInst : Inst
{
    // ...
    int reps[4];
    // ...
};
```

### Parameter Mapping

The `reps[4]` array controls animation playback with this structure:

| Index | Name          | Purpose                                                        |
|-------|---------------|----------------------------------------------------------------|
| [0]   | `hold_start`  | Number of frames to hold on frame 0 before animation starts   |
| [1]   | `frame_step`  | Number of "time units" to hold on each frame (playback speed)  |
| [2]   | `hold_mid`    | Number of frames to hold at end of forward playback            |
| [3]   | `reverse_step`| Number of "time units" to hold on each frame during reverse    |

### Animation Sequence Logic (from world.cpp:5773-5792)

```cpp
int len = si->reps[0] + si->reps[1] * sp->anim[anim].length + si->reps[2] + si->reps[3] * sp->anim[anim].length;

if (time < si->reps[0])
    frame = 0;                                          // HOLD_START: hold first frame
else if (time < si->reps[0] + si->reps[1] * sp->anim[anim].length)
    frame = (time - si->reps[0]) / si->reps[1];        // FORWARD: play animation
else if (time < si->reps[0] + si->reps[1] * sp->anim[anim].length + si->reps[2])
    frame = sp->anim[anim].length - 1;                 // HOLD_MID: hold last frame
else
    frame = sp->anim[anim].length - 1 -                // REVERSE: reverse playback
            (time - si->reps[0] - si->reps[1] * sp->anim[anim].length - si->reps[2]) / si->reps[3];
```

### Default Value

In `game.cpp`, all character sprites are initialized with:
```cpp
int reps[4] = { 0, 0, 0, 0 };
```

This means:
- No initial hold (reps[0] = 0)
- Normal speed forward (reps[1] = 0, treated as 1)
- No mid-hold (reps[2] = 0)
- No reverse (reps[3] = 0)

---

## 4. Animation State Machine Details

### Character State Enum

Located in `game.h` (lines 71-80):

```cpp
struct ACTION { enum
{
    NONE = 0,   // IDLE/MOVE - default animation (walking/idle)
    ATTACK,     // Attack animation for melee weapons
    FALL,       // Death/fall animation (plays once, transitions to DEAD)
    DEAD,       // Dead state (final frame of FALL, stays indefinitely)
    STAND,      // Standing up animation (appears unused)
    SIZE        // Array bounds sentinel
};};
```

### State Transition Rules

From `game.cpp` (SetAction* functions):

#### SetActionNone (line 4853)
- **Allowed from:** ANY state
- **Behavior:** Resets to anim=0, frame=0

#### SetActionAttack (line 4874)
- **Allowed from:** NONE only
- **Rejected from:** FALL, STAND, DEAD
- **Behavior:** Sets anim=0, frame=2 (sword) or frame=0 (crossbow)

#### SetActionStand (line 4910)
- **Allowed from:** FALL, DEAD only
- **Rejected from:** NONE, ATTACK
- **Behavior:** Recalculates timestamp to match current frame for smooth transition

#### SetActionFall (line 4944)
- **Allowed from:** NONE, STAND, ATTACK
- **Rejected from:** DEAD
- **Behavior:** If from STAND, smooth transition by recalculating timestamp. Otherwise starts at last frame.

#### SetActionDead (line 4981)
- **Allowed from:** Any state (no restrictions checked)
- **Behavior:** Sets final frame, stays indefinitely

### State Transition Diagram

```
                    +-------+
                    | NONE  | (idle/walk)
                    +-------+
                       |
          +------------+------------+
          |            |            |
          v            v            v
     +---------+  +---------+  +---------+
     | ATTACK  |  |  FALL   |  |  STAND  | (appears unused)
     +---------+  +---------+  +---------+
          |            |            |
          |      +------+------+    |
          |      |             |    |
          v      v             v    v
       +------+------------+--------+
       | DEAD |            |  NONE  |
       +------+            +--------+
```

---

## 5. Character State Timing Details

### NONE State (Idle/Walk)
- **Frame timing:** 30ms per frame (`stand_us_per_frame`)
- **Starting frame:** 0
- **Description:** Default animation for walking and idle states

### ATTACK State
- **Frame timing:** 20ms per frame (`attack_us_per_frame`) - FASTER than normal
- **Starting frame:** 2 (for sword), 0 (for crossbow)
- **Description:** Fast attack animation, ~2x speed of normal animation
- **Hit testing:** Occurs at specific frame indices (see game.cpp:6253, 6336, 6448, 6561)

### FALL State (Death)
- **Frame timing:** 30ms per frame (`fall_us_per_frame`)
- **Starting frame:** If from STAND, transitions smoothly. Otherwise starts at last frame.
- **Description:** Death/fall animation that plays once then typically transitions to DEAD

### DEAD State
- **Frame timing:** Static (no animation)
- **Starting frame:** Last frame of FALL animation
- **Description:** Terminal state - character stays at final frame indefinitely

### STAND State
- **Frame timing:** 30ms per frame (`stand_us_per_frame`)
- **Starting frame:** Recalculated to match current frame for smooth transition
- **Description:** Standing up animation - appears to be unused in practice

---

## 6. Animation Implementation Details

### Timestamp System
- Uses 64-bit microsecond timestamp (`uint64_t stamp`)
- `action_stamp` stored when action begins
- Current frame calculated as: `(current_stamp - action_stamp) / us_per_frame`

### Sprite Instance Structure

From `world.cpp`:
```cpp
struct SpriteInst : Inst
{
    World* w;
    Sprite* sprite;
    void* data;
    int anim;        // Current animation index
    int frame;       // Current frame (calculated from timestamp)
    int reps[4];     // Animation repetition parameters
    float yaw;
    float pos[3];
};
```

### Sprite Animation Structure

From `sprite.h`:
```cpp
struct Sprite
{
    int anims;           // Number of animations
    int angles;          // Number of view angles
    // ...
    struct Anim
    {
        int length;      // Number of frames in this animation
        int* frame_idx;  // Frame indices [angles * 2]
    };
    Anim anim[1];
};
```

---

## 7. Physics Integration with Animation

From `physics.cpp` (line 1250):
- Physics runs at fixed 15ms timesteps (interval = 15000 microseconds)
- Animation is decoupled from physics - frame is calculated from timestamp
- Returns number of physics substeps taken for animation sync

---

## Summary for Rust Port

### Key Constants to Port
```rust
const STAND_US_PER_FRAME: u64 = 30000;   // 30ms
const FALL_US_PER_FRAME: u64 = 30000;    // 30ms  
const ATTACK_US_PER_FRAME: u64 = 20000;  // 20ms
```

### Key Structs to Implement
1. `Character` with `action_stamp: u64`, `action: ACTION`, `anim: i32`, `frame: i32`
2. `SpriteInst` with `reps: [i32; 4]` for animation control
3. `ACTION` enum: NONE, ATTACK, FALL, DEAD, STAND

### Frame Calculation
```rust
fn current_frame(action_stamp: u64, current_stamp: u64, us_per_frame: u64) -> i32 {
    ((current_stamp - action_stamp) / us_per_frame) as i32
}
```

### reps[4] Behavior
- reps[0]: Initial hold (frames to wait before animation starts)
- reps[1]: Forward playback divisor (0 = 1, higher = slower)
- reps[2]: Mid-hold (frames to wait at end before reverse)
- reps[3]: Reverse playback divisor (0 = no reverse, higher = slower)
