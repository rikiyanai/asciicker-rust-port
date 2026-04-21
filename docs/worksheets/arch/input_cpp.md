# Asciicker Input System Architecture

## Overview

The Asciicker input system implements a **three-stage pipeline** that routes input events from operating system level through platform abstraction to game-level actions. The system supports keyboard, mouse, touch, and gamepad input across multiple platform backends (SDL2, X11, Win32, Web).

**Key Files:**
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/input.cpp` — Input routing documentation (minimal implementation)
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/platform.h` — PlatformInterface, KeyInfo, MouseInfo definitions
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/sdl.cpp` — SDL2 backend implementation
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/game.h` — Game::Input struct, GAME_KEYB, GAME_MOUSE enums
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/game.cpp` — Game::OnKeyb, Game::OnMouse, Game::OnTouch implementations
- `/Users/rikihernandez/Downloads/Aciicker-Y9-2/gamepad.cpp` — Gamepad mapping and event processing

---

## 1. Input Devices Supported

### 1.1 Keyboard

**Platform Backends:**
- **SDL2 (sdl.cpp)**: Uses `SDL_PollEvent()` with `SDL_KEYDOWN`/`SDL_KEYUP` events
- **X11 (x11.cpp)**: Uses `XNextEvent()` with `KeyPress`/`KeyRelease` events
- **Win32 (mswin.cpp)**: Uses `GetMessage()` with `WM_KEYDOWN`/`WM_KEYUP` messages
- **Web (game_web.cpp)**: Uses Emscripten callbacks (`EM_KEY_*` events)

**Key Features:**
- Physical key scanning (scancodes, not virtual key codes)
- Text input via `keyb_char` callback (UTF-8 character input)
- Modifier key tracking (Shift, Ctrl, Alt, Win)
- Auto-repeat detection via `A3D_AUTO_REPEAT` flag

### 1.2 Mouse

**Supported Events:**
- Movement (`MOUSE_MOVE`)
- Button events: Left, Middle, Right (`MOUSE_*_BUT_DOWN`, `MOUSE_*_BUT_UP`)
- Wheel scrolling (`MOUSE_WHEEL_UP`, `MOUSE_WHEEL_DOWN`)
- Window enter/leave events (`ENTER`, `LEAVE`)

**Platform Backends:**
- **SDL2**: `SDL_MOUSEMOTION`, `SDL_MOUSEBUTTONDOWN`/`SDL_MOUSEBUTTONUP`, `SDL_MOUSEWHEEL`
- **X11**: `MotionNotify`, `ButtonPress`/`ButtonRelease`
- **Win32**: `WM_MOUSEMOVE`, `WM_*BUTTON*`, `WM_MOUSEWHEEL`

**Mouse State Flags** (from `platform.h`):
```
MOVE = 1, LEFT_DN = 2, LEFT_UP = 3, RIGHT_DN = 4, RIGHT_UP = 5,
MIDDLE_DN = 6, MIDDLE_UP = 7, WHEEL_UP = 8, WHEEL_DN = 9,
ENTER = 10, LEAVE = 11,
LEFT = 0x10, RIGHT = 0x20, MIDDLE = 0x40, INSIDE = 0x80
```

Note: Message types (1-11) indicate discrete events, while flags (0x10-0x80) indicate current button/window state.

### 1.3 Touch

**Implementation:**
- Multi-touch support with 4 contacts (`input.contact[0..3]`)
- Contact 0: Primary mouse/touch
- Contacts 1-3: Additional touch points
- Emulated from mouse buttons when `TOUCH_EMU` defined

**Events:**
- `TOUCH_BEGIN`: New touch started
- `TOUCH_MOVE`: Touch moved
- `TOUCH_END`: Touch ended
- `TOUCH_CANCEL`: Touch cancelled

### 1.4 Gamepad

**Platform Support:**
- **SDL2 only**: Uses SDL_GameController API
- **X11**: Not supported (no standard X11 gamepad API)
- **Win32**: Not supported (no native Win32 gamepad API in this implementation)
- **Web**: Uses HTML5 Gamepad API via Emscripten

**Gamepad Features:**
- Up to 6 axes, 15 buttons
- Configurable mapping via visual drag-drop UI
- Supports Xbox and PS5 controller layouts
- Auto-reconnect on device swap

---

## 2. Input Event Handling

### 2.1 Three-Stage Pipeline

```
OS RAW INPUT → PLATFORM BACKEND → PLATFORM ABSTRACTION → GAME LAYER
   (SDL_PollEvent)    (translate)      (PlatformInterface)    (Game::OnKeyb)
