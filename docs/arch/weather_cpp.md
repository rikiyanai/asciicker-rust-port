# Asciicker Weather System Documentation

## Overview

The Asciicker weather system is a C++ implementation that provides atmospheric weather effects within the ASCII rendering engine. The system is implemented across two source files: weather.h containing type declarations and weather.cpp containing the full implementation. The weather system was developed as Phase 17 of the Asciicker project and integrates with the game loop through the render pipeline.

The weather system currently implements only snow-based weather effects, with support for four discrete weather states ranging from clear conditions to blizzard conditions. Each state controls particle density, snow accumulation rate on terrain, and the visual presentation of falling snow particles. The system uses a custom Perlin noise implementation for wind variation and maintains terrain state backups to enable realistic snow melt behavior when weather conditions change.

## 1. Weather Types

The Asciicker weather system defines four discrete weather states through the WeatherState enumeration in weather.h. Each state represents a progressively more severe snow condition, with associated parameters controlling particle spawn rates and target intensity values. The system does not currently implement rain, fog, or other weather types; it is exclusively a snow weather system.

### Weather State Definitions

The WeatherState enum defines the following states:

| State | Value | Description |
|-------|-------|-------------|
| CLEAR | 0 | No weather effects, clear skies |
| LIGHT_SNOW | 1 | Light snowfall with low particle density |
| HEAVY_SNOW | 2 | Heavy snowfall with moderate particle density |
| BLIZZARD | 3 | Severe blizzard conditions with maximum particle density |

Each state has an associated target intensity value stored in the static state_intensity array. The intensities progress from 0.0 for CLEAR conditions up to 1.0 for BLIZZARD, with intermediate values of 0.3 for LIGHT_SNOW and 0.7 for HEAVY_SNOW. These intensity values drive various aspects of the weather system including particle spawn rates, terrain accumulation probability, and visual rendering parameters.

### Spawn Rate Configuration

The spawn rate configuration is stored in the static spawn_rate array which defines how many particles are spawned per second at each weather state. CLEAR conditions produce zero particles per second, representing no active weather. LIGHT_SNOW generates 10 particles per second, providing a light atmospheric effect. HEAVY_SNOW increases this to 30 particles per second for more substantial snowfall, while BLIZZARD maximum conditions spawn 60 particles per second for intense blizzard effects.

The spawn rate is multiplied by the current weather intensity and the delta time between frames to determine the actual number of particles to spawn each update. This creates a smooth relationship between weather state and particle density rather than abrupt changes when transitioning between states. The fractional remainder is handled through probabilistic spawning, ensuring that even when the spawn count is less than one particle per frame, particles are still generated at the correct average rate over time.

## 2. Weather Particle Systems

The particle system forms the visual core of the weather effects, rendering falling snow as ASCII characters within the game world. The system uses a ring buffer implementation for efficient particle management, avoiding dynamic memory allocation during runtime updates. Each particle represents a single snowflake with position, velocity, lifetime, and visual properties.

### Particle Structure

The Particle structure contains all data required to track and render an individual snow particle. The structure is defined in weather.h and includes a three-component position array pos[3] representing X, Y, and Z coordinates in world space. Velocity is stored in vel[3] with separate components for each axis. The birth field stores a uint64 timestamp indicating when the particle was created, while lifetime stores the particle's duration in microseconds. The glyph field holds the ASCII character code to render, and fg[3] stores the foreground RGB color components.

The particle uses a three-dimensional coordinate system where the Z axis represents vertical position (height). Positive Z values indicate positions above the player's location, and particles fall in the negative Z direction toward the ground. The coordinate system is consistent with the terrain and rendering systems, allowing particles to be properly projected onto the two-dimensional screen buffer.

### Particle Pool Implementation

The ParticlePool structure manages a fixed-capacity ring buffer of 512 particles, defined by the constant ParticlePool::CAPACITY. The pool uses a ring buffer (circular buffer) approach where a head index tracks the insertion point, and count tracks the current number of active particles. When spawning a new particle, the head index advances modulo the capacity, overwriting the oldest particle if the pool is full. This approach provides O(1) insertion and removal without requiring dynamic memory allocation or garbage collection during gameplay.

