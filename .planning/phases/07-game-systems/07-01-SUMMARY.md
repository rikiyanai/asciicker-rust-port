---
phase: 07-game-systems
plan: 01
subsystem: audio
tags: [bevy_kira_audio, kira, audio-mixer, dynamic-channels, round-robin]

# Dependency graph
requires:
  - phase: 06-physics-and-character
    provides: GamePlugin, main.rs plugin tuple, Bevy 0.18 app structure
provides:
  - AsciickerAudioPlugin with bevy_kira_audio 0.25 integration
  - AudioMixer resource with 16-track round-robin (PLY_TRACKS=16)
  - PlaySoundEvent message for ECS-idiomatic sound playback
  - 16 named DynamicAudioChannels (track_0 through track_15)
affects: [07-02, 07-03, 06-02]

# Tech tracking
tech-stack:
  added: [bevy_kira_audio 0.25, kira 0.10]
  patterns: [DynamicAudioChannels for runtime channel count, amplitude-to-decibels conversion, Message-based sound events]

key-files:
  created:
    - engine-port/src/audio/mod.rs
    - engine-port/src/audio/mixer.rs
  modified:
    - engine-port/Cargo.toml
    - engine-port/Cargo.lock
    - engine-port/src/lib.rs
    - engine-port/src/main.rs

key-decisions:
  - "Volume stored as linear amplitude (0.0-1.0) internally, converted to kira::Decibels at play time"
  - "Startup system creates all 16 channels (not lazy initialization) for deterministic channel availability"
  - "AsciickerAudioPlugin registered BEFORE GamePlugin in main.rs tuple (R17-F214)"
  - "Events drained unconditionally (P7-055) to prevent accumulation outside Playing state"
  - "kira::Decibels(f32) is the actual Volume type (not Volume::Amplitude as plan speculated)"

patterns-established:
  - "Message-based audio: send PlaySoundEvent from any system, play_sound_system processes in Update"
  - "Round-robin track assignment: mixer.next_track() cycles 0..15 automatically"
  - "Amplitude-to-decibels conversion: 0.0 -> -60dB (silence), 1.0 -> 0dB (unity)"

requirements-completed: [AUD-01, AUD-02]

# Metrics
duration: 16min
completed: 2026-02-25
---

# Phase 7 Plan 01: Audio Subsystem Summary

**bevy_kira_audio 0.25 integration with 16-track round-robin mixer matching C++ PlyTrack[16] architecture**

## Performance

- **Duration:** 16 min
- **Started:** 2026-02-25T14:54:02Z
- **Completed:** 2026-02-25T15:10:31Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Integrated bevy_kira_audio 0.25 (confirmed Bevy 0.18 compatible, kira 0.10 backend)
- AudioMixer resource with 16-track round-robin, master + per-track volume, amplitude-to-dB conversion
- PlaySoundEvent as Bevy 0.18 Message for ECS-idiomatic sound playback from any system
- 28 unit tests covering round-robin, volume clamping, effective volume, decibel conversion, channel creation
- 379 total lib tests passing (28 new audio tests, 0 regressions)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add bevy_kira_audio dependency and create AudioPlugin** - `408d84c` (feat)
2. **Task 2: Implement AudioMixer with 16-track round-robin and tests** - `3a68c0a` (test)

## Files Created/Modified
- `engine-port/src/audio/mod.rs` - AsciickerAudioPlugin wrapping bevy_kira_audio::AudioPlugin, channel init, play system
- `engine-port/src/audio/mixer.rs` - AudioMixer resource, PlaySoundEvent message, play_sound_system, 28 unit tests
- `engine-port/Cargo.toml` - Added bevy_kira_audio = "0.25" dependency
- `engine-port/Cargo.lock` - Locked 14 new packages (kira 0.10.8, symphonia, etc.)
- `engine-port/src/lib.rs` - Added `pub mod audio;` declaration
- `engine-port/src/main.rs` - Registered AsciickerAudioPlugin before GamePlugin

## Decisions Made
- **Volume API discovery:** Plan predicted `Volume::Amplitude(f64)` but actual kira 0.10 API uses `Decibels(f32)` via `impl Into<Decibels>`. Stored amplitude internally, convert at play time using `20 * log10(amplitude)`.
- **Channel initialization:** Used a Startup system (`initialize_audio_channels`) rather than lazy init, ensuring all 16 channels exist before any PlaySoundEvent is processed.
- **Plugin ordering:** AsciickerAudioPlugin placed before GamePlugin in main.rs tuple per R17-F214 FIX.
- **Event drain:** play_sound_system drains events unconditionally per P7-055 FIX (no state gating).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Volume API uses Decibels, not Volume::Amplitude**
- **Found during:** Task 1 (API verification)
- **Issue:** Plan specified `Volume::Amplitude(f64)` (P7-044 FIX) but bevy_kira_audio 0.25 / kira 0.10 uses `kira::Decibels(f32)` with `impl Into<Decibels>` on `with_volume()`
- **Fix:** Store amplitude (0.0-1.0) internally, convert to decibels at play time via `amplitude_to_decibels()`
- **Files modified:** engine-port/src/audio/mixer.rs
- **Verification:** Unit test `test_amplitude_to_decibels_full` / `_half` / `_zero_is_silence` pass
- **Committed in:** 408d84c (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug - API mismatch)
**Impact on plan:** Minimal. The amplitude-to-decibels conversion is a simple formula. Internal API unchanged.

## Issues Encountered
- Pre-existing clippy warnings in `terrain_shader.rs` (collapsible_if, too_many_arguments) are unrelated to this plan and not fixed per SCOPE BOUNDARY rule.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Audio subsystem ready for integration with game systems (Plan 07-02 GamePlugin, Plan 06-02 footstep sounds)
- PlaySoundEvent provides the interface for any system to trigger audio playback
- 16 channels available for concurrent sound effects without starvation

## Self-Check: PASSED

- FOUND: engine-port/src/audio/mod.rs
- FOUND: engine-port/src/audio/mixer.rs
- FOUND: engine-port/Cargo.toml
- FOUND: engine-port/src/lib.rs
- FOUND: engine-port/src/main.rs
- FOUND: .planning/phases/07-game-systems/07-01-SUMMARY.md
- FOUND: commit 408d84c
- FOUND: commit 3a68c0a

---
*Phase: 07-game-systems*
*Completed: 2026-02-25*
