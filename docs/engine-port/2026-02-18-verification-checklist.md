> **STATUS: REFERENCE MATERIAL** — Written 2026-02-18 before the Bevy engine decision (D001, 2026-02-19). The Mage Core rendering analysis and ASCII architecture patterns described here remain valid as technical reference. Integration approach has been updated: Mage Core's 4-texture GPU rendering approach will be implemented within Bevy's render pipeline rather than as a standalone engine. See DECISION_LOG.md D001 for engine decision.

---
title: "Port Verification Gates"
type: research
status: REFERENCE
date: 2026-02-18
# blocked_by: docs/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md (not applicable to Rust port)
---

> **Note:** This document was created during the C++ project's pipeline closeout phase. The blocking dependency on pipeline closeout is not applicable to the Rust port. Content remains valid as reference material.

# Port Verification Gates

## Status: REFERENCE (originally deferred in C++ project)

---

## Gate Hierarchy

```
Pipeline Closeout Gates (HISTORICAL -- not applicable to Rust port)
├── Gate 1: Native Asciicker Conversion
├── Gate 2: Commercial Sprite Sheet
└── macOS Verification

Engine Port Gates
├── P1-1: Rust Core Rendering
├── P1-2: Sample Buffer Parity
├── P1-3: Material System
├── P2-1: XP Loading
├── P2-2: Sprite Atlas Assembly
├── P2-3: Font Rendering
├── P3-1: Character System
├── P3-2: Input Handling
└── P3-3: macOS Build
```

---

## Historical: Pipeline Closeout Gates (not applicable to Rust port)

**Original reference:** `docs/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md` Step 4

### Gate 1: Native Asciicker Conversion

| Check | Command | Expected |
|-------|---------|----------|
| Generate XP | `python3 -m scripts.asset_gen --source-type file --input player-0100.png --name player-0100 --angles 8 --frames 8` | File produced |
| Metadata | `xp_core.get_metadata()` | angles=8, frames=[8], projs=2 |
| Layers | `f.layer_count` | >= 3 |
| Size | `f.width, f.height` | 256 x 160 cells |
| Visual | `--render` output | Recognizable sprite frames |

### Gate 2: Commercial Sprite Sheet

Same checks for 2-3 representative sheets.

### macOS Verification

| Check | Criteria |
|-------|----------|
| Metadata correct | angles/frames/projs match input |
| Viewer loads | `xp_tool.py` opens without error |
| Visual export | Recognizable sprite frames |
| 3+ layers | colorkey, height, visual present |
| Non-trivial glyphs | Layer 2 has non-space chars |
| User sign-off | Manual visual confirmation |

---

## Phase 1 Gates: Core Rendering

### P1-1: Rust Core Rendering

**Objective:** Render single-frame test scene matching C++ output.

**Verification Steps:**

```bash
# 1. Build Rust renderer
cd mage-port
cargo build --release

# 2. Run test scene
cargo run --release --example test_scene

# 3. Compare output
diff output.txt reference_output.txt
```

**Acceptance Criteria:**

| Criterion | Measurement |
|-----------|-------------|
| Compiles | `cargo build` exits 0 |
| Runs | `cargo run` produces output |
| Output matches | `diff` shows no differences |
| Frame time | < 16ms (60 FPS target) |

**Evidence Required:**
- Screenshot of rendered output
- Frame timing data
- Diff output (or identical confirmation)

### P1-2: Sample Buffer Parity

**Objective:** 2x supersampled SampleBuffer matches C++ implementation.

**Verification Steps:**

```bash
# 1. Run single-pixel test
cargo test sample_buffer_pixel

# 2. Run triangle rasterization test
cargo test rasterize_triangle

# 3. Compare against C++ reference
./compare_sample_buffer output.bin reference.bin
```

**Acceptance Criteria:**

| Criterion | Measurement |
|-----------|-------------|
| Sample structure | Same size as C++ (8 bytes) |
| Depth test | Identical behavior at boundary conditions |
| Barycentric weights | Match within 1e-6 tolerance |
| Edge pairing | Same tie-breaking as C++ |

**Evidence Required:**
- Unit test output
- Memory layout comparison
- Boundary condition test results

### P1-3: Material System

**Objective:** Material shade tables produce identical output.

**Verification Steps:**

```bash
# 1. Generate auto_mat lookup table
cargo run --example generate_auto_mat > auto_mat.bin

# 2. Compare with C++ generated table
diff auto_mat.bin reference/auto_mat.bin
```

**Acceptance Criteria:**

| Criterion | Measurement |
|-----------|-------------|
| Table size | 32K entries x 3 bytes |
| RGB quantization | Match C++ rounding |
| Dither glyphs | Correct assignment |

**Evidence Required:**
- Binary comparison output
- Sample RGB→{bg,fg,gl} mappings

---

## Phase 2 Gates: Asset Pipeline

### P2-1: XP Loading

**Objective:** Load `player-0100.xp` and extract all data.

**Verification Steps:**

```bash
# 1. Load XP file
cargo test load_player_0100

# 2. Verify metadata
cargo run --example inspect_xp staging/xp/player-0100.xp
```

