# Asciicker Platform/Graphics Backends - C++ Architecture Documentation

This document provides comprehensive documentation of the Asciicker platform abstraction layer and its four backend implementations: SDL2, X11, Windows (Win32), and Web (Emscripten). The architecture enables cross-platform game execution while maintaining platform-specific optimizations for each target environment. Understanding this architecture is essential for anyone working on the Rust port, as it defines the contract between platform-agnostic game logic and platform-specific implementations.

## Overview

The Asciicker engine uses a layered architecture where the platform abstraction layer (platform.h) defines a contract that each backend must implement. This design allows the game engine to remain largely platform-agnostic while providing native performance on each target platform. The four backends serve different deployment scenarios: SDL2 for cross-platform desktop development, X11 for Linux systems requiring terminal integration, Win32 for native Windows performance, and Emscripten for browser deployment.

The platform selection occurs at compile time through preprocessor definitions. When USE_SDL is defined, the SDL2 backend is compiled. On Windows without SDL, the Win32 backend is used. On Unix-like systems (Linux, macOS) without SDL, the X11 backend is employed. The Web backend is compiled separately with Emscripten for WebAssembly deployment.

---

## 1. Platform Abstraction Layer (platform.h)

The platform abstraction layer defines the contract between the game engine and platform-specific implementations. This header file establishes the interface that all backends must implement, ensuring consistent behavior across platforms while allowing each backend to leverage native APIs.

### 1.1 Core Data Structures

The abstraction layer defines several critical data structures that form the foundation of the platform interface. These structures enable the game engine to interact with windows, graphics contexts, and input devices through a consistent API regardless of the underlying platform.

The PlatformInterface structure contains function pointers for all platform callbacks. This design uses C-style function pointers rather than C++ virtual methods to maintain compatibility with pure C code and avoid C++ runtime dependencies. Each callback corresponds to a specific game event or lifecycle stage, allowing the game engine to register handlers for window initialization, rendering, resizing, closing, keyboard input, character input, focus changes, and mouse events. The callbacks are optional, enabling headless server builds to leave input handling disabled.

```cpp
struct PlatformInterface
{
    void(*init)(A3D_WND* wnd);           // Called once when window is created
    void(*render)(A3D_WND* wnd);         // Called every frame (60 FPS target)
    void(*resize)(A3D_WND* wnd, int w, int h);  // Called when window size changes
    void(*close)(A3D_WND* wnd);          // Called when window is about to close
    void(*keyb_key)(A3D_WND* wnd, KeyInfo vk, bool down);  // Keyboard key press/release
    void(*keyb_char)(A3D_WND* wnd, wchar_t ch);  // Text input (Unicode character)
    void(*keyb_focus)(A3D_WND* wnd, bool set);   // Window focus changes
    void(*mouse)(A3D_WND* wnd, int x, int y, MouseInfo mi);  // Mouse movement/clicks
    void(*image)(void* cookie, int width, int height, int channels, int depth, void* data);  // Asset loader
    void(*sound)(void* cookie, int samples, int channels, int depth, void* data);  // Sound loader
};
```

The A3D_WND structure represents a window handle. Each backend implements this structure with platform-specific members while maintaining a consistent interface. The structure serves as the primary handle through which the game engine interacts with the windowing system. The structure contains pointers for linked list management (prev, next), platform-specific window and context handles, the platform interface callbacks, and user-defined cookie data.

The GraphicsDesc structure contains graphics initialization parameters that the game engine uses to configure the rendering context. This includes the OpenGL version (major and minor), color depth, alpha bits, depth bits, stencil bits, window dimensions and positioning, and window mode (normal, fullscreen, or frameless). The flags enum within GraphicsDesc specifies optional features like debug context and double buffering.

```cpp
struct GraphicsDesc
{
    enum FLAGS
    {
        DEBUG_CONTEXT = 1,     // Enable OpenGL debug context
        DOUBLE_BUFFER = 2      // Enable double buffering
    };

    int flags;
    int version[2];            // [0]:Major, [1]:Minor OpenGL version
    int color_bits;           // Color bits (including alpha)
    int alpha_bits;           // Dedicated alpha bits
    int depth_bits;           // Depth buffer bits
    int stencil_bits;         // Stencil buffer bits
    const int* wnd_xywh;     // Window position and size [x, y, width, height]
    WndMode wnd_mode;         // Window mode (normal, frameless, fullscreen)
};
```

