> **STATUS: ACTIVE REFERENCE** — Input system integration analysis is valid. Bevy's input handling replaces both Mage Core's limited input and the C++ platform backends.

# Bevy Input System Research: Replacing Mage-core's Minimal Input

## Overview

This document analyzes Bevy's comprehensive input system to replace Mage-core's minimal `input.rs` with a full-featured input handling solution. Bevy provides a rich, ECS-based input architecture that supports keyboard, mouse, gamepad, and touch inputs through both event-driven and polling-based patterns.

---

## 1. Keyboard Input

### Key Types

Bevy provides two distinct key representations:

#### KeyCode (Physical Key Location)
- Represents the **physical location** of a key on the keyboard
- Layout-independent: `KeyQ` is always the left part of the letter row regardless of QWERTY/AZERTY
- Based on the W3C UI Events Specification

```rust
use bevy_input::prelude::KeyCode;

// Common key codes
KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD  // WASD
KeyCode::Space, KeyCode::Escape, KeyCode::Enter
KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::ArrowLeft, KeyCode::ArrowRight
KeyCode::F1, KeyCode::F2,  // Function keys
KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::AltLeft  // Modifiers
```

#### Key (Logical Key)
- Represents the **actual character** produced, considering keyboard layout
- Layout-dependent: pressing the same physical key produces different `Key` values on different layouts

```rust
use bevy_input::keyboard::Key;

Key::Character("a".into())  // Character input
Key::Space, Key::Enter, Key::Escape  // Named keys
Key::Alt, Key::Control, Key::Shift  // Modifier keys
```

### Input Resource (Polling)

```rust
use bevy_input::ButtonInput;
use bevy_input::prelude::KeyCode;
use bevy_ecs::prelude::Res;

fn handle_keyboard(keyboard: Res<ButtonInput<KeyCode>>) {
    // Check if key is currently held down
    if keyboard.pressed(KeyCode::KeyW) {
        // Key is being held
    }
    
    // Check for single press (frame-accurate)
    if keyboard.just_pressed(KeyCode::Space) {
        // Key was pressed this frame
    }
    
    // Check for release (frame-accurate)
    if keyboard.just_released(KeyCode::KeyE) {
        // Key was released this frame
    }
    
    // Check multiple keys
    if keyboard.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        // Any of these keys are pressed
    }
    
    // Get all pressed keys
    for key in keyboard.get_pressed() {
        // Iterates over all currently pressed keys
    }
}
```

### Input Events (Event-Driven)

```rust
use bevy_input::keyboard::{KeyboardInput, KeyCode};
use bevy_ecs::event::EventReader;
use bevy_ecs::prelude::Res;

fn handle_keyboard_events(mut events: EventReader<KeyboardInput>) {
    for event in events.read() {
        match event.state {
            ButtonState::Pressed => {
                println!("Key pressed: {:?}", event.key_code);
            }
            ButtonState::Released => {
                println!("Key released: {:?}", event.key_code);
            }
        }
        
        // Event details available:
        // event.key_code - Physical key code
        // event.logical_key - Logical key (layout-aware)
        // event.state - Pressed/Released
        // event.text - Character produced (if any)
        // event.repeat - Whether this is an auto-repeat
        // event.window - Window that received input
    }
}
```

---

## 2. Mouse Input

### Mouse Buttons

```rust
use bevy_input::mouse::MouseButton;

MouseButton::Left    // Primary click
MouseButton::Right   // Context menu
MouseButton::Middle  // Scroll wheel click
MouseButton::Back    // Browser back button
MouseButton::Forward // Browser forward button
MouseButton::Other(u16)  // Additional buttons
```

### Mouse Button Input (Polling)

```rust
use bevy_input::ButtonInput;
use bevy_input::mouse::MouseButton;
use bevy_ecs::prelude::Res;

fn handle_mouse_buttons(mouse: Res<ButtonInput<MouseButton>>) {
    if mouse.pressed(MouseButton::Left) {
        // Left button held
    }
    
    if mouse.just_pressed(MouseButton::Left) {
        // Left button clicked this frame
    }
    
    if mouse.just_released(MouseButton::Left) {
        // Left button released this frame
    }
}
```

### Mouse Motion (Relative Movement)

```rust
use bevy_input::mouse::{MouseMotion, AccumulatedMouseMotion};
use bevy_ecs::event::EventReader;
use bevy_ecs::prelude::Res;
use bevy_math::Vec2;

fn handle_mouse_motion(events: EventReader<MouseMotion>) {
    for event in events.read() {
        // Delta represents change in position since last event
        println!("Mouse moved by: {:?}", event.delta);
    }
}

// Or use the accumulated resource (reset every frame)
fn handle_accumulated_motion(motion: Res<AccumulatedMouseMotion>) {
    println!("Total mouse movement this frame: {:?}", motion.delta);
}
```

