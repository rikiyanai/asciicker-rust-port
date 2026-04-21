# Asciicker Main Menu System Architecture

**Source File:** `/Users/rikihernandez/Downloads/Aciicker-Y9-2/mainmenu.cpp` (2838 lines)  
**Header File:** `/Users/rikihernandez/Downloads/Aciicker-Y9-2/mainmenu.h` (68 lines)  
**Purpose:** Multi-platform main menu system with stack-based hierarchical navigation, smooth scrolling, level loading flow, and background rendering with half-tone dithering.

---

## 1. Menu System Architecture

The Asciicker menu system implements a **stack-based hierarchical navigation model** that supports multiple levels of menu nesting with a fixed maximum depth. This architecture provides a clean separation between menu structure definition and rendering logic, enabling easy extension and modification of menu hierarchies without changes to the core rendering code.

### 1.1 State Management

The menu system maintains its state through a set of interconnected variables that track the current navigation depth, highlighted item, and input capture state. The depth system allows users to navigate through nested submenus while preserving their position within each level, creating a natural breadcrumb trail through the menu hierarchy.

The primary state variables are defined in the `MainMenuContext` structure:

- **`menu_depth`**: An integer ranging from -1 to 3, representing the current navigation level within the menu hierarchy. A depth of -1 indicates the menu is closed (though this state is currently unused in the implementation). Depth 0 represents the root menu, while depths 1 through 3 represent increasingly nested submenu levels. This limited depth prevents excessive nesting that could confuse users while providing sufficient flexibility for typical game menu structures.

- **`menu_stack[4]`**: An integer array that stores the index of the currently highlighted menu item at each depth level. When navigating to a submenu, the previously selected item index is preserved in this array, allowing users to return to their previous position within any menu level. The stack operates such that `menu_stack[menu_depth]` always contains the index of the currently selected item.

- **`menu_temp`**: An integer that stores the keyboard or gamepad highlight position when mouse or touch input takes over. This variable serves as a preservation mechanism: when the mouse hovers over menu items, the keyboard highlight is set to -1 (no selection), but `menu_temp` retains the last valid keyboard position. When the user presses arrow keys again, the highlight restores from `menu_temp`, providing a seamless transition between input methods.

- **`menu_down`**: An integer with three possible states representing the current input capture status: 0 indicates no input is captured (the menu is in a neutral state), 1 indicates the mouse has captured input (blocking keyboard input), and 2 indicates touch has captured input (blocking both mouse and keyboard). This mutual exclusion prevents input conflicts when multiple input devices are used simultaneously.

### 1.2 Menu Structure Definition

The menu structure is defined through the `MainMenu` structure, which enables a data-driven approach to menu definition. This design allows menu hierarchies to be constructed entirely through static const arrays, eliminating runtime allocation and enabling compile-time verification of menu structures.

```cpp
struct MainMenu {
    const char* str;                    // Display string (NULL = terminator)
    const MainMenu* sub;                // Submenu array (NULL = leaf item)
    void (*action)(MainMenuContext*);  // Action callback (NULL = no action)
    bool (*getter)(MainMenuContext*);  // State getter for toggles
    void* cookie;                       // User data for action
};
```

Each field serves a specific purpose in defining menu behavior. The `str` field contains the text displayed for the menu item, with a NULL value serving as the terminator for menu arrays. The `sub` field points to an array of child menu items for submenus, or NULL for leaf items that perform actions. The `action` field holds a callback function executed when the user selects a leaf item, while the `getter` field provides a function that returns the current state for toggle items (such as mute or fullscreen). The `cookie` field passes arbitrary user data to action callbacks, enabling flexible parameter passing.

The static const array approach offers several advantages: menus are defined at compile time with no runtime allocation overhead, the hierarchical structure is immediately visible in the code, and modification of menu items requires only changes to the array definitions without touching rendering logic.

### 1.3 Loading State Machine

The level loading system implements an asynchronous state machine that spreads loading across multiple frames to prevent freezing the user interface. The `game_loading` variable tracks the overall loading state with three distinct values: 0 indicates no level is loaded and the continue option should not be shown, 1 indicates loading is in progress and the loading screen should be displayed, and 2 indicates the level is fully loaded and the continue option should be available.