The ring buffer design means that when the pool reaches capacity, newly spawned particles automatically replace the oldest existing particles in temporal order. This creates a natural effect where older particles fade out as new ones appear, maintaining a consistent visual density regardless of how long the weather system has been active. The fixed capacity also provides predictable memory usage, important for embedded or resource-constrained environments.

### Snow Glyphs and Visual Properties

The visual appearance of snow particles uses four different ASCII glyphs defined in the static snow_glyphs array: the asterisk (0x2A), plus sign (0x2B), period (0x2E), and comma (0x2C). These glyphs are drawn from the CP437 character set, providing classic ASCII art snow representations. Each glyph corresponds to a different fall speed defined in snow_speeds: asterisk falls at 15.0 units per second (fastest), plus sign at 12.0 units per second, period at 9.0 units per second, and comma at 6.0 units per second (slowest).

This variation in fall speeds creates visual depth within the snow field, with faster particles appearing closer to the viewer (larger and more prominent) while slower particles appear more distant. The glyph selection is randomized using the fast_rand() function when particles spawn, providing natural variation in the snow field appearance. The distribution is uniform across all four glyph types, creating a balanced mix of fast and slow falling particles.

### Color Configuration

Snow particles use one of two color schemes, randomly selected at spawn time. The first option is pure white with RGB values (255, 255, 255), representing bright snow crystals catching available light. The second option is a light blue tint with RGB values (200, 220, 255), representing the cooler blue-white appearance of snow in shadow or overcast conditions. The color selection uses a simple bitwise AND operation with a random value, providing a 50/50 distribution between the two color options.

The foreground color is stored in the particle's fg[3] array and is applied directly to the AnsiCell when rendering. This creates the characteristic white and light-blue snow that contrasts with the darker terrain and objects in the game world. The limited color palette maintains the ASCII aesthetic while providing enough visual interest to make the snow effect compelling.

### Particle Spawning Behavior

Particles spawn in a cylindrical volume centered on the player's position, extending from 50 units above the player up to 200 units above the player. The horizontal spawn area spans 100 units in both X and Y directions from the player position, creating a wide field of coverage that ensures particles are always visible on screen regardless of player movement direction. The spawn height range ensures particles enter the view from above and fall through the visible area before despawning.

The spawning logic uses fast_rand() to generate pseudo-random values for position selection, glyph selection, and color selection. This provides sufficient randomness for visual variety while avoiding the overhead of more sophisticated random number generation. The spawn position calculation distributes particles uniformly within the spawn cylinder, with each coordinate component receiving an independent random offset from the player's position.

### Particle Lifetime and Motion

Each particle has a randomized lifetime between 5 and 8 seconds (5,000,000 to 8,000,000 microseconds), determined at spawn time by adding a random duration to the birth timestamp. This variation in lifetime contributes to visual variety and ensures particles do not all despawn simultaneously, which would create jarring visual artifacts. The lifetime is stored as a uint64 value in microseconds, matching the timestamp format used throughout the game engine.

Particle motion is purely vertical with zero horizontal velocity at spawn time, causing particles to fall straight down toward the ground. The velocity magnitude depends on the assigned glyph, with faster glyphs corresponding to higher downward velocity. The vertical fall continues until either the particle's lifetime expires or the particle is recycled by the ring buffer when new particles are spawned. There is no collision detection with terrain; particles simply fall through the world and despawn after their lifetime expires.

### Wind Simulation with Perlin Noise

The weather system uses the siv::PerlinNoise library to generate wind effects that influence particle horizontal motion. The Perlin noise is sampled at time intervals in the UpdateWeather function, with separate samples for X and Y wind components. The noise time coordinate advances based on the delta time multiplied by 0.3, creating slowly evolving wind patterns rather than rapid changes.