### Mouse Scroll (Wheel)

```rust
use bevy_input::mouse::{MouseWheel, MouseScrollUnit, AccumulatedMouseScroll};
use bevy_ecs::event::EventReader;
use bevy_ecs::prelude::Res;
use bevy_math::Vec2;

fn handle_mouse_scroll(events: EventReader<MouseWheel>) {
    for event in events.read() {
        // event.unit - Line or Pixel
        // event.x - Horizontal scroll
        // event.y - Vertical scroll
        // event.window - Window that received input
        
        match event.unit {
            MouseScrollUnit::Line => println!("Scrolled {} lines", event.y),
            MouseScrollUnit::Pixel => println!("Scrolled {} pixels", event.y),
        }
    }
}

// Accumulated scroll (sum of all scroll events this frame)
fn handle_accumulated_scroll(scroll: Res<AccumulatedMouseScroll>) {
    println!("Total scroll: {:?}", scroll.delta);
}
```

### Mouse Position

Note: For absolute cursor position, Bevy uses the window subsystem:

```rust
use bevy_window::PrimaryWindow;
use bevy_ecs::prelude::Query;
use bevy_math::Vec2;

fn get_cursor_position(q_window: Query<&Window, With<PrimaryWindow>>) {
    if let Ok(window) = q_window.get_single() {
        if let Some(position) = window.cursor_position() {
            // position is in window coordinates (pixels)
            println!("Cursor at: {:?}", position);
        }
    }
}
```

---

## 3. Gamepad Input

### Gamepad Buttons

```rust
use bevy_input::gamepad::{GamepadButton, GamepadAxis};

// Action buttons (standard layout)
GamepadButton::South  // A on Xbox, X on PlayStation
GamepadButton::East   // B on Xbox, O on PlayStation  
GamepadButton::North // Y on Xbox, Triangle on PlayStation
GamepadButton::West  // X on Xbox, Square on PlayStation

// Triggers
GamepadButton::LeftTrigger   // L2
GamepadButton::RightTrigger  // R2
GamepadButton::LeftTrigger2  // L2 (additional)
GamepadButton::RightTrigger2 // R2 (additional)

// D-Pad
GamepadButton::DPadUp, GamepadButton::DPadDown
GamepadButton::DPadLeft, GamepadButton::DPadRight

// Thumbsticks (as buttons when pressed)
GamepadButton::LeftThumb, GamepadButton::RightThumb

// Other
GamepadButton::Select, GamepadButton::Start, GamepadButton::Mode
GamepadButton::C, GamepadButton::Z  // Legacy
GamepadButton::Other(u8)  // Non-standard buttons
```

### Gamepad Axes

```rust
use bevy_input::gamepad::GamepadAxis;

GamepadAxis::LeftStickX   // Left stick horizontal
GamepadAxis::LeftStickY   // Left stick vertical
GamepadAxis::RightStickX  // Right stick horizontal
GamepadAxis::RightStickY  // Right stick vertical
GamepadAxis::LeftZ        // Left analog (often throttle)
GamepadAxis::RightZ       // Right analog
GamepadAxis::Other(u8)    // Non-standard axes
```

### Gamepad Component (Per-Gamepad State)

Gamepads are represented as ECS entities with a `Gamepad` component:

```rust
use bevy_input::gamepad::{Gamepad, GamepadButton, GamepadAxis};
use bevy_ecs::prelude::Query;
use bevy_math::Vec2;

fn handle_gamepad(query: Query<&Gamepad>) {
    for gamepad in &query {
        // Button state (digital)
        if gamepad.pressed(GamepadButton::South) {
            // A/X button held
        }
        
        if gamepad.just_pressed(GamepadButton::South) {
            // Just pressed this frame
        }
        
        // Analog values (axes and triggers)
        if let Some(left_x) = gamepad.get(GamepadAxis::LeftStickX) {
            // Returns f32 in range [-1.0, 1.0]
            println!("Left stick X: {}", left_x);
        }
        
        // Convenience methods for sticks
        let left_stick: Vec2 = gamepad.left_stick();
        let right_stick: Vec2 = gamepad.right_stick();
        
        // D-pad as directional vector
        let dpad: Vec2 = gamepad.dpad();
        
        // Analog trigger values (0.0 to 1.0)
        if let Some(lt) = gamepad.get(GamepadButton::LeftTrigger) {
            println!("Left trigger: {}", lt);
        }
    }
}
```

