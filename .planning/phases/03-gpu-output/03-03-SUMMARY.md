---
phase: 03-gpu-output
plan: 03
subsystem: gpu-output
tags: [window-resize, msaa, visual-verification, bevy-window, hidpi]

# Dependency graph
requires:
  - phase: 03-gpu-output
    plan: 02
    provides: AsciiGpuPlugin render pipeline, ViewNode, extract/prepare systems
provides:
  - Window resize handler using physical pixel dimensions (HiDPI-aware)
  - Msaa::Off on Camera2d (matches pipeline sample_count=1)
  - Human-verified visual output (checkerboard test pattern, resize behavior)
---

## Summary

Window resize handling and visual verification of the complete Phase 3 GPU output pipeline.

## What was built

1. **Window resize system** (`handle_window_resize`): Listens for `WindowResized` messages, reads physical pixel dimensions from the Window component (HiDPI-aware), computes new grid dimensions as `physical_size / font_size`, and reallocates AsciiCellGrid arrays.

2. **MSAA fix**: Added `Msaa::Off` component to Camera2d entity. Bevy 0.18 defaults to `Msaa::Sample4` but our render pipeline uses `sample_count=1`. The mismatch caused a wgpu validation panic at runtime (F005 in FAILURE_LOG).

3. **Visual verification**: Human confirmed checkerboard test pattern renders correctly with distinct CP437 glyphs (full block + medium shade), correct fg/bg colors (orange/green on dark blue/dark red), smooth resize behavior, and stable framerate.

## Key files

### Created
None (this plan modifies existing files only)

### Modified
- `engine-port/src/output/mod.rs` — resize handler, Msaa::Off on Camera2d, HiDPI physical pixel dimensions

## Commits
- `0dfe33d` feat(03-03): add window resize handler with grid dimension recalculation
- `dd748b0` fix(03-03): disable MSAA on ASCII camera to match pipeline sample count

## Deviations

1. **[F005] MSAA sample count mismatch** — Runtime wgpu validation panic. Bevy 0.18 moved MSAA from global Resource to per-camera Component. Fixed by adding `Msaa::Off` to Camera2d spawn.
2. **[Bevy 0.18 API] EventReader renamed to MessageReader** — Events are now "Messages" in Bevy 0.18. Used `Option<MessageReader<WindowResized>>` for MinimalPlugins compatibility.
3. **[HiDPI] Physical vs logical pixels** — WindowResized reports logical pixels, but the WGSL shader's `@builtin(position)` operates in physical pixel space. Fixed by reading `window.physical_width/height()` directly from the Window component.

## Test results
- 124 unit tests pass, 0 failures
- Clippy clean (zero warnings)
- Human visual verification: APPROVED

## Self-Check: PASSED
