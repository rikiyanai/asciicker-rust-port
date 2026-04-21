# Control Reference

This is the master control list for the current Rust port.

It mixes normal gameplay controls, debug/tuning controls, and capture controls.
Some controls only work in specific modes.

## Normal Gameplay

These work while a player character exists and the game is in `Playing`.

### Movement

- `W` / `A` / `S` / `D`
- `Arrow Up` / `Arrow Left` / `Arrow Down` / `Arrow Right`

What it does:
- Moves the character relative to the current camera yaw.

What it changes visually:
- Moves the world view as the player moves.
- Changes what terrain, meshes, reflections, and water are visible.

### Slow Move

- `Left Shift`
- `Right Shift`

What it does:
- Halves movement force.

What it changes visually:
- Makes movement slower and easier to position precisely for visual checks and captures.

### Turn

- `Q`
- `E`

What it does:
- Applies yaw torque to the player/camera-facing direction.

What it changes visually:
- Rotates the world around your viewpoint.
- Very useful for checking edge contrast, mesh readability, and water/reflection behavior from different angles.

### Jump

- `Space`

What it does:
- Triggers a one-shot jump impulse.

What it changes visually:
- Raises the camera/player vertically.
- This changes perspective strongly, especially on terrain edges and water boundaries.

### Block

- `F`

What it does:
- Starts blocking if the equipped shield allows it.
- Releasing `F` stops blocking.

What it changes visually:
- Mostly changes character state/animation logic.
- Not a primary rendering-debug control.

## Menu / Game State

### Main Menu Navigation

- `Arrow Up`
- `Arrow Down`

What it does:
- Moves the main menu selection.

What it changes visually:
- Changes which menu item is highlighted.

### Main Menu Confirm

- `Enter`

What it does:
- Activates the selected menu item.

What it changes visually:
- Starts the game or enters the selected menu action.

### Pause / Return

- `Escape`

What it does:
- `Playing` -> `Paused`
- `Paused` -> `Playing`
- Other states -> returns to `MainMenu`

What it changes visually:
- Pauses or resumes gameplay rendering.
- Can return you out of stuck states.

## Spectator Mode

These only matter when no `Character` entity exists.

### Spectator Move

- `W` / `A` / `S` / `D`

What it does:
- Moves the camera directly in the world.

What it changes visually:
- Slides the whole view without player physics.

### Spectator Turn

- `Q`
- `E`

What it does:
- Rotates camera yaw by 45 degrees per key press.

What it changes visually:
- Snaps the camera to fixed viewing angles.

## Weather Debug

### Cycle Weather

- `F5`

What it does:
- Cycles weather through:
  - `Clear`
  - `LightSnow`
  - `HeavySnow`
  - `Blizzard`

What it changes visually:
- Changes snow particle density and wind-driven weather intensity.
- Useful for checking whether weather overlays hurt readability.

## Shape-Vector / Glyph Tuning

These are the main live visual-tuning controls.

### Compare Render Modes

- `F12`

What it does:
- Cycles between:
  - `original_only`
  - `combined`
  - `harri_priority`

What it changes visually:
- `original_only`
  - disables shape-vector entirely
  - you see only the original resolve/material/overlay path
- `combined`
  - keeps original semantic edge/overlay glyphs authoritative
  - shape-vector is allowed only on non-semantic cells
- `harri_priority`
  - lets the Alex Harri path win broadly
  - much easier to compare whether the extra edge chaos is coming from shape-vector overrides or the original resolve path

Use this when:
- you want to flip the same camera view between the three render policies without relaunching
- you want to judge whether a bad edge is coming from Harri override behavior or from the base resolve/render pipeline

### Cycle Alphabet

- `F6`

What it does:
- Cycles the active shape-vector alphabet.

What it changes visually:
- Changes the pool of glyphs the renderer is allowed to pick from.
- This can make the scene look denser, sparser, cleaner, or harsher depending on the alphabet.

### Match Threshold

- `[`
- `]`

What it does:
- Adjusts `distance_threshold`.

What it changes visually:
- Lower threshold:
  - more conservative matching
  - more blank/color-only cells
  - fewer risky glyph guesses
- Higher threshold:
  - more aggressive matching
  - more glyphs and stronger edges
  - higher chance of noisy or wrong glyphs

### Structural Fallback Distance

- `9`
- `0`

What it does:
- Adjusts the extra fallback threshold used when the main matcher rejects a glyph but the renderer still tries to keep structure.

What it changes visually:
- Lower value:
  - fewer fallback structural glyphs
  - more blank-but-colored cells
- Higher value:
  - more structure preserved
  - stronger outlines and terrain texture
  - can also add noise if pushed too far

### Adaptive Threshold Boost

- `7`
- `8`

What it does:
- Lowers or raises the extra threshold bonus given to high-contrast cells.

What it changes visually:
- Lower value:
  - fewer adaptive shape matches
  - more conservative output
- Higher value:
  - fewer threshold rejects on strong edges
  - potentially more glyph overrides and more visual noise

### Global Crunch

- `;`
- `'`

What it does:
- Adjusts global contrast emphasis in shape matching.

What it changes visually:
- Lower value:
  - flatter overall tone response
  - softer edges
- Higher value:
  - stronger contrast separation
  - more pronounced bright/dark structure

### Directional Crunch

- `,`
- `.`

What it does:
- Adjusts directional edge emphasis in shape matching.

What it changes visually:
- Lower value:
  - weaker edge detection
  - softer silhouettes