Within the loading process, the `mainmenu_context.progress` variable provides fine-grained tracking of loading stages, counting downward from 3 to 0 to indicate completion. A progress value of 3 represents the initialization phase, 2 indicates patch loading (the bulk of the loading work), 1 represents world rebuilding, and 0 indicates loading is complete. This countdown approach simplifies completion detection through a simple equality comparison.

---

## 2. UI Rendering

The menu rendering system combines several techniques to produce the distinctive ASCII aesthetic of Asciicker. The rendering pipeline handles background image scaling with half-tone dithering, sprite compositing, font rendering for menu text, and animated transitions through dither effects. Each rendering stage contributes to the overall visual presentation while maintaining the retro terminal aesthetic.

### 2.1 Background Rendering with Half-Tone Dithering

The background rendering system scales a source image to fit the screen while applying a sophisticated half-tone dithering algorithm that converts continuous color values into the limited ANSI color palette. This process involves gamma correction, palette quantization, and error diffusion dithering to produce the characteristic retro visual style.

The pipeline begins with gamma correction through the `Gamma` structure, which provides lookup tables for converting between sRGB and linear color spaces. The decode table (`dec[256]`) converts 8-bit sRGB values to 16-bit linear values in the range 0-8192, while the encode table (`enc[8193]`) performs the reverse transformation. This separation allows the dithering algorithm to operate in linear space where color blending produces perceptually accurate results.

The `ScaleImg` function implements the core rendering algorithm. It samples the source image at four corners of each destination cell, applies error diffusion from previous pixels, evaluates three encoding strategies (half-tone, bottom-top, and left-right dithering), selects the strategy with minimum weighted error, and distributes quantization errors to neighboring pixels. The weighted error formula prioritizes green (weight 3) over red (weight 2) and blue (weight 1), reflecting the human eye's varying sensitivity to different colors.

The palette system uses a 6x6x6 RGB cube with 216 colors, where each channel takes values 0, 51, 102, 153, 204, or 255. The `half_tone` lookup table precomputes the result of blending two palette colors, enabling fast dither pattern generation. The inverse palettizer loaded from `palette.gz` provides a mapping from arbitrary RGB values to the nearest palette colors.

### 2.2 Menu Item Rendering

Menu items are rendered through the `MainMenuContext::Paint` method, which handles title display, item listing, highlighting, and smooth scrolling. The rendering positions menu items at the right side of the screen (x = width - 5) with vertical spacing determined by font height plus one pixel of padding.

The title rendering displays the path to the current menu location, built by traversing the menu hierarchy and concatenating parent menu names with a prefix character. The title appears at the top of the menu area with an underline rendered in pink skin tone. Menu items below the title are rendered in either gold (for the currently highlighted item) or grey (for unselected items), providing clear visual feedback about the current selection state.

Item suffixes indicate menu item types: a right arrow character ("\x03") appears next to items that have submenus, while toggle items display either an on symbol ("\x02") or off symbol ("\x01") based on their getter function state. This visual encoding allows users to immediately identify which actions will occur upon selection.

### 2.3 Smooth Scrolling System

The scrolling system provides smooth animation when navigating through menus with many items. Two variables track scrolling state: `menu_scroll` holds the target scroll position (set directly during navigation), and `menu_smooth_scroll` holds the interpolated value used for rendering. Each frame, the smooth scroll value moves one unit toward the target, creating a linear interpolation effect that takes approximately 16 milliseconds per pixel at 60 frames per second.

The auto-scroll feature ensures the highlighted item remains visible on screen. After keyboard or gamepad navigation, the `menu_rescroll` flag triggers a calculation that adjusts the scroll position if the highlighted item has moved outside the visible area. This automatic adjustment prevents users from losing track of their selection when navigating through long menus.

### 2.4 Dither Fade-In Animation

The menu implements a fade-in effect through the `mainmenu_dither` counter, which controls the transparency of sprites rendered over the background. The counter starts at twice the `mainmenu_dither_hidden` value (40) and decrements at approximately 60 frames per second until reaching zero. The `SetSpriteDither` function divides this value by two (through right-shift) when passing it to the sprite renderer, creating a gradual transition from fully dithered (transparent) to solid appearance.

