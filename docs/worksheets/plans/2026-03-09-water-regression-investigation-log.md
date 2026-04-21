# 2026-03-09 Water Regression Investigation Log

## Goal

Record the exact water/reflection regression investigation sequence, what was tried, what failed, what evidence was collected, and which repo state was restored as the working baseline.

## Restore Baseline

- Requested backup snapshot: `3a621b818c05689a57835548fcdd3552dd3a6b56`
- Safety tag created before restore: `safety-restore-20260309-before-3a621b8`
- Pre-restore patch snapshot written to:
  `artifacts/session_snapshots/2026-03-09-pre-restore-to-3a621b8.patch`
- Current tracked files were restored to the `3a621b8` snapshot contents with `git restore --source 3a621b8 --worktree --staged .`

Important distinction:
- `3a621b8` is a docs-tip commit, but restoring to it restores the entire repo snapshot at that point in history, including all code beneath it.
- `32efee5` was kept separately as an older renderer rollback baseline, not as the requested backup snapshot.

## Isolated Comparison States

Two isolated worktrees were created so runtime comparisons would not disturb the main checkout:

- `artifacts/worktrees/backup-3a621b8`
  Exact backup snapshot at `3a621b8`
- `artifacts/worktrees/rollback-32efee5`
  Older render baseline at `32efee56ea63c4ea47c201a43a18c9eda32bafc8`

## What Was Tried

### 1. Retry native V8/Homebrew build while other work proceeded

Result:
- Not directly relevant to the render regression.
- Homebrew was still building `llvm`, so this did not produce a useful V8 outcome in this investigation window.

### 2. Build watchdog / visual test harness

Files added in the earlier working state:
- `engine-port/scripts/testing/watchdog.mjs`
- `engine-port/scripts/testing/lib/png_diff.mjs`
- `engine-port/tests/visual-baselines/*`

Result:
- The watchdog worked for static baseline comparisons.
- It was not sufficient for the water regression because the bug looked time-based and camera-sensitive.

Why it failed for this case:
- It could tell that rendered output drifted, but not whether drift came from replay state differences or render instability under identical state.

### 3. Investigate water as missing Perlin or missing water-plane animation

Changes tried:
- Added animated water-plane motion to the Rust port to match original `render.cpp`.
- Adjusted water ripple sampling to use reconstructed water/world-space coordinates.
- Tightened ripple gating closer to the original full-block underwater condition.

Result:
- The water still presented as a blank or nearly invisible surface.
- The user still saw reflection-like behavior and a broken overlay look.

Why it failed:
- The remaining defect is not explained by missing Perlin or missing water-plane bobbing alone.
- The comparison evidence later showed the problem persisted under fixed replay state, pointing upstream to resolve/compositing or a related time-varying render path.

### 4. Investigate old reflection-overlay regression

Reference:
- Old reflection overwrite issue already logged as `F241` in `docs/FAILURE_LOG.md`

Result:
- The old full-frame overwrite bug had already been marked resolved.
- The current symptom did not match that exact old failure.

Why this did not close the bug:
- The present issue appears subtler: large render divergence, likely mixed compositing or time-varying layer behavior, not just a whole-frame reflection overwrite.

### 5. Run the exact backup snapshot in isolation

Work done:
- Linked the required map asset into the isolated `backup-3a621b8` worktree.
- Linked missing mesh assets so the comparison run was representative.
- Verified the isolated snapshot passed `Loading` and reached `Playing`.

Result:
- The backup snapshot still looked broken, but visibly better than the current experimental state.

Why this mattered:
- It established that the working target should be the exact backup snapshot first, not the later local experiments.

### 6. Add deterministic capture + replay harness

Added in the current checkout first, then ported minimally into the backup worktree:
- `engine-port/src/output/capture.rs`
- `engine-port/src/output/replay.rs`
- `engine-port/scripts/compare_replay_dirs.py`

Harness behavior:
- Records `trace.jsonl` with camera/player/water state
- Writes per-frame `.xp` output
- Replays the same trace across snapshots
- Compares frame directories

Result:
- This was the first useful diagnostic tool for the regression.

### 7. Record a 120-frame baseline from the backup snapshot

Output:
- `artifacts/baselines/backup-3a621b8-run2`

Observed metadata:
- Camera yaw constant at `45.0`
- Zoom constant at `1.0`
- Water constant at `55` raw / `3.4375` world
- Camera/player positions only changed by tiny floating-point drift

Observed render behavior:
- Despite almost-static metadata, XP frame hashes changed over time

Why this mattered:
- It proved the visible bob/shift was not simply caused by meaningful movement state changes.

### 8. Replay that exact trace on the current checkout and compare

Replay output:
- `artifacts/baselines/current-head-run1`

Detailed comparison artifact:
- `artifacts/comparisons/backup-3a621b8-vs-current-head-detailed.json`

Observed result:
- Current output differed from the backup in roughly `3088` to `3932` cells per frame
- That is about `53.6%` to `68.3%` of the screen
- Mismatches were heavily concentrated in foreground/background compositing, with additional glyph substitutions

Why this mattered:
- It ruled out replay drift as the main cause.
- The regression is render-side under the same recorded state.

## What Failed and What That Means

### Failed: Perlin/water-plane fixes as the main solution

Reason:
- They did not materially fix the blank-water / overlay symptom.
- Deterministic comparison later showed a broader render divergence.

### Failed: metadata that only captured coordinates/yaw/zoom

Reason:
- That metadata could say the camera was stable.
- It could not explain why the image still changed, or which exact cells changed frame to frame.

Implication:
- Capture metadata needed to include frame-to-frame cell diffs, not just replay state.

### Failed: assuming the user-visible bobbing was just movement or camera drift

Reason:
- Deterministic replay held the state effectively constant.
- The output still changed, and current diverged sharply from backup.

Implication:
- The bug is in rendering, compositing, or another time-varying visual layer.

## Main Findings

- The exact backup snapshot is `3a621b8`, and that is now the requested restore target.
- `32efee5` remains useful only as an older renderer reference point.
- The current render regression is reproducible under replay with matched camera/player/water state.
- Existing metadata was not informative enough because it did not describe exact cell changes between frames.
- The correct next investment is stronger observability:
  per-frame diff metadata, exact changed-cell samples, and replayable scripted runs.

## Next Work

1. Keep the main checkout restored to the `3a621b8` snapshot contents.
2. Reattach the deterministic capture harness on top of that restored state.
3. Expand frame JSON so each captured frame includes:
   - replay state
   - XP/hash summary
   - non-space cell count
   - exact previous-frame change counts
   - sampled changed cells with old/new glyph, fg, and bg
4. Re-run a short capture on the restored snapshot and verify the JSON is explanatory enough to diagnose time-varying render bugs.