```

**Stage 1: OS Captures Raw Input**
- SDL: `SDL_PollEvent()` → `SDL_KEYDOWN`, `SDL_MOUSEBUTTONDOWN`, `SDL_CONTROLLERAXISMOTION`
- X11: `XNextEvent()` → `KeyPress`, `ButtonPress`, `MotionNotify`
- Win32: `GetMessage()` → `WM_KEYDOWN`, `WM_LBUTTONDOWN`, `WM_MOUSEMOVE`
- Web: Emscripten callbacks → `keydown`, `mousedown`, `touchstart`

**Stage 2: Platform Backend Translates to Abstractions**
- SDL: `SDL_SCANCODE_SPACE` → `KeyInfo::A3D_SPACE`
- X11: `XLookupKeysym(XK_space)` → `KeyInfo::A3D_SPACE`
- Win32: `VK_SPACE` → `KeyInfo::A3D_SPACE`
- Gamepad: `SDL_CONTROLLER_BUTTON_A` → `gpad_button(0, 32767)`

**Stage 3: Game Layer Receives Abstracted Events**
- Keyboard: `wnd->platform_api.keyb_key(wnd, A3D_SPACE, true)` → `game.cpp OnKeyb()`
- Mouse: `wnd->platform_api.mouse(wnd, x, y, LEFT_DN)` → `game.cpp OnMouse()`
- Gamepad: `li->gpad_button(0, 32767)` → `gamepad.cpp UpdateGamePadButton()`

### 2.2 PlatformInterface Contract

From `platform.h`, the PlatformInterface struct defines input callbacks:

```c
struct PlatformInterface
{
    void(*init)(A3D_WND* wnd);
    void(*render)(A3D_WND* wnd);
    void(*resize)(A3D_WND* wnd, int w, int h);
    void(*close)(A3D_WND* wnd);
    void(*keyb_key)(A3D_WND* wnd, KeyInfo vk, bool down);    // Keyboard key event
    void(*keyb_char)(A3D_WND* wnd, wchar_t ch);              // Text character input
    void(*keyb_focus)(A3D_WND* wnd, bool set);              // Window focus change
    void(*mouse)(A3D_WND* wnd, int x, int y, MouseInfo mi); // Mouse event
    // ... asset loaders
};
```

**NULL Callback Policy:**
- All callbacks are OPTIONAL (can be null)
- Backends MUST check before calling: `if (wnd->platform_api.keyb_key) ...`
- Use case: Headless server builds can leave input callbacks null

### 2.3 LoopInterface Contract (Gamepad)

```c
struct LoopInterface
{
    void(*gpad_mount)(const char* name, int axes, int buttons, const uint8_t mapping[]);
    void(*gpad_unmount)();
    void(*gpad_button)(int b, int16_t pos);
    void(*gpad_axis)(int a, int16_t pos);
};
```

---

## 3. Key Mapping System

### 3.1 Platform-Independent KeyInfo Enum

From `platform.h`, the KeyInfo enum defines 135 platform-independent key codes:

```c
enum KeyInfo
{
    A3D_NONE = 0,           // Reserved: "no key" (invalid key code)
    
    // Navigation keys
    A3D_BACKSPACE, A3D_TAB, A3D_ENTER,
    A3D_PAUSE, A3D_ESCAPE,
    A3D_SPACE,
    A3D_PAGEUP, A3D_PAGEDOWN, A3D_END, A3D_HOME,
    A3D_LEFT, A3D_UP, A3D_RIGHT, A3D_DOWN,
    A3D_PRINT, A3D_INSERT, A3D_DELETE,
    
    // Alphanumeric keys
    A3D_0 through A3D_9,
    A3D_A through A3D_Z,
    
    // Modifier keys
    A3D_LWIN, A3D_RWIN, A3D_APPS,
    A3D_LSHIFT, A3D_RSHIFT,
    A3D_LCTRL, A3D_RCTRL,
    A3D_LALT, A3D_RALT,
    A3D_CAPSLOCK, A3D_NUMLOCK, A3D_SCROLLLOCK,
    
    // Numpad keys
    A3D_NUMPAD_0 through A3D_NUMPAD_9,
    A3D_NUMPAD_MULTIPLY, A3D_NUMPAD_DIVIDE,
    A3D_NUMPAD_ADD, A3D_NUMPAD_SUBTRACT,
    A3D_NUMPAD_DECIMAL, A3D_NUMPAD_ENTER,
    