### Gamepad Events

```rust
use bevy_input::gamepad::{
    GamepadConnectionEvent, GamepadEvent,
    GamepadButtonChangedEvent, GamepadAxisChangedEvent,
    GamepadButtonStateChangedEvent,
    GamepadConnection,
};
use bevy_ecs::event::EventReader;

fn handle_gamepad_events(mut events: EventReader<GamepadEvent>) {
    for event in events.read() {
        match event {
            GamepadEvent::Connection(conn_event) => {
                match conn_event.connection {
                    GamepadConnection::Connected { .. } => {
                        println!("Gamepad connected: {:?}", conn_event.gamepad);
                    }
                    GamepadConnection::Disconnected => {
                        println!("Gamepad disconnected: {:?}", conn_event.gamepad);
                    }
                }
            }
            GamepadEvent::Button(btn_event) => {
                // Analog button value (0.0 to 1.0)
                println!("Button {:?} value: {}", btn_event.button, btn_event.value);
            }
            GamepadEvent::Axis(axis_event) => {
                // Axis value (-1.0 to 1.0)
                println!("Axis {:?} value: {}", axis_event.axis, axis_event.value);
            }
        }
    }
}
```

### Gamepad Settings (Deadzones, Thresholds)

```rust
use bevy_input::gamepad::{GamepadSettings, GamepadButton, GamepadAxis, AxisSettings, ButtonSettings};

// Configure per-gamepad in the GamepadSettings component
fn configure_gamepad_settings(settings: &mut GamepadSettings) {
    // Axis deadzone settings
    let axis_settings = AxisSettings::new(
        -1.0,    // livezone_lowerbound
        -0.1,    // deadzone_lowerbound  
        0.1,     // deadzone_upperbound
        1.0,     // livezone_upperbound
        0.01,    // threshold
    ).unwrap();
    
    settings.axis_settings.insert(GamepadAxis::LeftStickX, axis_settings);
    
    // Button press/release thresholds
    let button_settings = ButtonSettings::new(0.8, 0.7).unwrap();
    settings.button_settings.insert(GamepadButton::South, button_settings);
}
```

---

## 4. Input Events and Polling

### Two Input Patterns

Bevy provides two complementary approaches:

#### 1. Event-Based (EventReader)

- Reacts to input as it happens
- Useful for handling ALL input generically
- Preserves in-frame ordering

```rust
use bevy_ecs::event::EventReader;

// For any key press
fn handle_any_keyboard_input(mut events: EventReader<KeyboardInput>) {
    for event in events.read() {
        // Process all keyboard events
    }
}

// For any mouse button
fn handle_any_mouse_button(mut events: EventReader<MouseButtonInput>) {
    for event in events.read() {
        // Process all mouse button events
    }
}
```

#### 2. Resource-Based (Polling)

- Queries current state of input
- Useful for checking specific inputs
- Provides `pressed()`, `just_pressed()`, `just_released()`

```rust
use bevy_ecs::prelude::Res;
use bevy_input::ButtonInput;

// In a system
fn my_system(keyboard: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>) {
    // Polling approach
}
```

### Common Conditions (Run Conditions)

Bevy provides convenience run conditions:

```rust
use bevy_input::common_conditions::input_just_pressed;

fn jump_system() // runs when Space is just pressed
where
    impl bevy_ecs::system::Condition<bool>,
{
    // Only runs when Space key was pressed this frame
}

// Manual check
fn movement_system(keyboard: Res<ButtonInput<KeyCode>>) {
    if input_just_pressed(KeyCode::Space).evaluate(&keyboard) {
        // Jump!
    }
}
```

---

## 5. Core Abstractions

### ButtonInput<T> Resource

The generic button input type:

```rust
use bevy_input::ButtonInput;

// Internally tracks:
// - pressed: HashSet<T> - currently held buttons
// - just_pressed: HashSet<T> - pressed this frame
// - just_released: HashSet<T> - released this frame

// Key methods:
impl<T: Clone + Eq + Hash> ButtonInput<T> {
    fn press(&mut self, input: T);
    fn release(&mut self, input: T);
    fn pressed(&self, input: T) -> bool;
    fn just_pressed(&self, input: T) -> bool;
    fn just_released(&self, input: T) -> bool;
    fn any_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool;
    fn all_pressed(&self, inputs: impl IntoIterator<Item = T>) -> bool;
    fn get_pressed(&self) -> impl Iterator<Item = &T>;
    fn get_just_pressed(&self) -> impl Iterator<Item = &T>;
    fn get_just_released(&self) -> impl Iterator<Item = &T>;
    fn clear(&mut self);           // Clear just_* states
    fn reset_all(&mut self);      // Full reset
}
```