The wind values are scaled by the current weather intensity, so wind effects are absent during CLEAR weather conditions and reach maximum strength during BLIZZARD conditions. The wind calculation multiplies the Perlin noise output by 2.0 and then by the intensity, producing wind vectors in the range of approximately [-2.0, 2.0] scaled by intensity. These wind values are applied to particle positions in the update loop, creating gentle drifting motion that makes the snow fall appear more natural and dynamic.

## 3. Weather Effects on Rendering

The weather system's rendering component overlays snow particles onto the ASCII cell buffer after the main scene rendering completes. This compositing approach ensures weather effects appear on top of terrain, objects, and other scene elements. The rendering uses screen-space projection to convert 3D particle positions into 2D buffer coordinates, with appropriate bounds checking to handle particles that fall outside the visible area.

### CompositeSnowParticles Function

The CompositeSnowParticles function handles the rendering of all active weather particles to the AnsiCell buffer. This function is called during the render phase after the base scene has been drawn to the buffer. It receives pointers to the weather system, the target buffer, buffer dimensions, the renderer, and the current timestamp. The function performs early exit if the weather intensity is below 0.01, avoiding unnecessary processing during clear weather or very light snow conditions.

The function iterates through all particles in the pool, checking each for validity before rendering. A particle is considered valid if its birth timestamp is in the past (it has already spawned) and the current timestamp is within its lifetime (it has not expired yet). Invalid particles are skipped, allowing the ring buffer to naturally recycle them without explicit removal. This lazy deletion approach is efficient and avoids the overhead of compacting the particle array.

### Coordinate Projection

Each valid particle's 3D world position is converted to 2D screen coordinates using the ProjectCoords function from the render system. This projection applies the camera transformation and perspective projection to determine where the particle appears on screen. The ProjectCoords function returns a boolean indicating whether the projection was successful (particle is within the view frustum) and writes the screen coordinates to an output array if successful.

The projection function handles all the complexity of converting world coordinates to screen coordinates, including camera position, orientation, field of view, and aspect ratio considerations. If a particle projects successfully, the resulting screen X and Y coordinates are used as indices into the AnsiCell buffer. The function also performs bounds checking against the buffer dimensions, ensuring that only particles that actually appear on screen are rendered.

### Cell Rendering

The rendering operation modifies the AnsiCell at the projected screen position by setting the glyph and foreground color while preserving the background color. This approach overlays the snow particle on top of whatever scene element was previously rendered at that position, creating the effect of snow falling in front of terrain and objects. The foreground color is set to 255, which triggers special handling in the terminal rendering pipeline to use the particle's actual RGB color values stored in fg[3].

The choice to preserve background color while replacing glyph and foreground creates a layered compositing effect where snow appears translucent against the background. This is appropriate for distant snow particles that should partially reveal the terrain beneath. The rendering does not perform any depth sorting relative to scene geometry; it simply draws on top of whatever was previously in the buffer, which is consistent with the 2.5D rendering approach used throughout Asciicker.

### Color Mapping

The weather system uses a helper function RgbToXterm256 to convert RGB color values to xterm-256 color indices when needed for terminal rendering. This function maps RGB values (0-255 per component) to the 216-color xterm-256 cube (indices 16-231) by quantizing each component to one of six levels (0, 51, 102, 153, 204, 255). The formula computes the index as 16 plus the red component index multiplied by 36, plus the green component index multiplied by 6, plus the blue component index.

However, the current implementation of CompositeSnowParticles sets the foreground color to 255 (the special "true color" indicator) rather than using the xterm-256 mapping. This triggers true color output (24-bit RGB) in supported terminals, providing more accurate snow colors than the limited 256-color palette would allow. The RgbToXterm256 function remains available in the codebase for other rendering scenarios that require palette-based colors.

### Terrain Snow Accumulation