### 1.2 Input Event Enums

The platform abstraction layer defines platform-independent enumerations for input events, abstracting away the differences between platform-specific input APIs. This design allows the game engine to work with a consistent representation of input events regardless of which platform is being used.

The KeyInfo enumeration defines platform-independent keyboard key codes. This enum starts at A3D_BACKSPACE = 1 rather than 0, with A3D_NONE = 0 reserved for "no key" or "translation failed." The design choice of starting at 1 allows 0 to indicate an invalid key code or unmapped keys. The enum includes standard keys (alphanumeric, function keys, modifiers), navigation keys (arrow keys, Page Up/Down, Home, End), numpad keys, and special keys (Escape, Tab, Enter, Delete, Insert). The A3D_AUTO_REPEAT flag (value 256) is used as a bit flag to indicate auto-repeat events.

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
    // ... continues with A-Z, 0-9, function keys, modifiers, etc.
    A3D_MAPEND,
    A3D_AUTO_REPEAT = 256   // Flag for auto-repeat events
};
```

The MouseInfo enumeration combines event types and state flags in a bitwise fashion. Event types (MOVE, LEFT_DN, LEFT_UP, RIGHT_DN, RIGHT_UP, MIDDLE_DN, MIDDLE_UP, WHEEL_UP, WHEEL_DN, ENTER, LEAVE) represent discrete events, while flags (LEFT, RIGHT, MIDDLE, INSIDE) represent current button state. This dual-purpose design allows the game to query both what happened (event type) and the current state (which buttons are held) in a single integer value.

```cpp
enum MouseInfo
{
    // Event types
    MOVE = 1,
    LEFT_DN = 2,
    LEFT_UP = 3,
    RIGHT_DN = 4,
    RIGHT_UP = 5,
    MIDDLE_DN = 6,
    MIDDLE_UP = 7,
    WHEEL_UP = 8,
    WHEEL_DN = 9,
    ENTER = 10,
    LEAVE = 11,

    // State flags
    LEFT = 0x10,
    RIGHT = 0x20,
    MIDDLE = 0x40,
    INSIDE = 0x80
};
```

### 1.3 Gamepad Interface

The LoopInterface structure defines optional gamepad callbacks. Gamepad support is considered optional because not all backends have equal gamepad capabilities, and some deployment scenarios (such as headless servers) do not require gamepad input. The SDL2 backend provides full gamepad support, while X11 and Win32 backends do not implement gamepad functionality.

```cpp
struct LoopInterface
{
    void(*gpad_mount)(const char* name, int axes, int buttons, const uint8_t mapping[]);  // Gamepad connected
    void(*gpad_unmount)();                    // Gamepad disconnected
    void(*gpad_button)(int b, int16_t pos);  // Button press/release
    void(*gpad_axis)(int a, int16_t pos);   // Axis movement
};
```

### 1.4 Timing API

The a3dGetTime() function returns the current time in microseconds as a monotonic timestamp. This function is critical for game timing, physics calculations, and frame rate control. The use of microseconds (rather than milliseconds or nanoseconds) provides sufficient precision for game timing (1/1000000 second granularity) while fitting within a 64-bit integer that won't wrap for hundreds of thousands of years. The implementation uses CLOCK_MONOTONIC on Unix systems and QueryPerformanceCounter on Windows, both of which provide monotonic (never-going-backwards) time sources that are unaffected by system clock adjustments.

---

## 2. SDL2 Backend (sdl.cpp)

The SDL2 backend serves as the primary cross-platform solution, supporting Windows, Linux, and macOS through a single codebase. This backend leverages the SDL2 library for windowing, input handling, and gamepad support while using OpenGL for rendering. The SDL2 backend is the most feature-complete, providing gamepad support and handling all platform features consistently.

### 2.1 Window Creation

Window creation in the SDL2 backend begins with initializing SDL itself and then creating a window with OpenGL support. The a3dOpen() function handles the entire window creation process, including context setup and platform API registration. The initialization process first checks if a shared context is requested, which allows multiple windows to share OpenGL resources.

The window is created using SDL_CreateWindow() with several flags: SDL_WINDOW_ALLOW_HIGHDPI ensures proper handling of high-DPI displays, SDL_WINDOW_OPENGL requests an OpenGL-compatible window, SDL_WINDOW_RESIZABLE enables window resizing, and SDL_WINDOW_HIDDEN creates the window in a hidden state to allow complete initialization before display. The window title is set to "ASCIIID SDL" by default.

After window creation, the OpenGL context is configured with the requested parameters from the GraphicsDesc structure. The backend sets attributes for depth size, stencil size, double buffering, context flags (including forward compatibility and debug mode if requested), OpenGL version, and context profile (core or compatibility). The context is created using SDL_GL_CreateContext() and verified to meet the minimum version requirements specified in the GraphicsDesc.

```cpp
wnd->win = SDL_CreateWindow("ASCIIID SDL", x, y, w, h,
    SDL_WINDOW_ALLOW_HIGHDPI |
    SDL_WINDOW_OPENGL |
    SDL_WINDOW_RESIZABLE |
    SDL_WINDOW_HIDDEN);
