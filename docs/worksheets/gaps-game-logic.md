> **STATUS: ACTIVE GAP ANALYSIS** — Generated 2026-02-20. Plan generated: plan-game-logic-gaps.md.

# GAP ANALYSIS: Asciicker Game Logic Documentation

This document identifies areas of the Asciicker game logic system that were NOT fully covered in existing research documents. The analysis compares the existing documentation in `game_logic_cpp.md` and `game_cpp_part1.md` against the actual implementation in `game.cpp` and `game.h`.

---

## 1. Game States and Modes Not Fully Documented

### 1.1 Fly Mode

The existing documentation mentions `main_menu` as the primary game state flag but does not document `fly_mode`, a boolean that controls camera behavior.

```cpp
// From game.cpp line 4161-4165
#ifdef PURE_TERM
    g->fly_mode = true;
#else
    g->fly_mode = false;
#endif
```

The `fly_mode` allows free camera movement without physics constraints. This mode is enabled by default in pure terminal builds but disabled in graphical builds.

**Gap:** No documentation exists for fly_mode behavior, triggers, or controls.

### 1.2 Editor Mode

The code contains `#ifdef EDITOR` conditionals that modify game behavior, but this mode is not documented in existing research.

```cpp
// From game.cpp line 4195-4200
#ifdef EDITOR
    g->main_menu = false;
#else
    g->main_menu = true;
    MainMenu_Show();
#endif
```

**Gap:** Editor-specific functionality, how it differs from gameplay mode, and what features are enabled/disabled.

### 1.3 Weather System Integration

The weather system is documented in `weather_cpp.md` but its integration with game logic is not covered. The game logic references weather through `GetWeather()` and `SetWeather()` functions.

```cpp
// From game.cpp - weather toggle in input handling
int w = (GetWeather() + 1) % 4;
SetWeather(w);
```

**Gap:** How weather affects gameplay, what triggers weather changes, and how weather state persists.

### 1.4 Camera Control System

The existing documentation does not cover the camera control system including `scene_shift`, `cam_shift`, and `zoom`.

```cpp
// From game.h
int scene_shift;     // horizontal scene offset for inventory
int cam_shift;       // vertical camera pan
float zoom;          // zoom level (1.0 default)
```

```cpp
// From game.cpp line 4160
g->zoom = 1.0f;  // match legacy web build default zoom
```

**Gap:** How these values interact, how they affect rendering, and user controls for adjusting them.

---

## 2. AI Behaviors Not Documented

### 2.1 Shoot-By Tracking

The existing AI documentation covers target selection and movement but does not document the `shoot_by` system that tracks which character shot whom.

```cpp
// From game.h
Character* shoot_by;         // character being shot by
uint64_t shoot_by_stamp;     // timestamp of shooting
```

This system affects AI target prioritization. When a character is shot by another, they receive higher priority in target selection:

```cpp
// From game.cpp - AI target selection with shoot_by weighting
if (h->shoot_by == h2 && 
    stamp > 500000 + h->shoot_by_stamp &&
    stamp < 5000000 + h->shoot_by_stamp)
{
    d *= 0.2f;  // Higher priority for being shot
}
```

**Gap:** How shoot_by is set, duration of the "being shot" state, and tactical implications.

### 2.2 Follower System

The existing documentation does not cover the `followers` counter or how it affects AI behavior.

```cpp
// From game.h
int followers;  // number of followers
```

```cpp
// From game.cpp - follower consideration in target selection
if (!enemy_ch || d * (h2->followers + 4) < enemy_cd * (enemy_cf + 4))
```

**Gap:** How followers are counted, how this affects enemy target selection, and strategic implications.

### 2.3 Buddy/Friendly AI Behavior

The existing documentation focuses on enemy AI but does not separately document buddy (friendly NPC) behavior.

```cpp
// From game.cpp line 3979-4099 - buddy spawning
#ifndef EDITOR
    for (int i = 0; i < 2; i++)
    {
        NPC_Human* buddy = (NPC_Human*)malloc(sizeof(NPC_Human));
        // buddy initialization with friendly color (clr = 0)
        buddy->enemy = false;
        // ...
    }
#endif
```

**Gap:** How buddies differ from enemies in AI behavior, what triggers buddy actions, and buddy equipment/abilities.

### 2.4 Multi-Stage Stuck Resolution

The existing AI documentation covers basic stuck detection but does not document the multi-stage resolution system.

```cpp
// From game.cpp lines 1402-1433 - stuck resolution stages
if (h->stuck >= 100 && h->stuck < 200)
{
    // Stage 1: Go opposite direction
    pio.x_force = -pio.x_force;
    pio.y_force = -pio.y_force;
}
else if (h->stuck >= 200 && h->stuck < 300)
{
    // Stage 2: Go around (perpendicular)
}
else if (h->stuck >= 300 && h->stuck < 400)
{
    // Stage 3: Keep jumping
}
else if (h->stuck >= 400)
{
    // Stage 4: Reset stuck counter
    h->stuck = 0;
}
```

**Gap:** The progression of stuck states, what triggers each stage, and the logic behind resolution attempts.

---

## 3. Save/Load Features

### 3.1 Screenshot Export System

The existing save/load documentation covers configuration persistence but does not document the screenshot export functionality.

```cpp
// From game.cpp lines 7832-7866
if (input.shot)
{
    input.shot = false;
    FILE* f = fopen("./shot.xp", "wb");
    // ... write sprite data
    WriteShotJson("./shot.json", _stamp, &io, this, width, height);
}
```

**Gap:** What is exported in screenshots, the JSON metadata format, and use cases for this data.

### 3.2 Game State JSON Export

The `WriteShotJson` function exports comprehensive game state but is not documented.