    // Function keys
    A3D_F1 through A3D_F24,
    
    // OEM keys (keyboard layout dependent)
    A3D_OEM_COLON, A3D_OEM_PLUS, A3D_OEM_COMMA,
    A3D_OEM_MINUS, A3D_OEM_PERIOD, A3D_OEM_SLASH,
    A3D_OEM_TILDE, A3D_OEM_OPEN, A3D_OEM_CLOSE,
    A3D_OEM_BACKSLASH, A3D_OEM_QUOTATION,
    
    A3D_MAPEND,
    A3D_AUTO_REPEAT = 256  // Flag: key event is auto-repeating
};
```

**Design Rationale:**
- Starts at `A3D_BACKSPACE = 1` (not 0): `A3D_NONE = 0` reserved for "no key" / translation failed
- `A3D_AUTO_REPEAT = 256`: Used as flag bit (`key | A3D_AUTO_REPEAT` = auto-repeat event)
- OEM keys are keyboard layout dependent (US-specific interpretations in comments)

### 3.2 SDL Key Mapping Tables

From `sdl.cpp`, bidirectional translation tables:

**A3D2SDL[] (KeyInfo → SDL_Scancode):**
- 135-element array mapping platform-independent keys to SDL scancodes
- Used by `a3dGetKeyb()` to query keyboard state via `SDL_GetKeyboardState()`

**SDL2A3D[] (SDL_Scancode → KeyInfo):**
- 128-element array mapping SDL scancodes to platform-independent keys
- Used in event loop to translate SDL events to KeyInfo enum

```c
// Example mappings from sdl.cpp:
int A3D2SDL[] =
{
    SDL_SCANCODE_UNKNOWN,      // A3D_NONE
    SDL_SCANCODE_BACKSPACE,    // A3D_BACKSPACE
    SDL_SCANCODE_TAB,          // A3D_TAB
    SDL_SCANCODE_RETURN,       // A3D_ENTER
    // ... 131 more
};

KeyInfo SDL2A3D[128] =
{
    A3D_NONE,                  // SDL_SCANCODE_UNKNOWN
    A3D_NONE, A3D_NONE, A3D_NONE, // reserved
    A3D_A, A3D_B, A3D_C, ...  // letter keys
    A3D_1, A3D_2, A3D_3, ...  // number keys
    A3D_ENTER, A3D_ESCAPE, A3D_BACKSPACE, A3D_TAB, A3D_SPACE,
    // ... 80+ more
};
```

### 3.3 Auto-Repeat Detection

From `game.cpp`:
```c
bool auto_rep = (key & A3D_AUTO_REPEAT) != 0;
int shot_key = key & ~A3D_AUTO_REPEAT;
```

The `A3D_AUTO_REPEAT` flag is OR'd with the key code to indicate an auto-repeating key event. Game code can check this to handle initial keypresses differently from repeated events.

---

## 4. Input State Tracking

### 4.1 Game::Input Struct

From `game.h`, the Input struct accumulates input events for frame-by-frame processing:

```c
struct Input 
{
    int last_hit_char;
    uint8_t key[32];                    // Keyboard state (256 bits = 256 possible keys)

    // Gamepad state
    int pad_item;                       // Item index to pick + 1
    bool pad_connected;
    int pad_autorep;                    // Button+1 for auto-repeat
    uint64_t pad_stamp;
    uint32_t pad_button;
    int16_t pad_axis[32];

    // Touch/contact state
    struct Contact
    {
        enum
        {
            NONE,
            KEYBCAP,                    // Virtual keyboard
            PLAYER,                     // Player interaction
            TORQUE,                     // Rotation control (right mouse or timer touch)
            FORCE,                      // Movement force
            ITEM_LIST_CLICK,
            ITEM_LIST_DRAG,
            ITEM_GRID_CLICK,
            ITEM_GRID_DRAG,
            ITEM_GRID_SCROLL,
        };
        
        int action;                      // Current contact action
        int drag;                       // Button that initiated drag (0 if none)
        int pos[2];                     // Current position
        int drag_from[2];                // Drag start position
        Item* item;
        int my_item;
        int keyb_cap;                   // Virtual keycap if touch started there
        bool player_hit;
        int margin;                     // -1: left, +1: right, 0: none
        float start_yaw;
        int scroll;
    };
    
    Contact contact[4];                  // 0: mouse, 1-3: touch points

    uint8_t but;                        // Mouse button state
    int wheel;                          // Relative mouse wheel
    int size[2];                        // Window size in pixels
    bool jump;