```

### 2.2 Event Loop

The SDL2 backend implements its event loop in the a3dLoop() function, which processes input events and triggers rendering. The loop follows a two-phase design: first, all pending events are processed, then all visible windows are rendered. This design ensures all input is processed before rendering begins.

The event processing phase uses SDL_PollEvent() in a loop to process all pending events. Each event type is translated from SDL's event structure callbacks. The SDL to the platform abstraction_QUIT event terminates the application by setting the running flag to false. Window events (SDL_WINDOWEVENT) are handled for close, resize, focus change, and mouse enter/leave events.

Keyboard events (SDL_KEYDOWN, SDL_KEYUP) are translated from SDL scancodes to the platform's KeyInfo enumeration using the SDL2A3D translation table. Text input events (SDL_TEXTINPUT) provide character input through the keyb_char callback. Mouse events include motion (SDL_MOUSEMOTION), button press and release (SDL_MOUSEBUTTONDOWN, SDL_MOUSEBUTTONUP), and wheel scrolling (SDL_MOUSEWHEEL).

Gamepad events are handled through SDL's game controller API. Device added events (SDL_CONTROLLERDEVICEADDED) open the gamepad and call the mount callback with the controller name, axes count, buttons count, and mapping information. Device removed events (SDL_CONTROLLERDEVICEREMOVED) close the gamepad and call the unmount callback, attempting to reconnect to another gamepad if available.

### 2.3 Graphics Initialization

Graphics initialization in the SDL2 backend involves configuring the OpenGL context attributes before creating the context. The backend sets attributes for the color buffer (red, green, blue, alpha), depth buffer, stencil buffer, and double buffering based on the GraphicsDesc structure. The context is created with forward compatibility enabled by default, which ensures the application doesn't use deprecated functionality.

The backend also handles context sharing, where multiple windows can share textures and other OpenGL resources. This is achieved by setting the SDL_GL_SHARE_WITH_CURRENT_CONTEXT attribute before creating additional windows. After context creation, the backend verifies that the created context meets the minimum version requirements.

### 2.4 Input Handling

Input handling in the SDL2 backend involves translating SDL's event structures to the platform abstraction callbacks. The backend maintains two translation tables: A3D2SDL maps platform KeyInfo codes to SDL scancodes for querying keyboard state, and SDL2A3D maps SDL scancodes to platform KeyInfo codes for event processing.

Keyboard events are processed by looking up the SDL scancode in the translation table. The backend handles special cases for modifier keys (left versus right Control, Shift, Alt, and Windows keys) that don't fit well in the translation table. If the key translation succeeds, the keyb_key callback is invoked with the platform key code and the pressed state.

Text input is handled separately from key events through the SDL_TEXTINPUT event. This event provides the actual character typed, which may differ from the key pressed due to keyboard layout, modifier keys (Shift for capitalization), and input method editors (IMEs).

Mouse handling combines event types with state flags. The current state of all mouse buttons is queried from SDL and combined with the specific event (such as LEFT_DN or MOVE) to form the complete MouseInfo value. This allows the game to know both what happened and the current button state in a single callback.

The SDL2 backend also implements mouse capture for improved tracking during drag operations. When a mouse button is pressed, mouse capture is enabled using SDL_CaptureMouse() to ensure all mouse events are delivered even if the cursor moves outside the window bounds. Capture is released when the button is released.

---

## 3. X11 Backend (x11.cpp)

The X11 backend provides native Linux/Unix support with direct access to the X11 windowing system and GLX for OpenGL rendering. This backend offers lower latency than SDL by avoiding the SDL translation layer and provides additional features not available in other backends, such as embedded terminal (PTY) support.

### 3.1 Window Creation

Window creation in the X11 backend is more complex than the SDL backend due to the verbose nature of the X11 API. The process involves opening a display connection, choosing a visual configuration, creating a colormap, creating the window, and finally creating the OpenGL context.

The backend first opens a connection to the X server using XOpenDisplay(). This connection is stored globally and used for all subsequent X11 operations. If no display is specified, the default display (from the DISPLAY environment variable) is used.

The backend then queries for a suitable framebuffer configuration using glXChooseFBConfig(). The configuration attributes are derived from the GraphicsDesc structure, specifying requirements for color depth, alpha, depth buffer, stencil buffer, and double buffering. Among all matching configurations, the backend selects the one that most closely matches the requested parameters using an error calculation that considers sample buffers, RGBA sizes, depth/stencil sizes, and other factors.

A visual is extracted from the chosen framebuffer configuration using glXGetVisualFromFBConfig(). A colormap is created for this visual using XCreateColormap(), which is necessary for the window to display colors correctly. The window is created using XCreateWindow() with event masks for all event types the backend needs to handle.

```cpp
static int visual_attribs[] =
{
    GLX_X_RENDERABLE, True,
    GLX_DRAWABLE_TYPE, GLX_WINDOW_BIT,
    GLX_RENDER_TYPE, GLX_RGBA_BIT,
    GLX_X_VISUAL_TYPE, GLX_TRUE_COLOR,
    GLX_RED_SIZE, color_bits[0],
    GLX_GREEN_SIZE, color_bits[1],
    GLX_BLUE_SIZE, color_bits[2],
    GLX_ALPHA_SIZE, color_bits[3],
    GLX_DEPTH_SIZE, gd->depth_bits,
    GLX_STENCIL_SIZE, gd->stencil_bits,
    GLX_DOUBLEBUFFER, True,
    None
};
```

The backend also sets up international text input support through X11's Input Method (XIM) infrastructure. If the locale is supported, an input context is created using XCreateIC(), which enables proper handling of multi-byte character input for international keyboards. The input context is created with XNInputStyle set to XIMPreeditNothing | XIMStatusNothing for simplicity.

The OpenGL context is created using the ARB extension glXCreateContextAttribsARB(), which allows specifying the OpenGL version and profile (core or compatibility) as context creation attributes.

### 3.2 Event Loop

The X11 event loop processes events using XNextEvent() in conjunction with XPending() to check for pending events without blocking. The loop follows the same two-phase design as the SDL backend: process all pending events first, then render all visible windows.

The event handling covers all necessary X11 event types. Client messages handle window close requests through the WM_DELETE_WINDOW protocol using XInternAtom(). ConfigureNotify events handle window resize by calling the resize callback with the new dimensions. FocusIn and FocusOut events are translated to the keyb_focus callback.

Key press events use two translation mechanisms. The keycode is translated to a KeyInfo value using the kc_to_ki translation table. For text input, if an input context is available, Xutf8LookupString() provides the actual characters typed, handling multi-byte UTF-8 sequences and composing characters correctly. If no input context is available, the simpler XLookupString() function provides basic ASCII input.

Mouse button events are translated from X11 button numbers (Button1, Button2, Button3 for left, middle, right, and Button4, Button5 for wheel up/down) to the platform's MouseInfo enumeration. Motion events include both the movement type and the current state of all mouse buttons.

### 3.3 Graphics Initialization

Graphics initialization in the X11 backend centers on the GLX context creation. After choosing a framebuffer configuration and creating the window, the backend creates an OpenGL context using glXCreateContextAttribsARB(). This function allows specifying context attributes including the major and minor version numbers, context flags (forward compatibility, debug), and profile mask (core or compatibility).

The backend verifies the created context meets the minimum version requirements by querying GL_VERSION with glGetString() and comparing it against the requested version from the GraphicsDesc structure. If the version is insufficient, the window creation fails and returns null.

For fullscreen support, the X11 backend uses Xinerama to query available monitors and their positions. When entering fullscreen mode, the backend sends client messages to the window manager requesting fullscreen on the monitor that most closely matches the window's current position.

### 3.4 Input Handling

Input handling in the X11 backend uses two translation tables: ki_to_kc maps platform KeyInfo codes to X11 keycodes (for querying keyboard state), and kc_to_ki maps X11 keycodes to platform KeyInfo codes (for event processing).

Keyboard state queries use XQueryKeymap() to get the current state of all keys. The keycode to platform key translation is performed using the kc_to_ki table, which covers the standard 128-key keyboard layout.

The X11 backend implements auto-repeat detection for key release events. When detectable auto-repeat is available (queried through XKB using XkbSetDetectableAutoRepeat()), the backend relies on it. Otherwise, the backend peeks at the next event to determine if the release event is a genuine release or an auto-generated repeat of the previous press.

---

## 4. Windows Backend (mswin.cpp)

The Windows backend provides native Windows support using the Win32 API for windowing and WGL for OpenGL rendering. This backend offers the lowest latency on Windows systems and provides full integration with Windows features like DPI awareness and multi-monitor support.

### 4.1 Window Creation

Window creation in the Windows backend involves registering a window class and creating a window using the Win32 API. The process begins with registering a window class if no windows exist yet.

```cpp
WNDCLASS wc;
wc.style = 0;
wc.lpfnWndProc = a3dWndProc;
wc.cbClsExtra = 0;
wc.cbWndExtra = 0;
wc.hInstance = GetModuleHandle(0);
wc.hIcon = LoadIcon(0, IDI_APPLICATION);
wc.hCursor = LoadCursor(0, IDC_ARROW);
wc.hbrBackground = NULL;
wc.lpszMenuName = NULL;
wc.lpszClassName = L"A3DWNDCLASS";