This animation triggers on several occasions: when entering submenus, when popping back to parent menus, when toggling settings like fullscreen or zoom, and when opening or closing the virtual gamepad. The consistent animation provides visual feedback for state changes without requiring explicit animation code for each transition.

---

## 3. Menu Navigation

The menu system supports four distinct input methods: keyboard, mouse, touch, and gamepad. Each input method maps to a common internal state machine, ensuring consistent behavior regardless of how the user interacts with the menu. The input handling code prioritizes inputs to prevent conflicts when multiple devices are used simultaneously.

### 3.1 Input Priority System

The priority system ensures that only one input method controls the menu highlight at any given time, preventing contradictory state updates. Touch input has the highest priority: when a touch is in progress (menu_down equals 2), mouse and keyboard inputs are ignored. Mouse input has the second priority: when the mouse button is held (menu_down equals 1), keyboard input is blocked. Keyboard and gamepad inputs share the lowest priority and directly modify the menu stack without capture restrictions.

This priority system resolves naturally through the `menu_down` variable, which transitions between states based on input events. Touch events set menu_down to 2, mouse events set it to 1, and releasing either input sets it back to 0. The input handlers simply check this variable at the start of processing and return early if the input should be ignored.

### 3.2 Keyboard Navigation

Keyboard navigation uses the arrow keys for menu traversal, Enter or Right arrow for selection, and Escape or Left arrow for going back. The handler distinguishes between key press types (KEYB_CHAR for character keys, KEYB_DOWN for initial presses, KEYB_PRESS for held keys) to filter duplicate events and handle key repeat appropriately.

When pressing Up or Down, the handler first restores the highlight from `menu_temp` if it was previously -1 (meaning mouse had taken over), then increments or decrements the selection index while checking bounds against the current menu array. The `menu_rescroll` flag is set to trigger auto-scroll after rendering.

When pressing Enter or Right on a submenu item, the handler increments `menu_depth` to descend into the submenu, resets the scroll position, and sets the new highlight to the first item. When pressing on a leaf item with an action callback, it executes the callback. When pressing Escape or Left, it decrements `menu_depth` to return to the parent menu.

### 3.3 Mouse Navigation

Mouse navigation provides direct selection through hit testing. The `HitMenu` function converts screen coordinates to cell coordinates, then tests against the bounding boxes of menu items to determine which item (if any) the mouse is over. Return values indicate: -3 for closed menu, -2 for no hit, -1 for title hit (back button), and non-negative values for item indices.

Mouse button down captures input and performs an initial hit test. The handler stores the initial cell coordinates for drag scrolling and sets `menu_down` to 1 to capture subsequent input. The hit result is stored as the current highlight, and `menu_temp` is updated to preserve this position for keyboard restoration.

Mouse button up releases capture and executes actions. The handler performs another hit test to verify the mouse is still over the same item (this allows click-and-drag cancellation). If the item is still selected, it either pops a menu level (for back navigation), descends into a submenu, or executes an action callback. Finally, it sets `menu_down` to 0 and clears the highlight.

Mouse movement while the button is held enables drag scrolling. The vertical distance traveled becomes the scroll offset, and the highlight is cleared during scrolling to prevent visual inconsistency. Mouse wheel events provide discrete scroll increments of 5 pixels per notch.

### 3.4 Touch Navigation

Touch navigation closely mirrors mouse navigation with event-specific handling. Touch begin captures input similarly to mouse button down, touch move handles drag scrolling, and touch end performs the final selection action. The touch handler only processes the first touch point (id equals 1), ignoring multi-touch gestures to maintain simple interaction.

Touch cancel releases capture without executing any action, providing a clean way to abort touches that were interrupted by system events like incoming calls.

### 3.5 Gamepad Navigation

Gamepad navigation uses button indices mapped to menu actions. The primary buttons (0 for face button, 1 for secondary) provide selection and cancellation. The directional pad buttons (11-14) map to up, down, left, and right directions with equivalent behavior to keyboard arrows. Additional buttons like shoulder buttons and special function buttons can trigger specific actions or remain unmapped.