**Acceptance Criteria:**

| Criterion | Expected |
|-----------|----------|
| Gzip parse | Header ID1=31, ID2=139, CM=8 |
| Decompression | Correct output size |
| Layer count | >= 3 |
| Cell data | Column-major order |
| Glyph range | All 0-255 |

**Evidence Required:**
- Metadata printout
- Layer extraction results
- Glyph value distribution

### P2-2: Sprite Atlas Assembly

**Objective:** Convert XP layers to Sprite struct with correct atlas layout.

**Verification Steps:**

```bash
# 1. Build sprite atlas
cargo run --example build_atlas staging/xp/player-0100.xp

# 2. Render first frame
cargo run --example render_frame staging/xp/player-0100.xp 0 0
```

**Acceptance Criteria:**

| Criterion | Expected |
|-----------|----------|
| Frame count | angles * projs * sum(anim_lengths) |
| Frame size | Correct subdivision |
| Reference points | Match Layer 0 encoding |
| Height data | Layer 1 glyph interpretation |

**Evidence Required:**
- Frame grid visualization
- Reference point coordinates
- Height map output

### P2-3: Font Rendering

**Objective:** Render text using CP437 font atlas.

**Verification Steps:**

```bash
# 1. Load font
cargo run --example render_text "HELLO WORLD"

# 2. Compare with C++ output
diff text_output.txt reference_text.txt
```

**Acceptance Criteria:**

| Criterion | Expected |
|-----------|----------|
| Glyph mapping | ASCII→atlas index correct |
| Variable width | Advances match font1_xadv |
| Skin recolor | Grey/Gold/Pink palettes |
| Y-inversion | Correct for atlas orientation |

**Evidence Required:**
- Rendered text image
- Advance measurements
- Skin color samples

---

## Phase 3 Gates: Game Logic

### P3-1: Character System

**Objective:** Character entity with equipment state.

**Verification Steps:**

```bash
# 1. Create character
cargo test character_creation

# 2. Test equipment changes
cargo test equipment_state

# 3. Test animation
cargo test character_animation
```

**Acceptance Criteria:**

| Criterion | Expected |
|-----------|----------|
| Entity creation | Position, yaw, equipment |
| Animation state | Correct frame selection |
| Sprite selection | Equipment-dependent atlas lookup |

**Evidence Required:**
- Entity state dump
- Animation frame sequence
- Sprite index for equipment combo

### P3-2: Input Handling

**Objective:** Keyboard/mouse/gamepad input processing.

**Verification Steps:**

```bash
# 1. Test keyboard mapping
cargo test keyboard_input

# 2. Test gamepad mapping
cargo test gamepad_input
```

**Acceptance Criteria:**

| Criterion | Expected |
|-----------|----------|
| UTF-8→CP437 | Correct conversion |
| Gamepad mapping | Configurable bindings |
| Touch support | Gesture recognition |

**Evidence Required:**
- Input event log
- CP437 conversion table verification
- Gamepad mapping file

### P3-3: macOS Build

**Objective:** Build and run on macOS.

**Verification Steps:**

```bash
# 1. Build on macOS
cargo build --release --target aarch64-apple-darwin

# 2. Run test scene
cargo run --release --target aarch64-apple-darwin --example test_scene

# 3. Verify window
# Manual: Check that window opens and renders correctly
```

**Acceptance Criteria:**

| Criterion | Expected |
|-----------|----------|
| Cargo build | Exits 0 |
| Window creation | Visible window |
| Rendering | Correct output |
| Input | Keyboard/mouse responsive |

**Evidence Required:**
- Build log
- Screenshot
- Performance metrics

---

## Gate Status Tracking

| Gate | Status | Date | Evidence |
|------|--------|------|----------|
| Pipeline Gate 1 | N/A (Rust port) | - | Original C++ dependency |
| Pipeline Gate 2 | N/A (Rust port) | - | Original C++ dependency |
| macOS Verification | N/A (Rust port) | - | Original C++ dependency |
| P1-1 Core Rendering | NOT STARTED | - | - |
| P1-2 Sample Buffer | NOT STARTED | - | - |
| P1-3 Material System | NOT STARTED | - | - |
| P2-1 XP Loading | NOT STARTED | - | - |
| P2-2 Sprite Atlas | NOT STARTED | - | - |
| P2-3 Font Rendering | NOT STARTED | - | - |
| P3-1 Character System | NOT STARTED | - | - |
| P3-2 Input Handling | NOT STARTED | - | - |
| P3-3 macOS Build | NOT STARTED | - | - |

---

## Regression Testing

After each gate passes, run regression suite:

```bash
cargo test --all
cargo clippy --all-targets
cargo fmt --check
```

**Acceptance:** All tests pass, no clippy warnings, formatted code.

---

## References

- Closeout Plan: `docs/plans/2026-02-18-feat-pipeline-closeout-minimal-plan.md`
- Mage Core tests: `/Users/r/Projects/ascii research/Mage-core/` (no test directory found)
- Asciicker test files: `/Users/r/Downloads/asciicker-Y9-2/tests/` (if present)