if (!wnd_head && !RegisterClass(&wc))
    return false;
```

The window is created using CreateWindow() with standard window styles including caption, system menu, minimize and maximize boxes, and resizing borders. The window is created with default positioning and size (CW_USEDEFAULT), which is later adjusted if specific dimensions are provided in the GraphicsDesc.

The OpenGL initialization uses the Windows pixel format API. A pixel format descriptor is configured based on the GraphicsDesc parameters, specifying RGBA color, depth buffer, stencil buffer, and double buffering requirements. ChoosePixelFormat() selects an appropriate pixel format, and SetPixelFormat() applies it to the device context.

The WGL context is created in two steps for modern OpenGL support. First, a basic context is created using wglCreateContext(). Then, if the WGL_ARB_create_context extension is available, the preferred context is created with the requested version and profile using wglCreateContextAttribsARB(). This two-step approach ensures a fallback for systems without the extension while supporting modern OpenGL when available.

### 4.2 Event Loop

The Windows backend implements its event loop using the standard Win32 message processing: PeekMessage() with PM_REMOVE to retrieve messages without blocking, TranslateMessage() to process keyboard translation, and DispatchMessage() to deliver messages to the window procedure.

The a3dWndProc() function serves as the central event dispatcher, handling all window messages and translating them to platform abstraction callbacks. This function is registered as the window procedure during window class registration.

Window messages handled include WM_CLOSE for closing the window, WM_SIZE for resize notifications, WM_PAINT for paint requests (which are validated immediately as the game handles its own rendering), WM_TIMER for rendering during menu and size/move operations, and WM_ENTERMENULOOP and WM_ENTERSIZEMOVE to enable continuous rendering during these operations.

Keyboard messages (WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP) are translated from virtual key codes to platform KeyInfo codes using the vk_to_ki translation table. The translation handles extended keys (distinguishing left and right modifier keys and numpad Enter from regular Enter), numpad key behavior when NumLock is enabled or disabled, and auto-repeat detection by checking the bit 30 of the lParam.

Text input is handled through the WM_CHAR message, which provides the Unicode character (UTF-16) directly. This is separate from key events to properly handle character input versus key input.

Mouse messages are handled comprehensively: WM_MOUSEMOVE for movement with enter tracking, WM_LBUTTONDOWN/UP, WM_RBUTTONDOWN/UP, WM_MBUTTONDOWN/UP for button events, WM_MOUSELEAVE for when the mouse leaves the window using TrackMouseEvent(), and WM_MOUSEWHEEL for wheel events with delta processing.

### 4.3 Graphics Initialization

Graphics initialization in the Windows backend configures the pixel format and creates the WGL context. The pixel format selection considers the requested color depth, alpha bits, depth buffer, stencil buffer, and double buffering requirements from the GraphicsDesc.

The context creation uses the WGL_ARB_create_context extension when available to create an OpenGL 3.x+ context with forward compatibility. If the extension is not available, the backend falls back to a basic context creation.

The backend also initializes high-resolution timing on first window creation by setting up QueryPerformanceFrequency() and QueryPerformanceCounter(). A timer is set to fire every minute (60000ms) to refresh the coarse timing values and prevent overflow in the time calculation.

### 4.4 Input Handling

Input handling in the Windows backend uses two translation tables: ki_to_vk maps platform KeyInfo codes to Windows virtual key codes (for querying keyboard state), and vk_to_ki maps Windows virtual key codes to platform KeyInfo codes (for event processing).

Keyboard state queries use GetKeyState() with the virtual key code to determine if a key is currently pressed. This function returns the key state with the high bit set if the key is down.

Mouse input handling includes tracking the mouse position within the window, tracking button states, and implementing mouse capture during button press operations using SetCapture() and ReleaseCapture(). TrackMouseEvent() is used to receive WM_MOUSELEAVE notifications when the cursor exits the window.

---

## 5. Web Backend (game_web.cpp)

The Web backend targets browser deployment through Emscripten/WebAssembly. This backend differs significantly from the native backends due to the unique constraints and capabilities of the browser environment. The primary differences include cooperative main loop scheduling, virtual filesystem for persistence, and JavaScript-based input handling.

### 5.1 Window Creation and Initialization

In the web backend, "window creation" refers to the canvas setup and initialization that occurs in JavaScript rather than through explicit C++ calls. The actual window (HTML canvas element) is created in the HTML file loaded by the browser, and the C++ code receives callbacks for various events.

The initialization sequence begins with the JavaScript side setting up event listeners for keyboard, mouse, touch, and gamepad input. The C++ Main() function is called from JavaScript after the Emscripten runtime is fully initialized.

The Main() function performs several critical initialization steps. First, it allocates a shared memory buffer for C++ and JavaScript communication through the akAPI_Buff buffer. This buffer enables efficient data exchange between the two environments without expensive marshalling. Next, the function initializes the JavaScript API system (akAPI_Init()) and sets up sandboxing for user scripts.

```cpp
int Main()
{
    // Allocate shared memory buffer for C++ <-> JavaScript communication
    akAPI_Buff = malloc(AKAPI_BUF_SIZE);
    memset(akAPI_Buff, 0, AKAPI_BUF_SIZE);

    // Initialize akAPI scripting system
    akAPI_Init();

    // Initialize all 256 terrain material definitions
    InitMaterials();

    // Load sprite graphics (player, enemies, items)
    LoadSprites();

    // Allocate ASCII render buffer (max 160x160 cells)
    render_buf = (AnsiCell*)malloc(sizeof(AnsiCell) * 160 * 160);

    // Create main game state object
    game = CreateGame();

    return 0;
}
```

### 5.2 Event Loop

The web backend uses a cooperative main loop implemented through Emscripten's emscripten_set_main_loop() function. This function registers a callback that the browser calls via requestAnimationFrame, which provides several advantages and constraints compared to native event loops.

The key difference from native platforms is that the browser controls the frame rate, typically syncing to the display refresh rate (often 60 FPS). The callback cannot block, as this would freeze the browser tab. Instead, the game must complete its work within the time allotted per frame and return control to the browser.

The main loop callback is Render(), which receives the canvas dimensions from JavaScript and returns a pointer to the render buffer containing the ASCII character data to display. The JavaScript side handles all platform interaction, input event collection, and display updates, while the C++ side focuses on game logic and rendering.

### 5.3 Graphics Initialization

Graphics initialization in the web backend differs significantly from native backends because rendering occurs through WebGL rather than native OpenGL. The JavaScript side creates the WebGL context and handles all rendering commands.

From the C++ perspective, graphics initialization is minimal. The game allocates a render buffer (render_buf) that holds the ASCII character data for each cell. The buffer is sized to accommodate the maximum possible display dimensions (160x160 cells).

The actual rendering of this buffer to the canvas is handled by JavaScript, which reads the buffer contents and renders them using the canvas API or WebGL. This separation allows the same C++ rendering code to work across all platforms while letting each platform handle the final display in its native way.

### 5.4 Input Handling

Input handling in the web backend is primarily implemented in JavaScript, which captures browser input events and passes them to C++ through exported functions. The JavaScript side maintains event listeners for keyboard, mouse, touch, and gamepad input.

Keyboard input is captured through JavaScript keydown and keyup event listeners. When a key event occurs, the JavaScript code determines the key type and calls the C++ Keyb() function with the event type (keydown or keyup) and the key value.

Mouse input is handled similarly, with JavaScript capturing mousemove, mousedown, mouseup, and wheel events. The Mouse() function is called with the event type and coordinates.

Touch input supports multi-touch scenarios on mobile devices. Each touch event includes a touch identifier to track individual fingers, along with the coordinates and event type (touchstart, touchmove, touchend).

Gamepad input uses the HTML5 Gamepad API, which is polled each frame rather than event-driven. The JavaScript side monitors connected gamepads and calls GamePad() with the appropriate event type, button or axis index, and value.

The web backend also handles focus changes through the Focus() function, which is called when the browser tab gains or loses focus. This allows the game to pause when the user switches to another tab.

---

## 6. Backend Comparison

The following summary table captures the key differences between the four backend implementations across various functional areas.

| Feature | SDL2 | X11 | Win32 | Web |
|---------|------|-----|-------|-----|
| **Platform Targets** | Windows, Linux, macOS | Linux, Unix, macOS (XQuartz) | Windows | Browsers (WebAssembly) |
| **Windowing API** | SDL2 | X11 | Win32 | HTML5 Canvas |
| **Graphics API** | OpenGL (via SDL) | GLX | WGL | WebGL |
| **Input Method** | SDL Events | X11 Events | Win32 Messages | JavaScript Callbacks |
| **Gamepad Support** | SDL_GameController | None | None | HTML5 Gamepad API |
| **Text Input** | SDL_TEXTINPUT | XIM/XIC + Xutf8LookupString | WM_CHAR | JavaScript Events |
| **Terminal (PTY) Support** | No | Yes | No | No |
| **Timing** | clock_gettime / QueryPerformanceCounter | clock_gettime | QueryPerformanceCounter | Emscripten (performance.now) |
| **Persistence** | Native filesystem | Native filesystem | Native filesystem | IndexedDB (IDBFS) |

---

## 7. Key Implementation Details

### 7.1 Translation Tables

Each backend maintains translation tables to map between platform-specific key codes and the platform-independent KeyInfo enumeration. These tables must be kept synchronized with the enum definition in platform.h. The SDL2 backend uses A3D2SDL[] (platform to SDL) and SDL2A3D[] (SDL to platform). The X11 backend uses ki_to_kc[] and kc_to_ki[]. The Windows backend uses ki_to_vk[] and vk_to_ki[]. The web backend receives pre-translated values from JavaScript and does not maintain translation tables.

### 7.2 Context Management

All backends support OpenGL context management through the a3dPushContext(), a3dPopContext(), and a3dSwitchContext() functions. These functions save and restore the current OpenGL context, allowing the game engine to work with multiple windows while ensuring the correct context is active for each operation. The push/pop pattern uses a local object that saves the current context in its constructor and restores it in the destructor, ensuring proper cleanup even if an exception occurs.

### 7.3 Thread Safety

The backends are designed to run on a single thread for the main event loop. However, they do provide thread creation and synchronization primitives through a3dCreateThread(), a3dCreateMutex(), and related functions. These are implemented using platform-specific APIs: pthreads on Unix systems (SDL and X11), and Windows threads on Windows. The web backend would require Web Workers for threading, though this is currently unused in the implementation.

---

## 8. Conclusion

The Asciicker platform abstraction layer demonstrates a well-designed approach to cross-platform game development. By defining a clear contract through the PlatformInterface structure and supporting data types, the architecture enables the game engine to remain largely platform-agnostic while allowing each backend to leverage native APIs for optimal performance.

The four backend implementations each serve specific purposes: SDL2 for maximum cross-platform compatibility, X11 for Linux systems requiring terminal integration, Win32 for native Windows performance, and Emscripten for browser deployment. This flexibility allows the same C++ game code to run across desktop operating systems and web browsers with minimal platform-specific adaptations.

The architecture supports optional features through callback pointers (such as gamepad support), enabling different feature sets on different platforms while maintaining a consistent base API. The use of compile-time platform selection through preprocessor definitions ensures minimal runtime overhead while providing maximum flexibility.

Understanding this architecture is essential for anyone working on the Rust port, as the new implementation must either replicate this multi-backend design or choose to focus on a subset of platforms. The key insight is that the platform abstraction layer separates game logic from platform-specific details, making it possible to port the engine to new platforms or rewrite in a different language while maintaining the same high-level interface.
