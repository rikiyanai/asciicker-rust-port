# Implementation Plan: MEDIUM Severity Game Logic Gaps

## Executive Summary

This document provides implementation plans for five MEDIUM severity gaps identified in the game logic system. Each plan includes Bevy ECS component and system design, implementation details, and phase recommendations based on dependencies and complexity.

**Gap Reference:** `/Users/r/Projects/asciicker rust port/docs/gaps-game-logic.md`

**Source Files:** `game.cpp`, `game.h`

**Target Framework:** Bevy ECS (Rust)

---

## Table of Contents

1. [Fly Mode Implementation](#1-fly-mode-implementation)
2. [Camera Controls (scene_shift, cam_shift, zoom)](#2-camera-controls)
3. [AI Behaviors (Follower System, Buddy AI)](#3-ai-behaviors)
4. [Multiplayer Lag Measurement](#4-multiplayer-lag-measurement)
5. [UI Features (Virtual Keyboard, Minimap)](#5-ui-features)

---

## 1. Fly Mode Implementation

### 1.1 Overview

Fly mode allows free camera movement without physics constraints. In the original C++ implementation, this is controlled by a boolean flag that is enabled by default in pure terminal builds (`PURE_TERM`) but disabled in graphical builds.

**Source Reference:** `gaps-game-logic.md` Section 1.1

```cpp
// From game.cpp line 4161-4165
#ifdef PURE_TERM
    g->fly_mode = true;
#else
    g->fly_mode = false;
#endif
```

### 1.2 Bevy ECS Implementation

#### Phase Recommendation: PHASE 1

Fly mode is a core camera behavior that affects player movement and should be implemented early as it impacts fundamental gameplay mechanics.

#### Components

```rust
/// Resource for managing global camera mode settings
#[derive(Resource)]
pub struct CameraMode {
    /// When true, camera moves freely without physics constraints
    pub fly_mode: bool,
    /// Toggle fly mode via input
    pub fly_mode_toggle_key: KeyCode,
}

impl Default for CameraMode {
    fn default() -> Self {
        Self {
            fly_mode: false,  // Default to disabled (matching graphical build)
            fly_mode_toggle_key: KeyCode::KeyF,
        }
    }
}

/// Tag component for entities in fly mode
#[derive(Component)]
pub struct FlyModeCamera;

/// Component for fly speed configuration
#[derive(Component)]
pub struct FlySpeed {
    pub horizontal: f32,  // Units per second
    pub vertical: f32,
}

impl Default for FlySpeed {
    fn default() -> Self {
        Self {
            horizontal: 10.0,
            vertical: 5.0,
        }
    }
}
```

#### Systems

```rust
/// System to toggle fly mode on key press
fn fly_mode_toggle_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    camera_mode: Res<CameraMode>,
    mut camera_query: Query<Entity, With<Camera>>,
) {
    if keys.just_pressed(camera_mode.fly_mode_toggle_key) {
        let mut new_mode = camera_mode.clone();
        new_mode.fly_mode = !new_mode.fly_mode;
        commands.insert_resource(new_mode);
        
        info!("Fly mode toggled: {}", new_mode.fly_mode);
    }
}

/// System to apply fly movement (no physics constraints)
fn fly_movement_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    camera_mode: Res<CameraMode>,
    mut query: Query<(&mut Transform, &FlySpeed), With<FlyModeCamera>>,
) {
    // Only apply fly movement when fly mode is enabled
    if !camera_mode.fly_mode {
        return;
    }

    for (mut transform, speed) in &mut query {
        let mut direction = Vec3::ZERO;
        
        // WASD for horizontal movement
        if keys.pressed(KeyCode::KeyW) { direction.z -= 1.0; }
        if keys.pressed(KeyCode::KeyS) { direction.z += 1.0; }
        if keys.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
        if keys.pressed(KeyCode::KeyD) { direction.x += 1.0; }
        
        // Space/Shift for vertical movement
        if keys.pressed(KeyCode::Space) { direction.y += 1.0; }
        if keys.pressed(KeyCode::ShiftLeft) { direction.y -= 1.0; }
        
        if direction != Vec3::ZERO {
            let delta = time.delta_seconds();
            transform.translation += direction.normalize() * speed.horizontal * delta;
            transform.translation.y += direction.y * speed.vertical * delta;
        }
    }
}

/// System to switch between physics-based movement and fly mode
fn movement_mode_switch_system(
    mut commands: Commands,
    camera_mode: Res<CameraMode>,
    mut query: Query<Entity, (With<Camera>, With<PhysicsBody>)>,
) {
    for entity in &mut query {
        if camera_mode.fly_mode {
            // Remove physics component when in fly mode
            commands.entity(entity).remove::<PhysicsBody>();
            commands.entity(entity).insert(FlyModeCamera);
        } else {
            // Re-add physics component when leaving fly mode
            // (requires storing original physics config)
            commands.entity(entity).remove::<FlyModeCamera>();
            // commands.entity(entity).insert(PhysicsBody { ... });
        }
    }
}
```

### 1.3 Integration Points

| Component | Integration Point | Notes |
|-----------|-------------------|-------|
| `CameraMode` | Resource | Insert in `GameState::Playing` setup |
| `FlySpeed` | Component on Camera | Configurable per-instance |
| Input handling | `input.cpp` bridge | Map toggle key from input system |
| Physics system | Disable when fly_mode active | Requires physics integration |

### 1.4 Dependencies

- Basic camera setup (already in roadmap)
- Input system (already in roadmap)
- Transform system (built into Bevy)

---

## 2. Camera Controls (scene_shift, cam_shift, zoom)

### 2.1 Overview

The camera control system manages three distinct camera adjustments:

- **scene_shift**: Horizontal offset for inventory display (slides scene when inventory opens)
- **cam_shift**: Vertical camera pan
- **zoom**: Zoom level (1.0 default)

**Source Reference:** `gaps-game-logic.md` Section 1.4

```cpp
// From game.h
int scene_shift;     // horizontal scene offset for inventory
int cam_shift;       // vertical camera pan
float zoom;          // zoom level (1.0 default)
```

### 2.2 Bevy ECS Implementation

#### Phase Recommendation: PHASE 1

Camera controls are fundamental to the game view and should be implemented alongside fly mode. These features have clear user-facing functionality.

#### Components

```rust
/// Camera configuration resource
#[derive(Resource)]
pub struct CameraConfig {
    /// Horizontal scene offset for UI elements (inventory slide)
    pub scene_shift: f32,
    /// Target scene_shift for smooth animation
    pub scene_shift_target: f32,
    /// Vertical camera pan offset
    pub cam_shift: f32,
    /// Target cam_shift for smooth animation
    pub cam_shift_target: f32,
    /// Zoom level (1.0 = default, 2.0 = 2x zoom)
    pub zoom: f32,
    /// Animation lerp factor (0.0-1.0)
    pub smoothing: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            scene_shift: 0.0,
            scene_shift_target: 0.0,
            cam_shift: 0.0,
            cam_shift_target: 0.0,
            zoom: 1.0,
            smoothing: 0.15,  // Matching C++ implementation
        }
    }
}

/// Tag for main game camera
#[derive(Component)]
pub struct MainCamera;

/// Orthographic camera settings for zoom
#[derive(Component)]
pub struct OrthographicZoom {
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub zoom_step: f32,
}

impl Default for OrthographicZoom {
    fn default() -> Self {
        Self {
            min_zoom: 0.5,
            max_zoom: 3.0,
            zoom_step: 0.25,
        }
    }
}

/// Event for triggering scene shift (inventory open/close)
#[derive(Event)]
pub struct SceneShiftEvent {
    pub shift_amount: f32,
}

/// Event for zoom changes
#[derive(Event)]
pub struct ZoomEvent {
    pub delta: f32,
}
```

#### Systems

```rust
/// System to handle scene shift (inventory slide animation)
fn scene_shift_system(
    time: Res<Time>,
    mut camera_config: ResMut<CameraConfig>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    // Smooth interpolation toward target
    camera_config.scene_shift = lerp(
        camera_config.scene_shift,
        camera_config.scene_shift_target,
        camera_config.smoothing * time.delta_seconds() * 60.0,
    );
    
    // Apply to camera transform
    for mut transform in &mut query {
        transform.translation.x = camera_config.scene_shift;
    }
}

/// System to handle vertical camera pan
fn cam_shift_system(
    time: Res<Time>,
    mut camera_config: ResMut<CameraConfig>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    // Smooth interpolation toward target
    camera_config.cam_shift = lerp(
        camera_config.cam_shift,
        camera_config.cam_shift_target,
        camera_config.smoothing * time.delta_seconds() * 60.0,
    );
    
    // Apply to camera transform
    for mut transform in &mut query {
        transform.translation.y = camera_config.cam_shift;
    }
}

/// System to handle zoom changes
fn zoom_system(
    mut camera_config: ResMut<CameraConfig>,
    mut query: Query<(&mut OrthographicProjection, &OrthographicZoom), With<MainCamera>>,
) {
    for (mut projection, zoom_settings) in &mut query {
        // Clamp zoom to valid range
        camera_config.zoom = camera_config.zoom.clamp(
            zoom_settings.min_zoom,
            zoom_settings.max_zoom,
        );
        
        // Update orthographic scale
        projection.scale = camera_config.zoom;
    }
}

/// System to handle zoom input
fn zoom_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut zoom_events: EventWriter<ZoomEvent>,
    zoom_settings: Query<&OrthographicZoom>,
) {
    let step = zoom_settings.single().zoom_step;
    
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        zoom_events.send(ZoomEvent { delta: step });
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        zoom_events.send(ZoomEvent { delta: -step });
    }
}

/// System to process zoom events
fn zoom_event_system(
    mut events: EventReader<ZoomEvent>,
    mut camera_config: ResMut<CameraConfig>,
) {
    for event in events.read() {
        camera_config.zoom += event.delta;
    }
}

/// System to handle inventory open/close scene shift
fn inventory_scene_shift_system(
    mut scene_shift_events: EventWriter<SceneShiftEvent>,
    inventory_state: Res<InventoryState>,
    camera_config: Res<CameraConfig>,
) {
    let inventory_width = 200.0; // From C++ implementation
    
    let target = if inventory_state.is_open {
        inventory_width
    } else {
        0.0
    };
    
    // Only send event if target changed
    if (camera_config.scene_shift_target - target).abs() > 0.01 {
        scene_shift_events.send(SceneShiftEvent { shift_amount: target });
    }
}

/// System to process scene shift events
fn scene_shift_event_system(
    mut events: EventReader<SceneShiftEvent>,
    mut camera_config: ResMut<CameraConfig>,
) {
    for event in events.read() {
        camera_config.scene_shift_target = event.shift_amount;
    }
}

// Helper function matching C++ Lerp
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}
```

### 2.3 Integration Points

| Component | Integration Point | Notes |
|-----------|-------------------|-------|
| `CameraConfig` | Resource | Insert in camera setup |
| `MainCamera` | Component | Tag main camera entity |
| Inventory system | Event-driven | Trigger scene_shift on open/close |
| UI system | Zoom controls | Map to +/- or mouse wheel |

### 2.4 Input Mapping

| Input | Action | Notes |
|-------|--------|-------|
| `=` / `+` | Zoom in | Increment by zoom_step |
| `-` | Zoom out | Decrement by zoom_step |
| Mouse wheel | Zoom in/out | Optional alternative |
| Inventory toggle | Scene shift | Slide scene horizontally |

### 2.5 Dependencies

- Fly mode (shares CameraConfig resource)
- Inventory system (triggers scene_shift)
- Input system (zoom keys)

---

## 3. AI Behaviors (Follower System, Buddy AI)

### 3.1 Overview

This section covers two related AI features:

1. **Follower System**: Tracks how many followers a character has, affecting enemy AI target selection
2. **Buddy AI**: Friendly NPCs that assist the player

**Source Reference:** `gaps-game-logic.md` Sections 2.2 and 2.3

```cpp
// Follower tracking
int followers;  // number of followers

// Target selection with follower weighting
if (!enemy_ch || d * (h2->followers + 4) < enemy_cd * (enemy_cf + 4))

// Buddy spawning
NPC_Human* buddy = (NPC_Human*)malloc(sizeof(NPC_Human));
buddy->enemy = false;  // Friendly character
```

### 3.2 Bevy ECS Implementation

#### Phase Recommendation: DEFER to Phase 2

AI behaviors depend on the core character/AI system being implemented first. These are more complex and should be deferred until the base AI system is functional.

#### Components

```rust
/// Component for tracking follower count
#[derive(Component, Default)]
pub struct FollowerCount {
    pub count: i32,
    pub leader: Option<Entity>,
}

/// Component to mark entities as followers
#[derive(Component)]
pub struct Follower {
    pub leader: Entity,
    pub formation_position: i32,  // Position in formation (0, 1, 2, ...)
    pub follow_distance: f32,
    pub interpolation_speed: f32,
}

/// Component to mark buddy/friendly AI characters
#[derive(Component)]
pub struct BuddyAI {
    pub is_active: bool,
    pub help_range: f32,       // Distance to offer help
    pub follow_player: bool,   // Whether to follow player
    pub default_equipment: Vec<EquipmentSlot>,
}

/// Tag for buddy state (buddy is idle, following, fighting, etc.)
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum BuddyState {
    Idle,
    Following,
    Fighting,
    Helping,
}

/// Component for buddy behavior configuration
#[derive(Component)]
pub struct BuddyConfig {
    pub spawn_position_offset: Vec3,
    pub idle_ai_interval: f32,     // Seconds between AI decisions
    pub help_threshold: f32,        // Player HP threshold to offer help
}

impl Default for BuddyConfig {
    fn default() -> Self {
        Self {
            spawn_position_offset: Vec3::new(2.0, 0.0, 2.0),
            idle_ai_interval: 1.0,
            help_threshold: 0.3,  // 30% HP
        }
    }
}

/// Component for tracking shoot-by relationship
#[derive(Component)]
pub struct ShootByTracker {
    pub shooter: Option<Entity>,
    pub shot_timestamp: u64,
    pub priority_window: u64,   // 5000000 microseconds = 5 seconds
    pub cooldown_window: u64,   // 500000 microseconds = 0.5 seconds
}

impl Default for ShootByTracker {
    fn default() -> Self {
        Self {
            shooter: None,
            shot_timestamp: 0,
            priority_window: 5_000_000,
            cooldown_window: 500_000,
        }
    }
}

/// Event when a character is shot
#[derive(Event)]
pub struct CharacterShotEvent {
    pub shooter: Entity,
    pub target: Entity,
    pub timestamp: u64,
}
```

#### Systems

```rust
/// System to update follower positions
fn follower_movement_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Follower)>,
    leader_query: Query<&Transform, Without<Follower>>,
) {
    for (mut transform, follower) in &mut query {
        if let Ok(leader_transform) = leader_query.get(follower.leader) {
            // Calculate target position based on formation
            let offset = Vec3::new(
                (follower.formation_position as f32 % 3.0) * 2.0,
                0.0,
                (follower.formation_position as f32 / 3.0).floor() * 2.0,
            );
            
            let target_pos = leader_transform.translation + offset;
            
            // Smooth interpolation toward target
            transform.translation = transform.translation.lerp(
                target_pos,
                follower.interpolation_speed * time.delta_seconds(),
            );
        }
    }
}

/// System to handle buddy AI decision making
fn buddy_ai_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut BuddyAI, &mut BuddyState, &BuddyConfig)>,
    player_query: Query<&Transform, With<Player>>,
    enemy_query: Query<(Entity, &Transform, &Health), With<Enemy>>,
) {
    for (mut transform, mut buddy, mut state, config) in &mut query {
        // Get player position for distance calculations
        let player_pos = player_query.single().translation;
        let buddy_pos = transform.translation;
        let dist_to_player = buddy_pos.distance(player_pos);
        
        // Find nearest enemy
        let mut nearest_enemy: Option<(Entity, f32)> = None;
        for (enemy, enemy_transform, health) in &enemy_query {
            let dist = buddy_pos.distance(enemy_transform.translation);
            if health.current > 0.0 && (nearest_enemy.is_none() || dist < nearest_enemy.unwrap().1) {
                nearest_enemy = Some((enemy, dist));
            }
        }
        
        // AI state machine
        match *state {
            BuddyState::Idle => {
                if buddy.follow_player && dist_to_player > config.spawn_position_offset.length() {
                    *state = BuddyState::Following;
                } else if let Some((_, dist)) = nearest_enemy {
                    if dist < buddy.help_range {
                        *state = BuddyState::Fighting;
                    }
                }
            }
            BuddyState::Following => {
                if dist_to_player < config.spawn_position_offset.length() {
                    *state = BuddyState::Idle;
                } else if let Some((_, dist)) = nearest_enemy {
                    if dist < buddy.help_range {
                        *state = BuddyState::Fighting;
                    }
                }
            }
            BuddyState::Fighting => {
                if let Some((_, dist)) = nearest_enemy {
                    if dist > buddy.help_range * 1.5 {
                        *state = BuddyState::Following;
                    }
                } else {
                    *state = BuddyState::Idle;
                }
            }
            BuddyState::Helping => {
                // Move toward player to help
                let direction = (player_pos - buddy_pos).normalize();
                transform.translation += direction * 5.0 * time.delta_seconds();
                
                if dist_to_player < 3.0 {
                    *state = BuddyState::Idle;
                }
            }
        }
    }
}

/// System to handle shoot_by priority in target selection
fn shoot_by_priority_system(
    mut events: EventReader<CharacterShotEvent>,
    mut query: Query<(&mut ShootByTracker, &Transform)>,
) {
    for event in events.read() {
        if let Ok((mut tracker, _)) = query.get_mut(event.target) {
            // Only update if outside cooldown
            if event.timestamp > tracker.shot_timestamp + tracker.cooldown_window {
                tracker.shooter = Some(event.shooter);
                tracker.shot_timestamp = event.timestamp;
            }
        }
    }
}

/// System to apply shoot_by weight in AI targeting
fn apply_shoot_by_weight_system(
    current_time: Res<CurrentGameTime>,
    mut ai_target_query: Query<(&mut TargetPriority, &ShootByTracker)>,
) {
    for (mut priority, tracker) in &mut ai_target_query {
        if let Some(shooter) = tracker.shooter {
            let time_since_shot = current_time.timestamp.saturating_sub(tracker.shot_timestamp);
            
            // Within priority window: multiply distance by 0.2 (higher priority)
            if time_since_shot < tracker.priority_window 
               && time_since_shot > tracker.cooldown_window {
                priority.distance_multiplier *= 0.2;
                priority.is_aggressive = true;
            } else if time_since_shot >= tracker.priority_window {
                // Outside window: clear shooter reference
                priority.is_aggressive = false;
            }
        }
    }
}

/// System to spawn buddy NPCs at game start
fn buddy_spawn_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    config: Res<BuddyConfig>,
) {
    let player_pos = player_query.single().translation;
    
    // Spawn 2 buddies (matching C++ implementation)
    for i in 0..2 {
        let offset = Vec3::new(
            (i as f32) * 3.0,
            0.0,
            2.0,
        );
        
        commands.spawn((
            Transform::from_translation(player_pos + offset),
            BuddyAI {
                is_active: true,
                help_range: 10.0,
                follow_player: true,
                default_equipment: vec![],
            },
            BuddyState::Idle,
            config.clone(),
            FollowerCount { count: 0, leader: None },
        ));
    }
}

/// Component for AI target priority calculations
#[derive(Component, Default)]
pub struct TargetPriority {
    pub distance_multiplier: f32,
    pub is_aggressive: bool,
}

/// Resource for current game time
#[derive(Resource, Default)]
pub struct CurrentGameTime {
    pub timestamp: u64,
}
```

### 3.3 Target Selection Algorithm (C++ to Rust Translation)

The C++ target selection logic:

```cpp
// Original C++
if (!enemy_ch || d * (h2->followers + 4) < enemy_cd * (enemy_cf + 4))

// Translation to Rust with shoot_by
fn calculate_target_priority(
    distance: f32,
    follower_count: i32,
    shooter_priority: f32,
) -> f32 {
    // Base priority = distance * (followers + 4)
    // Lower value = higher priority
    let base_priority = distance * (follower_count as f32 + 4.0);
    
    // Apply shoot_by weight if being shot
    if shooter_priority > 0.0 {
        base_priority * shooter_priority
    } else {
        base_priority
    }
}
```

### 3.4 Integration Points

| Component | Integration Point | Notes |
|-----------|-------------------|-------|
| `FollowerCount` | Character component | Track on player/AI entities |
| `BuddyAI` | Spawn in `GameState::Playing` | 2 buddies spawned at start |
| `ShootByTracker` | Combat system | On weapon fire event |
| Target selection | AI system | Integrate with existing AI |

### 3.5 Dependencies

- Character/Entity system (base AI)
- Combat system (shoot_by events)
- Health system (buddy help logic)

---

## 4. Multiplayer Lag Measurement

### 4.1 Overview

The lag measurement system tracks network latency between client and server, enabling the game to adapt to network conditions.

**Source Reference:** `gaps-game-logic.md` Section 4.1

```cpp
// From game.h
uint64_t last_lag;
int lag_ms;
bool lag_wait;

// Lag response handling
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

### 4.2 Bevy ECS Implementation

#### Phase Recommendation: DEFER to Phase 2

Multiplayer features are complex and should be deferred until single-player gameplay is stable. Lag measurement specifically depends on the network system being in place.

#### Components

```rust
/// Resource for network lag measurement
#[derive(Resource)]
pub struct LagMeasurement {
    /// Timestamp of last lag measurement request
    pub last_lag_timestamp: u64,
    /// Measured lag in milliseconds
    pub lag_ms: i32,
    /// Whether client is currently waiting on server response
    pub is_waiting: bool,
    /// Lag measurement interval in milliseconds
    pub ping_interval_ms: u64,
    /// Maximum acceptable lag before showing warning (ms)
    pub warning_threshold_ms: i32,
    /// Maximum acceptable lag before pausing (ms)
    pub pause_threshold_ms: i32,
    /// Rolling average of recent lag measurements
    pub lag_history: Vec<i32>,
    pub history_max_size: usize,
}

impl Default for LagMeasurement {
    fn default() -> Self {
        Self {
            last_lag_timestamp: 0,
            lag_ms: 0,
            is_waiting: false,
            ping_interval_ms: 1000,  // 1 second
            warning_threshold_ms: 200,
            pause_threshold_ms: 1000,
            lag_history: Vec::new(),
            history_max_size: 60,  // 60 samples = 1 minute
        }
    }
}

/// Network connection state
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub enum NetworkState {
    Disconnected,
    Connecting,
    Connected,
    Lagging,
}

/// Event for sending lag measurement request
#[derive(Event)]
pub struct LagPingEvent {
    pub client_timestamp: u64,
}

/// Event for receiving lag measurement response
#[derive(Event)]
pub struct LagPongEvent {
    pub client_timestamp: u64,
    pub server_timestamp: u64,
    pub receive_timestamp: u64,
}

/// Event for lag warning UI update
#[derive(Event)]
pub struct LagWarningEvent {
    pub lag_ms: i32,
    pub severity: LagSeverity,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LagSeverity {
    Normal,
    Warning,
    Critical,
}
```

#### Systems

```rust
/// System to send periodic lag ping requests
fn lag_ping_system(
    time: Res<Time>,
    mut lag_measurement: ResMut<LagMeasurement>,
    mut ping_events: EventWriter<LagPingEvent>,
    network_state: Res<NetworkState>,
) {
    // Only ping when connected and not already waiting
    if *network_state != NetworkState::Connected || lag_measurement.is_waiting {
        return;
    }
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    
    if current_time - lag_measurement.last_lag_timestamp 
       >= lag_measurement.ping_interval_ms * 1000 {
        
        lag_measurement.last_lag_timestamp = current_time;
        lag_measurement.is_waiting = true;
        
        ping_events.send(LagPingEvent { 
            client_timestamp: current_time,
        });
    }
}

/// System to process lag pong responses
fn lag_pong_system(
    mut events: EventReader<LagPongEvent>,
    mut lag_measurement: ResMut<LagMeasurement>,
    mut warning_events: EventWriter<LagWarningEvent>,
    mut network_state: ResMut<NetworkState>,
) {
    for event in events.read() {
        lag_measurement.is_waiting = false;
        
        let receive_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        
        // Calculate round-trip time
        let rtt = receive_time.saturating_sub(event.client_timestamp);
        let lag_ms = (rtt / 2) as i32;  // One-way latency
        
        lag_measurement.lag_ms = lag_ms;
        
        // Update rolling history
        if lag_measurement.lag_history.len() 
           >= lag_measurement.history_max_size {
            lag_measurement.lag_history.remove(0);
        }
        lag_measurement.lag_history.push(lag_ms);
        
        // Determine severity
        let severity = if lag_ms >= lag_measurement.pause_threshold_ms {
            LagSeverity::Critical
        } else if lag_ms >= lag_measurement.warning_threshold_ms {
            LagSeverity::Warning
        } else {
            LagSeverity::Normal
        };
        
        // Update network state
        if severity == LagSeverity::Critical {
            *network_state = NetworkState::Lagging;
        } else {
            *network_state = NetworkState::Connected;
        }
        
        // Send warning event for UI
        warning_events.send(LagWarningEvent { 
            lag_ms,
            severity,
        });
    }
}

/// System to calculate average lag
fn lag_average_system(
    lag_measurement: Res<LagMeasurement>,
    mut average_lag: ResMut<AverageLag>,
) {
    if lag_measurement.lag_history.is_empty() {
        average_lag.value = 0;
        return;
    }
    
    let sum: i32 = lag_measurement.lag_history.iter().sum();
    average_lag.value = sum / lag_measurement.lag_history.len() as i32;
}

/// Resource for displaying average lag
#[derive(Resource)]
pub struct AverageLag {
    pub value: i32,
}

impl Default for AverageLag {
    fn default() -> Self {
        Self { value: 0 }
    }
}

/// System to handle lag timeout (no response received)
fn lag_timeout_system(
    time: Res<Time>,
    mut lag_measurement: ResMut<LagMeasurement>,
    mut warning_events: EventWriter<LagWarningEvent>,
    mut network_state: ResMut<NetworkState>,
) {
    if !lag_measurement.is_waiting {
        return;
    }
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    
    let timeout_ms = lag_measurement.pause_threshold_ms;
    
    if current_time - lag_measurement.last_lag_timestamp 
       >= (timeout_ms as u64 * 1000) {
        // Timeout - treat as critical lag
        lag_measurement.is_waiting = false;
        lag_measurement.lag_ms = timeout_ms;
        
        *network_state = NetworkState::Lagging;
        
        warning_events.send(LagWarningEvent {
            lag_ms: timeout_ms,
            severity: LagSeverity::Critical,
        });
    }
}

/// System to apply lag compensation to player actions
fn lag_compensation_system(
    average_lag: Res<AverageLag>,
    mut player_action_query: Query<(&mut PlayerAction, &mut Transform)>,
) {
    // Lag compensation: predict player position based on average latency
    // This is a simplified version - full implementation would use
    // client-side prediction and server reconciliation
    
    let compensation_ms = average_lag.value;
    
    for (mut action, mut transform) in &mut player_action_query {
        match *action {
            PlayerAction::Shoot { ref mut predicted_pos, .. } => {
                // Adjust aim position based on lag
                // This would require velocity data to properly predict
                *predicted_pos = Some(transform.translation);
            }
            _ => {}
        }
    }
}

/// Player action with lag compensation data
#[derive(Component)]
pub enum PlayerAction {
    Idle,
    Move { direction: Vec3 },
    Shoot { target: Entity, predicted_pos: Option<Vec3> },
    UseItem { item: Entity },
}
```

### 4.3 Integration Points

| Component | Integration Point | Notes |
|-----------|-------------------|-------|
| `LagMeasurement` | Resource | Insert on network connect |
| `LagPingEvent` | Network client | Send to server |
| `LagPongEvent` | Network server | Process response |
| `LagWarningEvent` | UI system | Display warning |
| `NetworkState` | Game state | Affect gameplay |

### 4.4 Dependencies

- Network client/server system
- UI system (lag warnings)
- Game state management

---

## 5. UI Features (Virtual Keyboard, Minimap)

### 5.1 Overview

This section covers two UI features:

1. **Virtual Keyboard**: Touch/mouse input for character input (show_keyb, keyb_hide, keyb_key)
2. **Minimap**: 32x16 display showing terrain, NPCs, player position and direction

**Source Reference:** `gaps-game-logic.md` Sections 5.2 and 5.4

```cpp
// Virtual keyboard
bool show_keyb;     // activated together with talk_box by clicking on character
int keyb_hide;     // show / hide animator (vertical position)
uint8_t keyb_key[32];  // simulated key presses by touch/mouse

// Minimap
// Draws 32x16 minimap in top-right
// Shows terrain, NPCs, player position and direction
// Only rendered when !show_inventory && !main_menu
```

### 5.2 Bevy ECS Implementation

#### Phase Recommendation: DEFER to Phase 2

UI features are important but can be implemented after core gameplay. Virtual keyboard specifically may need platform-specific handling. Minimap rendering depends on world system.

#### Components - Virtual Keyboard

```rust
/// Resource for virtual keyboard state
#[derive(Resource)]
pub struct VirtualKeyboard {
    /// Whether virtual keyboard is visible
    pub is_visible: bool,
    /// Vertical position for show/hide animation
    pub position_y: f32,
    /// Target position for animation
    pub target_y: f32,
    /// Animation smoothing factor
    pub smoothing: f32,
    /// Currently pressed keys (simulated)
    pub pressed_keys: Vec<KeyCode>,
    /// Maximum keys that can be tracked
    pub max_keys: usize,
}

impl Default for VirtualKeyboard {
    fn default() -> Self {
        Self {
            is_visible: false,
            position_y: -200.0,  // Hidden position
            target_y: 0.0,
            smoothing: 0.15,
            pressed_keys: Vec::new(),
            max_keys: 32,
        }
    }
}

/// Event to show/hide virtual keyboard
#[derive(Event)]
pub struct VirtualKeyboardToggleEvent {
    pub show: bool,
}

/// Event when virtual key is pressed
#[derive(Event)]
pub struct VirtualKeyPressEvent {
    pub key: KeyCode,
}
```

#### Components - Minimap

```rust
/// Resource for minimap configuration
#[derive(Resource)]
pub struct MinimapConfig {
    /// Whether minimap is visible
    pub is_visible: bool,
    /// Width in cells
    pub width: u32,
    /// Height in cells
    pub height: u32,
    /// Position offset from top-right
    pub offset_x: f32,
    pub offset_y: f32,
    /// Scale factor for entity icons
    pub entity_scale: f32,
    /// Colors for different entity types
    pub player_color: Color,
    pub npc_color: Color,
    pub enemy_color: Color,
    pub terrain_color: Color,
    /// Render distance (cells from player)
    pub render_distance: f32,
}

impl Default for MinimapConfig {
    fn default() -> Self {
        Self {
            is_visible: true,
            width: 32,
            height: 16,
            offset_x: 10.0,
            offset_y: 10.0,
            entity_scale: 1.0,
            player_color: Color::GREEN,
            npc_color: Color::CYAN,
            enemy_color: Color::RED,
            terrain_color: Color::GRAY,
            render_distance: 16.0,
        }
    }
}

/// Component to tag minimap UI entity
#[derive(Component)]
pub struct MinimapUI;

/// Tag for entities that should appear on minimap
#[derive(Component)]
pub struct MinimapIcon {
    pub icon_type: MinimapIconType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MinimapIconType {
    Player,
    Ally,
    Enemy,
    Item,
    Objective,
}
```

#### Systems - Virtual Keyboard

```rust
/// System to toggle virtual keyboard visibility
fn virtual_keyboard_toggle_system(
    mut events: EventReader<VirtualKeyboardToggleEvent>,
    mut keyboard: ResMut<VirtualKeyboard>,
) {
    for event in events.read() {
        keyboard.is_visible = event.show;
        keyboard.target_y = if event.show { 0.0 } else { -200.0 };
    }
}

/// System to animate virtual keyboard show/hide
fn virtual_keyboard_animation_system(
    time: Res<Time>,
    mut keyboard: ResMut<VirtualKeyboard>,
) {
    keyboard.position_y = lerp(
        keyboard.position_y,
        keyboard.target_y,
        keyboard.smoothing * time.delta_seconds() * 60.0,
    );
}

/// System to handle virtual key presses (touch/mouse input)
fn virtual_key_input_system(
    mut events: EventReader<VirtualKeyPressEvent>,
    mut keyboard: ResMut<VirtualKeyboard>,
    mut key_input: ResMut<ButtonInput<KeyCode>>,
) {
    for event in events.read() {
        // Add to tracked keys
        if keyboard.pressed_keys.len() < keyboard.max_keys 
           && !keyboard.pressed_keys.contains(&event.key) {
            keyboard.pressed_keys.push(event.key);
        }
        
        // Press the key in Bevy's input system
        key_input.press(event.key);
    }
}

/// System to release virtual keys on touch/mouse release
fn virtual_key_release_system(
    mut events: EventReader<VirtualKeyReleaseEvent>,
    mut keyboard: ResMut<VirtualKeyboard>,
    mut key_input: ResMut<ButtonInput<KeyCode>>,
) {
    for event in events.read() {
        // Remove from tracked keys
        keyboard.pressed_keys.retain(|k| *k != event.key);
        
        // Release the key in Bevy's input system
        key_input.release(event.key);
    }
}

#[derive(Event)]
pub struct VirtualKeyReleaseEvent {
    pub key: KeyCode,
}

/// System to render virtual keyboard UI
fn render_virtual_keyboard_system(
    mut commands: Commands,
    keyboard: Res<VirtualKeyboard>,
    mut ui_query: Query<Entity, With<VirtualKeyboardUI>>,
    asset_server: Res<AssetServer>,
) {
    // This would spawn UI entities for the virtual keyboard
    // Implementation depends on UI framework choice (bevy_ui, bevy_egui, etc.)
    
    if keyboard.is_visible {
        // Spawn keyboard UI nodes at keyboard.position_y
    } else {
        // Despawn or hide keyboard UI
    }
}
```

#### Systems - Minimap

```rust
/// System to update minimap visibility based on game state
fn minimap_visibility_system(
    inventory_state: Res<InventoryState>,
    game_state: Res<State<GameState>>,
    mut minimap_config: ResMut<MinimapConfig>,
) {
    // Hide minimap when inventory is open or in main menu
    let should_show = !inventory_state.is_open 
        && *game_state == GameState::Playing;
    
    minimap_config.is_visible = should_show;
}

/// System to render minimap
fn minimap_render_system(
    mut commands: Commands,
    minimap_config: Res<MinimapConfig>,
    player_query: Query<&Transform, With<Player>>,
    terrain_query: Query<&TerrainData>,
    mut characters_query: Query<(Entity, &Transform, &CharacterType), Without<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !minimap_config.is_visible {
        return;
    }
    
    let player_pos = player_query.single().translation;
    
    // Calculate minimap bounds centered on player
    let half_width = minimap_config.width as f32 / 2.0;
    let half_height = minimap_config.height as f32 / 2.0;
    
    // For each cell in minimap
    for y in 0..minimap_config.height {
        for x in 0..minimap_config.width {
            let world_x = player_pos.x + (x as f32 - half_width);
            let world_z = player_pos.z + (y as f32 - half_height);
            
            // Query terrain at this position
            // Query characters at this position
            
            // Set cell color based on what's at this position
            // Render to minimap texture
        }
    }
    
    // Render player direction indicator
    // Render entity icons
}

/// System to spawn minimap UI element
fn minimap_ui_spawn_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    minimap_config: Res<MinimapConfig>,
) {
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(minimap_config.offset_x),
                top: Val::Px(minimap_config.offset_y),
                width: Val::Px(minimap_config.width as f32 * 8.0),
                height: Val::Px(minimap_config.height as f32 * 8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.7)),
            ..default()
        },
        MinimapUI,
    ));
}

/// Inventory state resource
#[derive(Resource)]
pub struct InventoryState {
    pub is_open: bool,
}

impl Default for InventoryState {
    fn default() -> Self {
        Self { is_open: false }
    }
}

/// Character type for minimap icon determination
#[derive(Component)]
pub struct CharacterType {
    pub is_enemy: bool,
    pub is_ally: bool,
    pub is_player: bool,
}
```

### 5.3 Integration Points

| Component | Integration Point | Notes |
|-----------|-------------------|-------|
| `VirtualKeyboard` | Resource | Toggle on talk interaction |
| `VirtualKeyPressEvent` | Touch/Mouse input | Platform-specific handling |
| `MinimapConfig` | Resource | Insert in UI setup |
| `MinimapIcon` | Character components | Add to all relevant entities |
| `InventoryState` | Resource | Trigger minimap visibility |

### 5.4 Dependencies

- UI framework (bevy_ui or bevy_egui)
- Input system (touch/mouse)
- World system (terrain queries)
- Character system (entity positions)

---

## 6. Phase Recommendations Summary

| Feature | Phase | Rationale |
|---------|-------|-----------|
| Fly Mode | Phase 1 | Core camera behavior, impacts movement mechanics |
| Camera Controls | Phase 1 | Fundamental to game view, integrates with inventory |
| AI Behaviors | Phase 2 | Depends on base AI system, complex logic |
| Multiplayer Lag | Phase 2 | Depends on network system, defer for single-player |
| Virtual Keyboard | Phase 2 | UI feature, platform-specific handling |
| Minimap | Phase 2 | UI feature, depends on world system |

---

## 7. Implementation Dependencies

### Phase 1 Prerequisites

- Basic Bevy ECS setup
- Transform system
- Input system
- Basic camera setup

### Phase 2 Prerequisites

- Full character/AI system
- Combat system
- Network system
- UI framework
- World/terrain system

---

## 8. Open Questions

### Fly Mode

1. Should fly mode toggle persist across save/load?
2. Should there be a visible indicator when fly mode is active?

### Camera Controls

1. Should zoom center on player or mouse position?
2. What is the maximum scene_shift value for inventory?

### AI Behaviors

1. How many followers can a character have?
2. Should buddy AI have equipment upgrading?

### Multiplayer Lag

1. What is the server-side lag measurement implementation?
2. Should lag compensation apply to all player actions?

### UI Features

1. What virtual keyboard layout should be used?
2. Should minimap support toggling via hotkey?

---

## 9. Summary

This implementation plan provides Bevy ECS designs for five MEDIUM severity game logic gaps:

1. **Fly Mode**: Implemented as a camera mode toggle with smooth movement
2. **Camera Controls**: Three distinct adjustments (scene_shift, cam_shift, zoom) with smooth animation
3. **AI Behaviors**: Follower tracking, buddy AI state machine, and shoot-by priority system
4. **Multiplayer Lag**: Ping-pong measurement with rolling average and warning system
5. **UI Features**: Virtual keyboard with touch input and minimap with entity icons

**Phase 1**: Fly Mode, Camera Controls (immediate implementation)

**Phase 2**: AI Behaviors, Multiplayer Lag, UI Features (after prerequisites)

---

## References

- Source gaps analysis: `/Users/r/Projects/asciicker rust port/docs/gaps-game-logic.md`
- ECS architecture: `/Users/r/Projects/asciicker rust port/docs/research-ecs-architecture.md`
- C++ source: `game.cpp`, `game.h`
- Integration decisions: `/Users/r/Projects/asciicker rust port/docs/plan-integration-decisions.md`

---

*Document Version: 1.0*
*Created: 2026-02-20*
*Scope: MEDIUM severity game logic gaps implementation plans*