The gamepad handler operates similarly to the keyboard handler: it restores highlight from `menu_temp` if needed, sets `menu_rescroll` for auto-scroll, executes actions on button 0, and pops or descends menu levels based on directional input.

---

## 4. Main Menu Screens

The menu hierarchy consists of a root menu with several submenus for different functionality areas. Each menu is defined as a static const array of `MainMenu` structures, with the root menu serving as the entry point and submenus providing access to specific feature areas. The menu structure reflects typical game menu organization with options for starting games, adjusting settings, and accessing tools.

### 4.1 Root Menu

The root menu (`mainmenu_root`) provides the top-level navigation options. Its entries include: CONTINUE (which appears conditionally when a game is already loaded), NEW GAME (which initiates the level loading process), PROFILE (currently a placeholder), CREDITS (currently a placeholder), VIDEO (submenu for display settings), CONTROLS (submenu for input configuration), TOOLS (desktop-only submenu for development utilities), MUTE SOUND (toggle for audio), and EXIT (submenu for confirmation).

The CONTINUE item uses a getter function to determine whether it should appear, showing only when `game_loading` equals 2 (a level is loaded and ready). The `MainMenuGetRoot` function returns a pointer offset by one position when continue is not shown, effectively skipping that entry in the array.

### 4.2 Video Settings Submenu

The VIDEO submenu (`main_menu_video`) provides display configuration options. ZOOM IN and ZOOM OUT adjust the font size through the `NextGLFont` and `PrevGLFont` functions, triggering dither animations on success. FULL SCREEN toggles the display mode through platform-specific fullscreen toggling, with a getter function that also monitors for external fullscreen changes (such as windowed mode toggling via Alt-Enter). PERSPECTIVE switches between perspective and orthographic projection modes. SHOW BLOOD toggles blood particle effects in the game world.

Each toggle item pairs an action callback that modifies the setting with a getter function that reports the current state. The getter functions ensure the menu display accurately reflects the current configuration, and some getters also trigger dither animations when detecting changes.

### 4.3 Controls Submenu

The CONTROLS submenu (`main_menu_controls`) lists available input methods: KEYBOARD, MOUSE, TOUCH, and GAMEPAD. The first three are currently informational placeholders, while GAMEPAD opens the virtual gamepad overlay. The `main_menu_gamepad` action callback sets `show_gamepad` to true and opens the gamepad interface through `GamePadOpen`, registering a close callback that restores the menu display.

### 4.4 Tools Submenu

The TOOLS submenu (`main_menu_tools`) provides access to development utilities and is only available on desktop builds (hidden for Emscripten web builds). WORLD EDITOR launches the asciiid world editor through the `./.run/asciiid` script. XP SPRITE EDITOR launches the Python XP sprite editor. ASSET GENERATOR launches the Python asset generation tool.

These launchers use platform-specific commands: macOS uses osascript to open a new Terminal.app window with the appropriate command, Linux runs commands in the background with the ampersand operator, and Windows uses the start command. The launchers capture the return code and print error messages if spawning fails.

### 4.5 Exit Confirmation

The EXIT submenu (`main_menu_exit`) provides a confirmation dialog with NO and YES options. The NO option pops back to the parent menu through `main_menu_no_exit`, while the YES option terminates the program through either `exit(0)` (for SDL builds) or `exit_handler(0)` (for other platforms).

### 4.6 Level Manifest System

The manifest system (`manifest` array) defines available game content through a collection of `Manifest` structures. Each entry specifies an XP preview sprite, title, description, A3D world file path, and optional AJD game script. The manifest supports several usage patterns: terminator entries with all NULL fields, directory entries with a NULL a3d field and ajs pointing to child manifests, server game entries with ajs NULL and a3d containing a server address, and ad entries with both a3d and ajs NULL and cookie pointing to a URL.

The current manifest includes: CONTROLS TUTORIAL (a basic tutorial world), Y9 DEMO (the main playable demo with quests), Y9 MULTIPLAYER DEMO (multiplayer demonstration), DEV TOYS (a directory of development examples), and GUMIX NEWS (an advertisement entry).