```cpp
// From game.cpp lines 485-535
static void WriteShotJson(const char* path, uint64_t stamp, const PhysicsIO* io, 
                          const Game* g, int width, int height)
{
    // Exports: version, stamp, size, map_path, camera (pos, yaw, zoom, perspective),
    // player (pos, dir), light (dir, ambience), water level
}
```

**Gap:** The complete JSON schema, what each field represents, and how to use this for debugging/replay.

### 3.3 Talk History Persistence

The `TalkMem` system for persisting chat history is defined but not documented.

```cpp
// From game.h
struct TalkMem
{
    char buf[256];
    int len;
};
TalkMem talk_mem[4];
```

```cpp
// From game.cpp lines 628-632
int r = (int)fread(g->talk_mem, sizeof(Game::TalkMem), 4, f);
```

**Gap:** How talk history is stored, what triggers saving, and how it is restored.

---

## 4. Multiplayer Specifics

### 4.1 Lag Measurement System

The existing networking documentation covers message types but does not document the lag measurement system.

```cpp
// From game.h
uint64_t last_lag;
int lag_ms;
bool lag_wait;
```

```cpp
// From game.cpp lines 1994-2000 - lag response handling
case 'l':
{
    STRUCT_RSP_LAG* lag = (STRUCT_RSP_LAG*)ptr;
    uint32_t s1 = 0;
    s1 |= lag->stamp[0] << 8;
    s1 |= lag->stamp[1] << 16;
    s1 |= lag->stamp[2] << 24;
    // ...
}
```

**Gap:** How lag is measured, how often pings occur, and how lag affects gameplay.

### 4.2 Item Synchronization

The existing documentation does not cover how items are synchronized between client and server.

```cpp
// From game.cpp lines 4300-4312 - item execution with story API
bool called = akAPI_OnItem(
    mi->in_use ? 'U' : 'E',
    mi->story_id,
    item->proto->kind,
    item->proto->sub_kind,
    item->proto->weight,
    mi->desc,
    &allowed,
    &story_id,
    &desc);
```

**Gap:** How item pickup/drop/equip is synchronized, server confirmation flow, and conflict resolution.

---

## 5. UI/HUD Features Not Fully Documented

### 5.1 Action Button System

The `bars_pos` and `show_buts` system for action button visibility is not documented.

```cpp
// From game.h
bool show_buts;  // true only if no popup is visible
int bars_pos;   // used to hide buts (0..7)
```

```cpp
// From game.cpp lines 4123-4124
g->show_buts = true;
g->bars_pos = 7;
```

```cpp
// From game.cpp lines 7681-7684 - animation
if (show_buts)
    bars_pos = Lerp(bars_pos, 7, 0.1f);
else
    bars_pos = Lerp(bars_pos, 0, 0.1f);
```

**Gap:** What buttons are shown, how bars_pos animates, and controls for showing/hiding.

### 5.2 Virtual Keyboard System

The virtual keyboard (`show_keyb`, `keyb_hide`) is not documented.

```cpp
// From game.h
bool show_keyb;     // activated together with talk_box by clicking on character
int keyb_hide;     // show / hide animator (vertical position)
uint8_t keyb_key[32];  // simulated key presses by touch/mouse
```

**Gap:** When virtual keyboard appears, how input is processed, and touch/mouse interaction.

### 5.3 Gamepad Overlay

The gamepad UI overlay (`show_gamepad`) is not documented.

```cpp
// From game.h
bool show_gamepad;
```

**Gap:** When gamepad overlay appears, controls displayed, and configuration options.

### 5.4 Minimap Rendering

The minimap rendering function exists but is not documented in game logic.

```cpp
// From game.cpp - RenderMinimap function
// Draws 32x16 minimap in top-right
// Shows terrain, NPCs, player position and direction
// Only rendered when !show_inventory && !main_menu
```

**Gap:** What information is displayed, how entities are represented, and performance implications.

### 5.5 HP Bar Rendering

The `HPBar` struct exists but is not fully documented.

```cpp
// From game.cpp lines 668-1045
struct HPBar
{
    static const int height = 4;
    void Paint(AnsiCell* ptr, int width, int height, float val, int xyw[3], bool flip) const
};
```

**Gap:** How HP percentage is calculated, flip parameter usage, and color scheme.

### 5.6 Camera Overlay (Debug Info)

The `show_cam_overlay` for debug information display is not documented.

```cpp
// From game.h
bool show_cam_overlay;
```

**Gap:** What debug information is shown, when it appears, and how to toggle it.

### 5.7 Inventory Visibility Animation

The `scene_shift` animation for inventory sliding is not documented.

```cpp
// From game.cpp lines 6650-6661
if (show_inventory)
    scene_shift = Lerp(scene_shift, inventory_width, 0.15f);
else
    scene_shift = Lerp(scene_shift, 0, 0.15f);
```

**Gap:** How inventory slides in/out, the animation curve, and performance considerations.

---

## Summary

The existing documentation in `game_logic_cpp.md` and `game_cpp_part1.md` provides a solid foundation for understanding the core game systems. However, several important areas remain undocumented or only partially covered:

1. **Game States:** fly_mode, Editor mode, weather integration, and camera controls need documentation
2. **AI Behaviors:** shoot_by tracking, follower system, buddy AI, and stuck resolution stages need full documentation
3. **Save/Load:** screenshot export, JSON state export, and talk history persistence need documentation
4. **Multiplayer:** lag measurement and item synchronization need documentation
5. **UI/HUD:** action buttons, virtual keyboard, gamepad overlay, minimap, HP bars, debug overlay, and inventory animations need documentation

These gaps represent areas where future documentation efforts should focus to provide a complete understanding of the Asciicker game logic system.
