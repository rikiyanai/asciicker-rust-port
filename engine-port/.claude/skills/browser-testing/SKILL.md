# Browser Testing Skill (Claude)

Use this skill when validating web UI flows in this project.

## Goal

Provide a screenshot-first, action-by-action browser automation loop.

## Layer 1 Tooling Contract

Core actions are implemented in `scripts/testing/lib/browser_skill.mjs`:

- `open_url`
- `click_element`
- `type_text`
- `capture_screenshot`

Every action must produce a screenshot artifact via `visual_trail`.

## Visual Trail Rule (Mandatory)

After each interaction:

1. Capture screenshot.
2. Append structured event to `trail.jsonl`.
3. Print artifact path in logs.

Artifacts live under:

- `artifacts/testing/<run-id>/<story>/`

## Recommended Execution Paths

- Smoke: `node scripts/testing/smoke.mjs`
- Full E2E: `node scripts/testing/e2e.mjs --feature full`
- Parallel stories: `node scripts/testing/parallel.mjs --suite core --workers 3`

For headed debugging:

- `node scripts/testing/smoke.mjs --debug 1`