Beyond particle rendering, the weather system also modifies the visual appearance of terrain by accumulating snow on ground surfaces. This effect is implemented through the UpdateSnowAccumulation function, which queries the terrain system for visible patches near the player and probabilistically transitions terrain materials from their current state to snow-covered variants. The accumulation is controlled by weather intensity and elevation, creating the effect of snow accumulating on higher ground first.

The accumulation system uses a callback-based approach where SnowAccumCB processes each terrain cell individually. For each cell, the function checks whether the cell's height is above the current snow line threshold. If conditions are favorable for accumulation (weather is active and elevation exceeds the snow line), the system evaluates a probabilistic transition based on the material's transition rate and current weather intensity. Successful transitions change the terrain material to snow (material ID 5), creating the visual effect of snow-covered ground.

### Material Transition Rates

Different terrain materials have different affinities for snow accumulation, defined by the mat_transition_rate array indexed by material ID. This array allows the weather system to differentiate between materials that readily accept snow coverage versus those that resist it. The initialization function InitMatTransitionRate configures the transition rates for known materials in the game world.

Grass (material 1), dirt (material 2), and sand (material 3) all have a transition rate of 1.0, meaning they accept snow at the full rate determined by weather intensity. Stone (material 4) has a reduced rate of 0.3, representing the idea that stone surfaces do not accumulate snow as readily as softer ground materials. Mud (material 6) has a rate of 0.8, while cobble (material 7) and gravel (material 8) both have rates of 0.5. Water (material 0) and already-snow-covered surfaces (material 5) have zero transition rates and never accumulate additional snow.

## 4. Weather State Management

The weather state management system controls transitions between weather states, tracks the current weather conditions, and coordinates all subsystems including particle updates, terrain modification, and rendering. The main Weather structure holds all state information, and public API functions provide controlled access to modify weather conditions from game logic, the editor, or external systems like the MCP (Machine Control Protocol).

### Weather Structure Definition

The Weather structure in weather.h contains all persistent state for the weather system. The state field holds the current WeatherState enum value indicating which weather preset is active. The intensity field is a floating-point value representing the current weather intensity (0.0 to 1.0), while target_intensity stores the intensity value that the system is transitioning toward. The transition_speed field controls how quickly intensity lerps toward the target, defaulting to 0.1.

The wind[2] array stores the current wind vector with separate X and Y components, computed each update from Perlin noise. The snow_line field stores the current elevation threshold above which terrain accumulates snow, computed from intensity. The accum_rate field controls the base accumulation probability for terrain materials. The stamp field stores the timestamp of the last update, used for delta time calculations. The pn_time field tracks time for Perlin noise sampling.

Cached player position fields (_player_x, _player_y, _player_z) store the player's current world position, updated each frame by UpdateWeather and used for particle spawning and terrain querying. The pool field contains the ParticlePool managing all active particles. The pn field is the Perlin noise generator instance. The backup system fields (backup_count, backup_alloc, backups) manage the storage of original terrain materials for melt restoration.

### State Transition Mechanism

Weather state transitions are handled through linear interpolation (lerp) of the intensity value toward the target intensity for the requested state. When SetWeather is called with a new state value, the function validates the state parameter (clamping to the valid range 0-3), updates the state field, and sets the target intensity to the corresponding value from state_intensity. The actual intensity gradually approaches this target in subsequent calls to UpdateWeather.

The lerp implementation in UpdateWeather computes the difference between target and current intensity, then advances the current intensity by a step proportional to the transition speed and delta time. If the step would overshoot the target, the intensity is clamped directly to the target value. This creates smooth, gradual transitions rather than abrupt changes when switching weather states. The transition speed of 0.1 provides a moderate pace for weather changes; complete transitions typically take several seconds.

### Intensity-Driven Systems

The weather intensity value drives multiple subsystems throughout the weather system. Particle spawn rates are multiplied by intensity, so at intensity 0.5, only half the nominal spawn rate produces particles. The wind magnitude is scaled by intensity, so calm conditions produce minimal drift while intense weather produces strong wind effects. The snow line elevation threshold is computed as 10000 minus intensity times 9000, meaning clear conditions have a snow line at 10000 units (effectively off) while blizzard conditions have a snow line at 1000 units.

