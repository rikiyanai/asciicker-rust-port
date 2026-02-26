# A3D Key Code Mapping Documentation

**Source Files:**
- `/Users/r/Downloads/asciicker-Y9-2/platform.h`
- `/Users/r/Downloads/asciicker-Y9-2/sdl.cpp`

---

## 1. KeyInfo Enum Definition

**Location:** `platform.h` lines 182-317

The `KeyInfo` enum defines platform-independent keyboard key codes used throughout the game engine.

```cpp
enum KeyInfo
{
    A3D_NONE = 0,

    A3D_BACKSPACE,
    A3D_TAB,
    A3D_ENTER,

    A3D_PAUSE,
    A3D_ESCAPE,

    A3D_SPACE,
    A3D_PAGEUP,
    A3D_PAGEDOWN,
    A3D_END,
    A3D_HOME,
    A3D_LEFT,
    A3D_UP,
    A3D_RIGHT,
    A3D_DOWN,

    A3D_PRINT,
    A3D_INSERT,
    A3D_DELETE,

    A3D_0,
    A3D_1,
    A3D_2,
    A3D_3,
    A3D_4,
    A3D_5,
    A3D_6,
    A3D_7,
    A3D_8,
    A3D_9,

    A3D_A,
    A3D_B,
    A3D_C,
    A3D_D,
    A3D_E,
    A3D_F,
    A3D_G,
    A3D_H,
    A3D_I,
    A3D_J,
    A3D_K,
    A3D_L,
    A3D_M,
    A3D_N,
    A3D_O,
    A3D_P,
    A3D_Q,
    A3D_R,
    A3D_S,
    A3D_T,
    A3D_U,
    A3D_V,
    A3D_W,
    A3D_X,
    A3D_Y,
    A3D_Z,

    A3D_LWIN,
    A3D_RWIN,
    A3D_APPS,

    A3D_NUMPAD_0,
    A3D_NUMPAD_1,
    A3D_NUMPAD_2,
    A3D_NUMPAD_3,
    A3D_NUMPAD_4,
    A3D_NUMPAD_5,
    A3D_NUMPAD_6,
    A3D_NUMPAD_7,
    A3D_NUMPAD_8,
    A3D_NUMPAD_9,
    A3D_NUMPAD_MULTIPLY,
    A3D_NUMPAD_DIVIDE,
    A3D_NUMPAD_ADD,
    A3D_NUMPAD_SUBTRACT,
    A3D_NUMPAD_DECIMAL,
    A3D_NUMPAD_ENTER,

    A3D_F1,
    A3D_F2,
    A3D_F3,
    A3D_F4,
    A3D_F5,
    A3D_F6,
    A3D_F7,
    A3D_F8,
    A3D_F9,
    A3D_F10,
    A3D_F11,
    A3D_F12,
    A3D_F13,
    A3D_F14,
    A3D_F15,
    A3D_F16,
    A3D_F17,
    A3D_F18,
    A3D_F19,
    A3D_F20,
    A3D_F21,
    A3D_F22,
    A3D_F23,
    A3D_F24,

    A3D_CAPSLOCK,
    A3D_NUMLOCK,
    A3D_SCROLLLOCK,

    A3D_LSHIFT,
    A3D_RSHIFT,
    A3D_LCTRL,
    A3D_RCTRL,
    A3D_LALT,
    A3D_RALT,

    A3D_OEM_COLON,       // ;: for US
    A3D_OEM_PLUS,        // =+ any country
    A3D_OEM_COMMA,       // ,< any country
    A3D_OEM_MINUS,       // -_ any country
    A3D_OEM_PERIOD,      // .> any country
    A3D_OEM_SLASH,       // /? for US
    A3D_OEM_TILDE,       // `~ for US

    A3D_OEM_OPEN,        // [{ for US
    A3D_OEM_CLOSE,       // ]} for US
    A3D_OEM_BACKSLASH,   // \| for US
    A3D_OEM_QUOTATION,   // single quote/double quote for US

    A3D_MAPEND,
    A3D_AUTO_REPEAT = 256
};
```

---

## 2. Key Code Definitions Summary

| Category | Key Codes | Count |
|----------|-----------|-------|
| Control Keys | BACKSPACE, TAB, ENTER, PAUSE, ESCAPE | 5 |
| Navigation | SPACE, PAGEUP, PAGEDOWN, END, HOME, LEFT, UP, RIGHT, DOWN | 9 |
| Editing | PRINT, INSERT, DELETE | 3 |
| Digits | 0-9 | 10 |
| Letters | A-Z | 26 |
| Windows Keys | LWIN, RWIN, APPS | 3 |
| Numpad | 0-9, MULTIPLY, DIVIDE, ADD, SUBTRACT, DECIMAL, ENTER | 17 |
| Function Keys | F1-F24 | 24 |
| Lock Keys | CAPSLOCK, NUMLOCK, SCROLLLOCK | 3 |
| Modifiers | LSHIFT, RSHIFT, LCTRL, RCTRL, LALT, RALT | 6 |
| OEM Keys | COLON, PLUS, COMMA, MINUS, PERIOD, SLASH, TILDE, OPEN, CLOSE, BACKSLASH, QUOTATION | 11 |

---

## 3. Total Number of Key Codes

**Key Code Allocation:**

- **A3D_NONE = 0**: Reserved for "no key" (invalid key code, unmapped keys)
- **A3D_BACKSPACE (1) to A3D_MAPEND (115)**: 115 key codes
- **A3D_AUTO_REPEAT = 256**: Flag for auto-repeat events (not a key code)

**Total: 115 key codes** (enum values 1-115)

---

## 4. Platform Translation Tables

### A3D2SDL[] (KeyInfo to SDL_Scancode)

**Location:** `sdl.cpp` lines 365-509

Maps platform-independent KeyInfo enum values to SDL scancodes. Used by `a3dGetKeyb()` to query keyboard state via `SDL_GetKeyboardState()`.

```cpp
int A3D2SDL[] =
{
    SDL_SCANCODE_UNKNOWN,              // A3D_NONE

    SDL_SCANCODE_BACKSPACE,            // A3D_BACKSPACE
    SDL_SCANCODE_TAB,                  // A3D_TAB
    SDL_SCANCODE_RETURN,               // A3D_ENTER

    SDL_SCANCODE_PAUSE,                // A3D_PAUSE
    SDL_SCANCODE_ESCAPE,               // A3D_ESCAPE

    SDL_SCANCODE_SPACE,                // A3D_SPACE
    SDL_SCANCODE_PAGEUP,               // A3D_PAGEUP
    SDL_SCANCODE_PAGEDOWN,             // A3D_PAGEDOWN
    SDL_SCANCODE_END,                  // A3D_END
    SDL_SCANCODE_HOME,                 // A3D_HOME
    SDL_SCANCODE_LEFT,                 // A3D_LEFT
    SDL_SCANCODE_UP,                   // A3D_UP
    SDL_SCANCODE_RIGHT,                // A3D_RIGHT
    SDL_SCANCODE_DOWN,                 // A3D_DOWN

    SDL_SCANCODE_PRINTSCREEN,          // A3D_PRINT
    SDL_SCANCODE_INSERT,               // A3D_INSERT
    SDL_SCANCODE_DELETE,               // A3D_DELETE

    SDL_SCANCODE_0,                    // A3D_0
    SDL_SCANCODE_1,                    // A3D_1
    SDL_SCANCODE_2,                    // A3D_2
    SDL_SCANCODE_3,                    // A3D_3
    SDL_SCANCODE_4,                    // A3D_4
    SDL_SCANCODE_5,                    // A3D_5
    SDL_SCANCODE_6,                    // A3D_6
    SDL_SCANCODE_7,                    // A3D_7
    SDL_SCANCODE_8,                    // A3D_8
    SDL_SCANCODE_9,                    // A3D_9

    SDL_SCANCODE_A,                    // A3D_A
    SDL_SCANCODE_B,                    // A3D_B
    SDL_SCANCODE_C,                    // A3D_C
    SDL_SCANCODE_D,                    // A3D_D
    SDL_SCANCODE_E,                    // A3D_E
    SDL_SCANCODE_F,                    // A3D_F
    SDL_SCANCODE_G,                    // A3D_G
    SDL_SCANCODE_H,                    // A3D_H
    SDL_SCANCODE_I,                    // A3D_I
    SDL_SCANCODE_J,                    // A3D_J
    SDL_SCANCODE_K,                    // A3D_K
    SDL_SCANCODE_L,                    // A3D_L
    SDL_SCANCODE_M,                    // A3D_M
    SDL_SCANCODE_N,                    // A3D_N
    SDL_SCANCODE_O,                    // A3D_O
    SDL_SCANCODE_P,                    // A3D_P
    SDL_SCANCODE_Q,                    // A3D_Q
    SDL_SCANCODE_R,                    // A3D_R
    SDL_SCANCODE_S,                    // A3D_S
    SDL_SCANCODE_T,                    // A3D_T
    SDL_SCANCODE_U,                    // A3D_U
    SDL_SCANCODE_V,                    // A3D_V
    SDL_SCANCODE_W,                    // A3D_W
    SDL_SCANCODE_X,                    // A3D_X
    SDL_SCANCODE_Y,                    // A3D_Y
    SDL_SCANCODE_Z,                    // A3D_Z

    SDL_SCANCODE_LGUI,                 // A3D_LWIN
    SDL_SCANCODE_RGUI,                 // A3D_RWIN
    SDL_SCANCODE_APPLICATION,          // A3D_APPS

    SDL_SCANCODE_KP_0,                 // A3D_NUMPAD_0
    SDL_SCANCODE_KP_1,                 // A3D_NUMPAD_1
    SDL_SCANCODE_KP_2,                 // A3D_NUMPAD_2
    SDL_SCANCODE_KP_3,                 // A3D_NUMPAD_3
    SDL_SCANCODE_KP_4,                 // A3D_NUMPAD_4
    SDL_SCANCODE_KP_5,                 // A3D_NUMPAD_5
    SDL_SCANCODE_KP_6,                 // A3D_NUMPAD_6
    SDL_SCANCODE_KP_7,                 // A3D_NUMPAD_7
    SDL_SCANCODE_KP_8,                 // A3D_NUMPAD_8
    SDL_SCANCODE_KP_9,                 // A3D_NUMPAD_9
    SDL_SCANCODE_KP_MULTIPLY,          // A3D_NUMPAD_MULTIPLY
    SDL_SCANCODE_KP_DIVIDE,            // A3D_NUMPAD_DIVIDE
    SDL_SCANCODE_KP_PLUS,              // A3D_NUMPAD_ADD
    SDL_SCANCODE_KP_MINUS,             // A3D_NUMPAD_SUBTRACT
    SDL_SCANCODE_KP_DECIMAL,           // A3D_NUMPAD_DECIMAL
    SDL_SCANCODE_KP_ENTER,              // A3D_NUMPAD_ENTER

    SDL_SCANCODE_F1,                   // A3D_F1
    SDL_SCANCODE_F2,                   // A3D_F2
    SDL_SCANCODE_F3,                   // A3D_F3
    SDL_SCANCODE_F4,                   // A3D_F4
    SDL_SCANCODE_F5,                   // A3D_F5
    SDL_SCANCODE_F6,                   // A3D_F6
    SDL_SCANCODE_F7,                   // A3D_F7
    SDL_SCANCODE_F8,                   // A3D_F8
    SDL_SCANCODE_F9,                   // A3D_F9
    SDL_SCANCODE_F10,                  // A3D_F10
    SDL_SCANCODE_F11,                  // A3D_F11
    SDL_SCANCODE_F12,                  // A3D_F12
    SDL_SCANCODE_F13,                  // A3D_F13
    SDL_SCANCODE_F14,                  // A3D_F14
    SDL_SCANCODE_F15,                  // A3D_F15
    SDL_SCANCODE_F16,                  // A3D_F16
    SDL_SCANCODE_F17,                  // A3D_F17
    SDL_SCANCODE_F18,                  // A3D_F18
    SDL_SCANCODE_F19,                  // A3D_F19
    SDL_SCANCODE_F20,                  // A3D_F20
    SDL_SCANCODE_F21,                  // A3D_F21
    SDL_SCANCODE_F22,                  // A3D_F22
    SDL_SCANCODE_F23,                  // A3D_F23
    SDL_SCANCODE_F24,                  // A3D_F24

    SDL_SCANCODE_CAPSLOCK,             // A3D_CAPSLOCK
    SDL_SCANCODE_NUMLOCKCLEAR,         // A3D_NUMLOCK
    SDL_SCANCODE_SCROLLLOCK,           // A3D_SCROLLLOCK

    SDL_SCANCODE_LSHIFT,               // A3D_LSHIFT
    SDL_SCANCODE_RSHIFT,               // A3D_RSHIFT
    SDL_SCANCODE_LCTRL,                // A3D_LCTRL
    SDL_SCANCODE_RCTRL,                // A3D_RCTRL
    SDL_SCANCODE_LALT,                 // A3D_LALT
    SDL_SCANCODE_RALT,                 // A3D_RALT

    SDL_SCANCODE_SEMICOLON,            // A3D_OEM_COLON
    SDL_SCANCODE_EQUALS,               // A3D_OEM_PLUS
    SDL_SCANCODE_COMMA,                // A3D_OEM_COMMA
    SDL_SCANCODE_MINUS,                // A3D_OEM_MINUS
    SDL_SCANCODE_PERIOD,               // A3D_OEM_PERIOD
    SDL_SCANCODE_SLASH,                // A3D_OEM_SLASH
    SDL_SCANCODE_GRAVE,                // A3D_OEM_TILDE

    SDL_SCANCODE_LEFTBRACKET,          // A3D_OEM_OPEN
    SDL_SCANCODE_RIGHTBRACKET,         // A3D_OEM_CLOSE
    SDL_SCANCODE_BACKSLASH,            // A3D_OEM_BACKSLASH
    SDL_SCANCODE_APOSTROPHE,           // A3D_OEM_QUOTATION

    // Padding entries for A3D_MAPEND and beyond
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN,
    SDL_SCANCODE_UNKNOWN
};
```

### SDL2A3D[] (SDL_Scancode to KeyInfo)

**Location:** `sdl.cpp` lines 515-659

Maps SDL scancodes to platform-independent KeyInfo enum. Array size is 128 to cover SDL scancode range (0-127).

```cpp
KeyInfo SDL2A3D[128] =
{
    A3D_NONE,                          // SDL_SCANCODE_UNKNOWN
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,

    A3D_A,                             // SDL_SCANCODE_A
    A3D_B,                             // SDL_SCANCODE_B
    A3D_C,                             // SDL_SCANCODE_C
    A3D_D,                             // SDL_SCANCODE_D
    A3D_E,                             // SDL_SCANCODE_E
    A3D_F,                             // SDL_SCANCODE_F
    A3D_G,                             // SDL_SCANCODE_G
    A3D_H,                             // SDL_SCANCODE_H
    A3D_I,                             // SDL_SCANCODE_I
    A3D_J,                             // SDL_SCANCODE_J
    A3D_K,                             // SDL_SCANCODE_K
    A3D_L,                             // SDL_SCANCODE_L
    A3D_M,                             // SDL_SCANCODE_M
    A3D_N,                             // SDL_SCANCODE_N
    A3D_O,                             // SDL_SCANCODE_O
    A3D_P,                             // SDL_SCANCODE_P
    A3D_Q,                             // SDL_SCANCODE_Q
    A3D_R,                             // SDL_SCANCODE_R
    A3D_S,                             // SDL_SCANCODE_S
    A3D_T,                             // SDL_SCANCODE_T
    A3D_U,                             // SDL_SCANCODE_U
    A3D_V,                             // SDL_SCANCODE_V
    A3D_W,                             // SDL_SCANCODE_W
    A3D_X,                             // SDL_SCANCODE_X
    A3D_Y,                             // SDL_SCANCODE_Y
    A3D_Z,                             // SDL_SCANCODE_Z

    A3D_1,                             // SDL_SCANCODE_1
    A3D_2,                             // SDL_SCANCODE_2
    A3D_3,                             // SDL_SCANCODE_3
    A3D_4,                             // SDL_SCANCODE_4
    A3D_5,                             // SDL_SCANCODE_5
    A3D_6,                             // SDL_SCANCODE_6
    A3D_7,                             // SDL_SCANCODE_7
    A3D_8,                             // SDL_SCANCODE_8
    A3D_9,                             // SDL_SCANCODE_9
    A3D_0,                             // SDL_SCANCODE_0

    A3D_ENTER,                         // SDL_SCANCODE_RETURN
    A3D_ESCAPE,                        // SDL_SCANCODE_ESCAPE
    A3D_BACKSPACE,                    // SDL_SCANCODE_BACKSPACE
    A3D_TAB,                           // SDL_SCANCODE_TAB
    A3D_SPACE,                         // SDL_SCANCODE_SPACE

    A3D_OEM_MINUS,                     // SDL_SCANCODE_MINUS
    A3D_OEM_PLUS,                      // SDL_SCANCODE_EQUALS
    A3D_OEM_OPEN,                      // SDL_SCANCODE_LEFTBRACKET
    A3D_OEM_CLOSE,                     // SDL_SCANCODE_RIGHTBRACKET
    A3D_OEM_BACKSLASH,                 // SDL_SCANCODE_BACKSLASH
    A3D_NONE,                          // SDL_SCANCODE_NONUSHASH
    A3D_OEM_COLON,                     // SDL_SCANCODE_SEMICOLON
    A3D_OEM_QUOTATION,                 // SDL_SCANCODE_APOSTROPHE
    A3D_OEM_TILDE,                     // SDL_SCANCODE_GRAVE
    A3D_OEM_COMMA,                     // SDL_SCANCODE_COMMA
    A3D_OEM_PERIOD,                    // SDL_SCANCODE_PERIOD
    A3D_OEM_SLASH,                     // SDL_SCANCODE_SLASH

    A3D_CAPSLOCK,                      // SDL_SCANCODE_CAPSLOCK

    A3D_F1,                            // SDL_SCANCODE_F1
    A3D_F2,                            // SDL_SCANCODE_F2
    A3D_F3,                            // SDL_SCANCODE_F3
    A3D_F4,                            // SDL_SCANCODE_F4
    A3D_F5,                            // SDL_SCANCODE_F5
    A3D_F6,                            // SDL_SCANCODE_F6
    A3D_F7,                            // SDL_SCANCODE_F7
    A3D_F8,                            // SDL_SCANCODE_F8
    A3D_F9,                            // SDL_SCANCODE_F9
    A3D_F10,                           // SDL_SCANCODE_F10
    A3D_F11,                           // SDL_SCANCODE_F11
    A3D_F12,                           // SDL_SCANCODE_F12

    A3D_PRINT,                         // SDL_SCANCODE_PRINTSCREEN
    A3D_SCROLLLOCK,                    // SDL_SCANCODE_SCROLLLOCK
    A3D_PAUSE,                         // SDL_SCANCODE_PAUSE
    A3D_INSERT,                        // SDL_SCANCODE_INSERT

    A3D_HOME,                          // SDL_SCANCODE_HOME
    A3D_PAGEUP,                        // SDL_SCANCODE_PAGEUP
    A3D_DELETE,                        // SDL_SCANCODE_DELETE
    A3D_END,                           // SDL_SCANCODE_END
    A3D_PAGEDOWN,                      // SDL_SCANCODE_PAGEDOWN
    A3D_RIGHT,                         // SDL_SCANCODE_RIGHT
    A3D_LEFT,                          // SDL_SCANCODE_LEFT
    A3D_DOWN,                          // SDL_SCANCODE_DOWN
    A3D_UP,                            // SDL_SCANCODE_UP

    A3D_NUMLOCK,                       // SDL_SCANCODE_NUMLOCKCLEAR

    A3D_NUMPAD_DIVIDE,                 // SDL_SCANCODE_KP_DIVIDE
    A3D_NUMPAD_MULTIPLY,               // SDL_SCANCODE_KP_MULTIPLY
    A3D_NUMPAD_SUBTRACT,               // SDL_SCANCODE_KP_MINUS
    A3D_NUMPAD_ADD,                    // SDL_SCANCODE_KP_PLUS
    A3D_NUMPAD_ENTER,                  // SDL_SCANCODE_KP_ENTER
    A3D_NUMPAD_1,                      // SDL_SCANCODE_KP_1
    A3D_NUMPAD_2,                      // SDL_SCANCODE_KP_2
    A3D_NUMPAD_3,                      // SDL_SCANCODE_KP_3
    A3D_NUMPAD_4,                      // SDL_SCANCODE_KP_4
    A3D_NUMPAD_5,                      // SDL_SCANCODE_KP_5
    A3D_NUMPAD_6,                      // SDL_SCANCODE_KP_6
    A3D_NUMPAD_7,                      // SDL_SCANCODE_KP_7
    A3D_NUMPAD_8,                      // SDL_SCANCODE_KP_8
    A3D_NUMPAD_9,                      // SDL_SCANCODE_KP_9
    A3D_NUMPAD_0,                      // SDL_SCANCODE_KP_0
    A3D_NUMPAD_DECIMAL,                // SDL_SCANCODE_KP_DECIMAL

    A3D_NONE,                          // SDL_SCANCODE_NONUSBACKSLASH
    A3D_APPS,                          // SDL_SCANCODE_APPLICATION
    A3D_NONE,                          // SDL_SCANCODE_POWER

    A3D_NONE,                          // SDL_SCANCODE_KP_EQUALS
    A3D_F13,                           // SDL_SCANCODE_F13
    A3D_F14,                           // SDL_SCANCODE_F14
    A3D_F15,                           // SDL_SCANCODE_F15
    A3D_F16,                           // SDL_SCANCODE_F16
    A3D_F17,                           // SDL_SCANCODE_F17
    A3D_F18,                           // SDL_SCANCODE_F18
    A3D_F19,                           // SDL_SCANCODE_F19
    A3D_F20,                           // SDL_SCANCODE_F20
    A3D_F21,                           // SDL_SCANCODE_F21
    A3D_F22,                           // SDL_SCANCODE_F22
    A3D_F23,                           // SDL_SCANCODE_F23
    A3D_F24,                           // SDL_SCANCODE_F24

    // Padding entries
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE,
    A3D_NONE
};
```

---

## 5. Design Notes

From `platform.h` comments:

### Why Key Codes Start at 1 (Not 0)

- **A3D_NONE = 0** is reserved for "no key" (invalid key code, unmapped keys)
- Backends map OS key codes (SDL_Scancode, X11 KeySym, Win32 VK_*) to KeyInfo enum
- Starting at 1 allows 0 to mean "translation failed" or "key not supported"

### A3D_AUTO_REPEAT Flag

- **A3D_AUTO_REPEAT = 256** is used as a flag bit
- Usage: `key | A3D_AUTO_REPEAT` marks an auto-repeat event
- Example: If user holds a key, subsequent events include this flag

### Translation Table Contract

- Both **A3D2SDL[]** and **SDL2A3D[]** must be updated together if `platform.h` KeyInfo enum changes
- These tables maintain bidirectional mapping between platform-independent key codes and SDL scancodes

### Input Event Flow

1. **OS Event**: User presses SPACE key
2. **SDL**: `SDL_PollEvent()` receives `SDL_KEYDOWN`, extracts `SDL_SCANCODE_SPACE`
3. **Backend**: Translates via `SDL2A3D[SDL_SCANCODE_SPACE]` to get `A3D_SPACE`
4. **Callback**: `wnd->platform_api.keyb_key(wnd, A3D_SPACE, true)`
5. **Game**: `game.cpp OnKeyb()` receives `A3D_SPACE`, triggers jump action