    float api_move[3];                  // Movement vector (x, y, alpha)
    bool shoot;
    int shoot_xy[2];
    bool shot;                          // Screenshot trigger

    // Helper methods
    bool IsKeyDown(int k)
    {
        return (key[k >> 3] & (1 << (k & 7))) != 0;
    }
    
    void ClearKey(int k)
    {
        key[k >> 3] &= ~(1 << (k & 7));
    }
};
```

### 4.2 Key State Storage

The keyboard state uses a **bitfield** approach:
- `key[32]` provides 256 bits (32 bytes × 8 bits)
- Each bit represents one key (KeyInfo enum value)
- `IsKeyDown(k)` checks if key `k` is currently pressed
- `ClearKey(k)` explicitly clears a key's pressed state

### 4.3 Button/Contact State

**Mouse buttons** (`but` field):
- Bit 0 (0x01): Left button
- Bit 1 (0x02): Right button
- Bit 2 (0x04): Middle button

**Contact actions** (for multi-touch and drag-drop):
- `NONE`: No active contact
- `KEYBCAP`: Touch on virtual keyboard
- `PLAYER`: Touch on player/sprite
- `TORQUE`: Rotation control (right mouse or long-press on margin)
- `FORCE`: Movement force input
- `ITEM_*`: Inventory interaction modes

---

## 5. Input Routing to Game Systems

### 5.1 Layered Input Dispatch

From `game.cpp`, input uses **layered dispatch** to prevent pass-through:

```
INPUT → main_menu → menu_depth → show_gamepad → game world
         ↓           ↓            ↓              ↓
    MainMenu_*    MenuKeyb    GamePadKeyb    Game logic
```

**Priority Order:**
1. **Main Menu** (`main_menu`): If true, all input goes to `MainMenu_OnKeyb/Mouse`
2. **Pause Menu** (`menu_depth >= 0`): If true, all input goes to `MenuKeyb/Mouse`
3. **Gamepad Config** (`show_gamepad`): If true, input routes to `GamePadKeyb/Contact`
4. **Game World**: Normal gameplay input handling

**Why layered dispatch?**
- UI layers consume input exclusively to prevent pass-through
- Example: Pressing ESC in menu closes menu, not opens inventory
- Prevents typing in chat from also triggering movement

### 5.2 Keyboard Routing (Game::OnKeyb)

From `game.cpp:7920`:

```c
void Game::OnKeyb(GAME_KEYB keyb, int key)
{
    // Global key handlers (F3: weather, F9: cam overlay, F10: screenshot)
    if (keyb == GAME_KEYB::KEYB_DOWN)
    {
        bool auto_rep = (key & A3D_AUTO_REPEAT) != 0;
        int shot_key = key & ~A3D_AUTO_REPEAT;
        
        if ((shot_key == A3D_F3 || shot_key == A3D_OEM_TILDE) && !auto_rep)
            SetWeather((GetWeather() + 1) % 4);
        if (shot_key == A3D_F9 && !auto_rep)
            show_cam_overlay = !show_cam_overlay;
        if (shot_key == A3D_F10 && !auto_rep)
            input.shot = true;
    }
    
    // Layer 1: Main menu
    if (main_menu)
    {
        MainMenu_OnKeyb(keyb, key);
        return;
    }
    
    // Layer 2: Pause/inventory menu
    if (menu_depth >= 0)
    {
        MenuKeyb(keyb, key);
        return;
    }
    
    // Layer 3: Gamepad config UI
    if (show_gamepad)
    {
        // Convert GAME_KEYB to gamepad config UI keys
        // Space → 0, Enter → 1, Escape/Backslash → 2, Arrows → 3-6
        GamePadKeyb(k, stamp);
        return;
    }
    
    // Layer 4: Game world
    // ... gameplay key handling (talk box, inventory, etc.)
}
```

### 5.3 Mouse Routing (Game::OnMouse)

From `game.cpp:9729`:

```c
void Game::OnMouse(GAME_MOUSE mouse, int x, int y)
{
    // Layer 1: Main menu
    if (main_menu)
    {
        MainMenu_OnMouse(mouse, x, y);
        return;
    }
    
    // Layer 2: Pause/inventory menu
    if (menu_depth >= 0)
    {
        MenuMouse(mouse, x, y);
        return;
    }
    
    // Layer 3: Gamepad config UI
    if (show_gamepad)
    {
        // Convert mouse events to gamepad UI events
        // LEFT_BUT_DOWN → 0 (begin), MOVE → 1, LEFT_BUT_UP → 2 (end)
        GamePadContact(0, ev, p[0], p[1], stamp);
        return;
    }
    
    // Layer 4: Game world
    switch (mouse)
    {
        case MOUSE_WHEEL_DOWN/UP:
            // Inventory scroll
        case MOUSE_LEFT_BUT_DOWN:
            // Start contact / attack
        case MOUSE_LEFT_BUT_UP:
            // End contact
        case MOUSE_RIGHT_BUT_DOWN:
            // Jump / start torque
        // ... more cases
    }
}
```

### 5.4 Touch Routing (Game::OnTouch)

From `game.cpp:9961`:

Multi-touch events are handled similarly to mouse events but with support for multiple simultaneous touch points (contacts 1-3, where contact 0 is reserved for mouse).

### 5.5 Gamepad Routing

**Flow:**
1. SDL backend detects gamepad → `gpad_mount()` callback
2. `ConnectGamePad()` in gamepad.cpp initializes state, builds inverse mapping tables
3. On button/axis events → `UpdateGamePadButton()` or `UpdateGamePadAxis()`
4. These functions apply `gamepad_mapping[]` to translate raw input to output
5. Game code reads `gamepad_button_output[]` and `gamepad_axis_output[]`

From `gamepad.cpp`:

```c
// Input → Output mapping
// gamepad_mapping[256]: Input index → Output index (0xFF = unmapped)