---

## 5. HUD Rendering

The Asciicker menu system serves a dual role as both the main menu and a persistent background layer for the game HUD. When the game is running, the menu system continues rendering beneath the 3D game view, providing visual continuity and allowing quick access to the menu through the ESC key. This design treats the menu as a fundamental part of the UI layer rather than a separate mode that completely replaces game rendering.

### 5.1 Persistent Background Layer

The menu rendering operates continuously regardless of game state, with the `game->main_menu` flag controlling whether the game or menu receives primary focus. When the flag is true, the menu occupies the full screen as the visual focus. When false, the game renders on top of the menu background, but the menu rendering still occurs as a base layer.

This persistent rendering approach enables several design benefits: the transition between menu and gameplay is seamless, the menu can remain accessible at all times, and the background imagery provides consistent visual atmosphere regardless of what mode the application is in. The background image with character sprites creates an immersive environment even when the actual gameplay occurs.

### 5.2 Loading Screen Overlay

During level loading (when `game_loading` equals 1), the menu system displays a loading overlay that communicates progress to the user. The overlay renders "LOADING" text at the top of the screen in pink skin tone, followed by a row of ten progress dots that fill from left to right as loading completes.

The loading UI distinguishes between three progress states through dot coloring: when `progress` equals 3 (initialization), all dots appear grey; when `progress` equals 2 (patch loading), dots fill proportionally based on the ratio of `patch_iter` to `patch_num`; when `progress` equals 0 or 3 (finalization or complete), all dots appear gold. This visual feedback allows users to gauge loading progress and estimate remaining time.

The loading process calls `LoadGame` each frame to advance through its asynchronous state machine. Each call may perform a portion of the loading work (such as processing a batch of terrain patches), then return control to allow the UI to update. When `progress` reaches 0, the loading completes, `game_loading` transitions to 2, and `game->main_menu` is set to false to begin gameplay.

### 5.3 Virtual Gamepad Overlay

The virtual gamepad (`show_gamepad` flag) provides on-screen touch controls for mobile and web builds. When activated, it replaces the menu display with a gamepad interface rendered through `PaintGamePad`. Input is routed through special handlers that translate keyboard, mouse, and touch events into gamepad button and contact events.

The gamepad overlay has its own input routing: instead of processing events through the standard menu handlers, they translate to virtual button presses and touch contacts that the gamepad system interprets. This translation enables touch-based gameplay without requiring native gamepad support.

The close callback (`main_menu_gamepad_close`) restores normal menu display when the user dismisses the gamepad, resetting `show_gamepad` to false and triggering a dither animation to smooth the transition.

### 5.4 Screenshot Capture

The menu system supports screenshot capture through the F10 key, which sets the `mainmenu_shot` flag. On the next render frame, this flag triggers a screenshot write operation that saves the current frame buffer to `./shot.xp` and metadata to `./shot.json`. The XP format stores each cell as a 32-bit character code followed by foreground and background RGB triplets, preserving the full visual state of the menu.

The screenshot feature encodes the ANSI color palette by converting from the internal 6x6x6 palette representation back to RGB values metadata includes. The JSON the version, timestamp, context (set to "main_menu" for menu screenshots), screen dimensions, and the path to the currently loaded level (if any).

---

## Summary

The Asciicker main menu system implements a comprehensive UI framework through several interconnected components. The stack-based hierarchical navigation provides intuitive multi-level menu traversal with consistent state management. The rendering pipeline combines half-tone dithering, font rendering, and sprite compositing to achieve the distinctive ASCII aesthetic. Multi-platform input handling through keyboard, mouse, touch, and gamepad ensures accessibility across different devices and use cases. The persistent background layer design treats the menu as an ever-present UI component rather than a replaceable mode, enabling seamless transitions and continuous accessibility.

The architecture demonstrates careful consideration of user experience through features like smooth scrolling, auto-scroll, input priority handling, and visual feedback through dither animations. The data-driven menu definition enables easy extension without modifying core rendering logic, while the asynchronous loading system prevents interface freezing during level transitions.