The terrain accumulation probability is also scaled by intensity, so light snow accumulates slowly while heavy snow accumulates quickly. The CompositeSnowParticles function checks intensity against a threshold of 0.01 and skips rendering entirely when intensity is negligible. This cascading use of intensity as a primary driver ensures that all weather effects are consistently proportional to the overall weather severity.

### Snow Line Management

The snow line represents the elevation threshold determining where snow accumulates on terrain. Higher snow lines mean only very high-elevation terrain receives snow, while lower snow lines allow snow to accumulate at lower elevations. The snow line is computed each update as 10000 minus intensity times 9000, yielding a range from 10000 (clear) down to 1000 (blizzard). This creates the effect of snow line descending as weather intensifies.

The snow line transitions gradually using the same delta-time-based approach as intensity. The difference between target and current snow line is computed, and the current value advances by half the difference times delta time. This creates a smooth descent of the snow line during snow events and a smooth ascent during clear weather, rather than abrupt threshold changes that would create visible waves of accumulation sweeping across the terrain.

### Terrain Backup System

The backup system stores original terrain material IDs to enable restoration when snow melts. When a terrain cell transitions to snow-covered, the system first creates a backup entry for the containing terrain patch if one does not already exist. The backup captures the original material ID for all 64 cells (8x8 visual grid) in the patch, allowing complete restoration of the pre-snow terrain appearance.

The PatchBackup structure stores a pointer to the Patch and an array of 64 bytes holding the original material IDs. The backup array grows dynamically as needed, starting with an allocation of 64 entries and doubling whenever the current allocation is exhausted. This provides flexible storage that expands to handle any amount of snow-covered terrain while avoiding excessive memory allocation for small snow areas.

### Melt Behavior

When weather intensity drops below 0.01 (CLEAR conditions), the system begins melting snow-covered terrain by restoring original materials from backups. The SnowAccumCB callback checks whether conditions favor melting (intensity is below threshold and cell elevation is below snow line) and whether the cell is currently snow-covered. If both conditions are true, the system looks up the backup for that patch and retrieves the original material ID.

The melt probability is fixed at 0.02 (2% per update cycle), creating a gradual melt effect rather than instant restoration. This slow melt rate ensures that snow does not disappear unrealistically quickly when weather clears. The system also checks that the original material was not snow; if the pre-snow terrain was already snow-covered (perhaps from a previous weather event), that cell remains snow-covered to avoid unnecessary transitions.

### Update Throttling

The terrain accumulation and melt system runs on a throttled schedule to avoid excessive terrain modifications. The UpdateSnowAccumulation function checks the timestamp against last_accum_stamp and returns early if less than 200000 microseconds (200 milliseconds) have elapsed since the last update. This throttling limits terrain modifications to a maximum of 5 times per second, reducing CPU load while still providing responsive accumulation and melt behavior.

The throttle applies to both accumulation and melt operations; when the throttle triggers, all eligible cells are processed in a single batch. This batch processing approach is more efficient than per-frame processing and creates a slightly stepped progression of terrain changes rather than continuous gradual modification.

### Public API Functions

The weather system exposes a small public API for integration with the Asciicker engine. CreateWeather() allocates and initializes a new Weather instance, returning a pointer that is also stored in the global weather variable. DeleteWeather() frees all allocated memory including the backup array. SetWeather(int state) transitions to a new weather state by index. GetWeather() returns the current state as an integer.

The update function UpdateWeather(uint64_t stamp, float player_x, float player_y, float player_z) must be called each frame with the current timestamp in microseconds and the player's world position. This function updates all weather systems including intensity transitions, wind computation, particle spawning, and particle motion. The compositing function CompositeSnowParticles renders particles to the buffer and should be called during the render phase after the main scene is drawn but before the buffer is displayed.

## 5. Integration Points