int UpdateGamePadButton(int b, int16_t v, uint32_t out[1])
{
    gamepad_button[b] = v;
    uint8_t m = gamepad_mapping[2*gamepad_axes + b];
    if (m < 0xFC)  // Valid output (0-20)
    {
        if (m & 0x80)
            UpdateButtonOutput(m & 0x3F, out);  // Button output
        else
            UpdateAxisOutput(m & 0x3F, out);     // Axis output
    }
}
```

---

## 6. Input Data Flow Example

**Example: User presses SPACE to jump**

```
1. SDL_PollEvent() → SDL_KEYDOWN, SDL_SCANCODE_SPACE
2. sdl.cpp translates: SDL_SCANCODE_SPACE → KeyInfo::A3D_SPACE
3. sdl.cpp calls: wnd->platform_api.keyb_key(wnd, A3D_SPACE, true)
4. game.cpp OnKeyb(GAME_KEYB::KEYB_DOWN, A3D_SPACE):
   - Not in main_menu, not in menu, not in gamepad config
   - Routes to game world handler
   - Sets input.jump = true (or processes attack/movement)
5. Game loop processes input.jump → player jumps
```

---

## 7. Platform Backend Comparison

| Feature          | SDL2      | X11       | Win32     | Web        |
|-----------------|-----------|-----------|-----------|------------|
| Keyboard        | Yes       | Yes       | Yes       | Yes        |
| Mouse           | Yes       | Yes       | Yes       | Yes        |
| Touch           | Yes*      | No        | No        | Yes        |
| Gamepad         | Yes       | No        | No        | Yes        |
| Text Input      | Yes       | Yes       | Yes       | Yes        |
| Window Focus    | Yes       | Yes       | Yes       | Yes        |

*SDL2 touch requires `TOUCH_EMU` define to emulate touches from mouse

---

## 8. Future Improvements (from input.cpp)

The current `input.cpp` is minimal placeholder. Future plans include:

1. **Aggregate ALL input sources:**
   - A3D window (platform backends): Keyboard, character, mouse, gamepad
   - Terminal escape codes (xterm.cpp): Character, mouse, Kitty keyboard codes
   - Web callbacks (web.cpp): Character, keyboard, mouse, touch, gamepad

2. **Dispatch to screen stacking:**
   - Priority system: Modal dialogs capture input before game world
   - Input mapping: Rebindable keys, custom gamepad layouts

3. **Network input:**
   - Support for remote input over network

---

## Summary

The Asciicker input system provides a robust, platform-agnostic input handling architecture:

1. **Device Support**: Keyboard, mouse, touch, and gamepad across multiple platforms
2. **Event Handling**: Three-stage pipeline from OS to game with platform abstraction
3. **Key Mapping**: Bidirectional translation between OS key codes and platform-independent KeyInfo enum
4. **State Tracking**: Bitfield-based keyboard state, contact-based touch/mouse state, gamepad mapping tables
5. **Input Routing**: Layered dispatch prevents UI input from leaking to game world

This architecture allows the game engine to remain portable while providing rich input customization (gamepad mapping UI) and consistent input handling across platforms.
