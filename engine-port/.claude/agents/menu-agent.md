# Menu Agent

Inherit rules from `base-browser-story.md`.

## Scope

Validate menu-to-playing transition and camera pan interaction.

## Flow

1. Open base URL.
2. Click Start Game.
3. Verify game state is `playing`.
4. Pan camera once and capture visual trail.

## Runner

- `node scripts/testing/e2e.mjs --feature menu`
