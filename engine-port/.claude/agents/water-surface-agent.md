# Water Surface Agent

Inherit rules from `base-browser-story.md`.

## Scope

Validate water-surface activation and frame/noise progression.

## Flow

1. Open base URL.
2. Ensure game state is `playing`.
3. Activate water surface.
4. Advance water frame.
5. Verify visual/surface state changed and log screenshots.

## Runner

- `node scripts/testing/e2e.mjs --feature water`