- Higher value:
  - stronger directional edge contrast
  - sharper-looking contours and slopes

### Sampling Quality

- `-`
- `=`

What it does:
- Decreases or increases the internal shape-vector sampling quality.

What it changes visually:
- Lower value:
  - faster
  - rougher glyph choices
- Higher value:
  - slower
  - more stable / more accurate glyph comparisons

### Toggle Global Crunch

- `F7`

What it does:
- Turns global crunch on or off.

What it changes visually:
- Lets you see whether the frame needs global contrast shaping or looks better without it.

### Toggle Directional Crunch

- `F8`

What it does:
- Turns directional crunch on or off.

What it changes visually:
- Lets you check whether contour clarity is coming from directional edge shaping.

### Toggle Structural Fallback

- `F10`

What it does:
- Turns structural fallback on or off.

What it changes visually:
- Off:
  - more cells may collapse to space/background color
- On:
  - more structure is preserved when the primary matcher declines a glyph

### Toggle Adaptive Threshold

- `F11`

What it does:
- Turns the contrast-adaptive threshold on or off.

What it changes visually:
- On:
  - reduces threshold rejects on some strong-contrast cells
  - may increase glyph overrides and noise
- Off:
  - keeps the stricter default threshold behavior

Important:
- This is currently an experimental tuning control.
- It is available live, but it is not the default mode.

### Reset Shape-Vector Tuning

- `\\`

What it does:
- Restores the default shape-vector settings.

What it changes visually:
- Returns the frame to the current default tuning baseline.

## Capture / Regression Harness

These are for deterministic comparison captures.

### Start Orbit Capture From Current Position

- `F9`

What it does:
- Arms the baseline orbit capture from your current pose.
- The harness freezes the capture anchor and steps yaw automatically frame by frame.

What it changes visually:
- Starts a controlled 360-degree capture sequence for regression comparison.
- You should not need to keep turning manually after this starts.

Important:
- `F9` only works if the game was launched with the baseline/capture harness enabled.
- `F12` still changes mode during normal harness use, unless a variant replay is forcing the mode sequence.

Typical launch:

```bash
cd /Users/rikihernandez/Downloads/asciicker-rust-port/engine-port
ASCIICKER_BASELINE_DIR=/tmp/asciicker-orbit-live \
ASCIICKER_BASELINE_RECORD=1 \
ASCIICKER_BASELINE_YAW_STEP=3 \
ASCIICKER_BASELINE_MAX_FRAMES=120 \
cargo run --release
```

Then:
1. Fly or move to the point you want.
2. Stop where you want the capture anchored.
3. Press `F9`.

### Auto-Locked Orbit Capture

No key press required if you launch with an explicit lock position.

Example:

```bash
cd /Users/rikihernandez/Downloads/asciicker-rust-port/engine-port
ASCIICKER_BASELINE_DIR=/tmp/asciicker-orbit-live \
ASCIICKER_BASELINE_RECORD=1 \
ASCIICKER_BASELINE_LOCK_POS="0,0,5.4375" \
ASCIICKER_BASELINE_YAW_START=45 \
ASCIICKER_BASELINE_YAW_STEP=3 \
ASCIICKER_BASELINE_MAX_FRAMES=120 \
ASCIICKER_BASELINE_EXIT=1 \
cargo run --release
```

What it does:
- Starts the capture automatically from the fixed coordinates and yaw you provide.

What it changes visually:
- Produces a repeatable comparison run without manual positioning.

### Sequenced Variant Replay

This is the reusable stitched comparison mode.

Example:

```bash
cd /Users/rikihernandez/Downloads/asciicker-rust-port/engine-port
ASCIICKER_BASELINE_DIR=/Users/rikihernandez/Downloads/asciicker-rust-port/artifacts/baselines/orbit-2026-03-11-variant-three-mode \
ASCIICKER_BASELINE_REPLAY=/Users/rikihernandez/Downloads/asciicker-rust-port/artifacts/baselines/orbit-2026-03-11-current/trace.jsonl \
ASCIICKER_BASELINE_VARIANT_MODES=original_only,combined,harri_priority \
ASCIICKER_BASELINE_VARIANT_FRAMES=120 \
ASCIICKER_BASELINE_EXIT=1 \
cargo run --release
```

What it does:
- Replays the same trace multiple times in one run.
- Forces the configured render mode sequence segment by segment.
- Draws a two-line bottom panel showing the current mode and the most relevant live settings/stats.

What it changes visually:
- Produces one stitched capture where you can watch `original_only`, then `combined`, then `harri_priority` back to back under identical camera motion.

Notes:
- This is meant for visual comparison captures, not normal gameplay.
- The bottom panel is only drawn during variant replay mode.

## Title Bar / Debug Readout

The window title now includes the most important live debug info:

- version / iteration
- FPS
- camera position
- yaw
- zoom
- shape-vector settings
- live shape-vector frame stats

What it is for:
- Lets you confirm exactly which build you are looking at.
- Lets you correlate visual changes with tuning changes.
- Helps avoid comparing the wrong build or wrong config.

## Recommended Capture Workflow

If your goal is a comparison run:

1. Launch with the baseline harness enabled.
2. Move to the viewpoint you want.
3. Let motion settle.
4. Press `F9`.
5. Wait for the orbit capture to finish.

If you want the cleanest visual comparison:

1. Avoid pressing tuning keys during the capture.
2. Keep the same alphabet and shape-vector settings between runs.
3. Use the same map area and same weather state.