### Axis<T> Resource

For analog inputs:

```rust
use bevy_input::Axis;

// Stores position data as f32
// Range: Axis::MIN (-1.0) to Axis::MAX (1.0)

impl<T: Copy + Eq + Hash> Axis<T> {
    const MIN: f32 = -1.0;
    const MAX: f32 = 1.0;
    
    fn set(&mut self, input: impl Into<T>, value: f32) -> Option<f32>;
    fn get(&self, input: impl Into<T>) -> Option<f32>;  // Clamped
    fn get_unclamped(&self, input: impl Into<T>) -> Option<f32>;
    fn remove(&mut self, input: impl Into<T>) -> Option<f32>;
    fn all_axes(&self) -> impl Iterator<Item = &T>;
}
```

---

## 6. Mage-core Current Input.rs Replacement

### Current Mage-core Implementation

```rust
// Current minimal implementation - only tracks modifier keys
use winit::keyboard::ModifiersState;

pub struct ShiftState {
    shift: bool,
    ctrl: bool,
    alt: bool,
}

impl ShiftState {
    pub fn new() -> Self { ... }
    pub fn shift_down(&self) -> bool { self.shift }
    pub fn ctrl_down(&self) -> bool { self.ctrl }
    pub fn alt_down(&self) -> bool { self.alt }
    // ... modifier combination helpers
    pub fn update(&mut self, modifiers: ModifiersState) { ... }
}
```

### Bevy Replacement Strategy

1. **Remove ShiftState** - Replace with direct use of `ButtonInput<KeyCode>`:
   ```rust
   fn handle_modifiers(keyboard: Res<ButtonInput<KeyCode>>) -> ShiftState {
       ShiftState {
           shift: keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
           ctrl: keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
           alt: keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
       }
   }
   ```

2. **Add Full Input Support**:
   - Keyboard polling (`ButtonInput<KeyCode>`)
   - Mouse buttons and motion
   - Gamepad support via entities

3. **Event Systems**: Add event handlers for:
   - `KeyboardInput` events
   - `MouseButtonInput`, `MouseMotion`, `MouseWheel` events
   - `GamepadEvent` for gamepad connections/state

---

## 7. Key Differences from Mage-core

| Aspect | Mage-core (Current) | Bevy |
|--------|-------------------|------|
| **Keyboard** | Only modifier tracking | Full key polling + events |
| **Mouse** | Not handled | Full support (buttons, motion, scroll) |
| **Gamepad** | Not supported | Full ECS-based gamepad support |
| **Pattern** | Manual state tracking | Event-driven + polling hybrid |
| **Integration** | winit direct | ECS resources and events |

---

## 8. Dependencies for Input

To use Bevy's input system, add to Cargo.toml:

```toml
[dependencies]
bevy = "0.18"  # Includes bevy_input

# Or just the input crate:
# bevy_input = "0.18"
```

Features to enable (enabled by default in full Bevy):
- `keyboard` - Keyboard input
- `mouse` - Mouse input  
- `gamepad` - Gamepad/joystick support
- `touch` - Touch screen support

---

## 9. References

- Bevy Source: `/Users/r/Projects/ascii research/bevy/crates/bevy_input/src/`
- Official Docs: https://docs.rs/bevy/latest/bevy/input/
- Unofficial Bevy Cheat Book: https://bevy-cheatbook.github.io/input.html
- Tainted Coders Guide: https://taintedcoders.com/bevy/input

---

## 10. Summary

Bevy's input system provides a comprehensive, ECS-based solution that far exceeds Mage-core's minimal modifier tracking. The key benefits are:

1. **Unified API**: `ButtonInput<T>` works for keyboard, mouse, and gamepad buttons
2. **Frame-Accurate Polling**: `just_pressed()`, `just_released()` for single-frame detection
3. **Event System**: Process all input events generically when needed
4. **Gamepad Support**: Full ECS entity-based gamepad handling with deadzones
5. **Accumulated Input**: `AccumulatedMouseMotion`, `AccumulatedMouseScroll` for frame totals

**Recommendation**: Replace Mage-core's `input.rs` entirely with Bevy's input system, leveraging both the resource-based polling for game logic and event handlers for input recording/replay features.