The weather system integrates with several other components of the Asciicker engine through defined interfaces. The render system provides the ProjectCoords function for 3D-to-2D projection and uses the AnsiCell buffer for final output. The terrain system provides query functions to iterate over visible patches and access to the visual map for material modification. The game loop calls update and compositing functions at appropriate points in the frame pipeline.

### Game Loop Integration

The weather system is typically initialized once at game startup through a call to CreateWeather(). During each render frame, UpdateWeather is called with the current timestamp and player position, updating all simulation state. After the main terrain and object rendering completes, CompositeSnowParticles overlays the weather particles. The terrain accumulation update may be called less frequently, throttled to 5 Hz, rather than every frame.

The game loop also provides user interface controls for weather selection, typically implemented through ImGui combo boxes in the editor view panel. The SetWeather and GetWeather functions provide the interface for these controls, allowing runtime weather changes during gameplay or editing.

### Terrain System Interface

The weather system interacts with the terrain system through several functions. GetTerrainVisualMap retrieves a pointer to the visual material map for a terrain patch, allowing read access to current materials and write access to modify them. GetTerrainHeightMap retrieves height values for elevation calculations. UpdateTerrainVisualMap signals that a patch's visual map has been modified and needs to be refreshed in rendering caches.

The QueryTerrain function provides a callback-based interface for iterating over terrain patches near a given position. The weather system uses this to find patches within 40 units of the player for accumulation processing. The callback receives the patch pointer and coordinates, allowing the weather system to process each visible patch individually.

## 6. Technical Details

The weather system uses several technical approaches to achieve its functionality while maintaining efficiency and compatibility with the ASCII rendering paradigm. Timestamps are stored as uint64_t microseconds, matching the convention used throughout the game engine. The Perlin noise implementation uses the siv::PerlinNoise library with default seeding. Random number generation uses the fast_rand() function for performance, providing pseudo-random values suitable for visual effects.

### Memory Management

All weather system memory is allocated at creation time or during backup growth events. The main Weather structure and ParticlePool are allocated as a single contiguous block via calloc, providing zero-initialized state. Backup storage grows dynamically via realloc when needed, doubling the allocation size to reduce allocation frequency. No memory is allocated during normal frame updates, eliminating allocation-related frame rate variability.

### Performance Characteristics

The particle update loop iterates through all particles in the pool but performs only simple arithmetic operations per particle. The O(n) complexity where n equals the pool capacity (512) is constant and very fast. Terrain accumulation processing is throttled and only runs at most 5 times per second, limiting its impact on frame rate. The ring buffer design ensures predictable cache behavior and eliminates iteration overhead for particle management.

### Rendering Performance

Weather particle rendering only processes active (valid-lifetime) particles, typically less than the full pool capacity. The projection and bounds checking filter out off-screen particles before buffer access. The compositing operation is a simple struct assignment per rendered particle. Overall rendering overhead is minimal compared to the main scene rendering, making weather effects lightweight even on slower hardware.

## 7. Summary

The Asciicker weather system provides a complete snow weather implementation with particle-based visual effects, terrain accumulation and melt simulation, and state-driven intensity control. The system uses a ring buffer particle pool for efficient management of up to 512 simultaneous snow particles, with varied glyphs and colors creating visual depth. Terrain modification uses a backup system to enable reversible snow accumulation that restores original materials when weather clears.

The four-state weather model (CLEAR, LIGHT_SNOW, HEAVY_SNOW, BLIZZARD) provides sufficient variation for gameplay while maintaining simplicity. Smooth intensity transitions through lerp create gradual changes rather than jarring state switches. The integration with the Perlin noise library provides natural wind variation that makes the snow effect feel dynamic and alive. The throttled terrain updates balance visual responsiveness with CPU efficiency.

For a Rust port, the key architectural decisions include the ring buffer particle pool design, the backup system for terrain material restoration, the Perlin noise wind simulation, and the state-driven intensity model. The system could be extended to support additional weather types (rain, fog) by adding new particle behaviors and rendering modes while preserving the existing state management framework.
