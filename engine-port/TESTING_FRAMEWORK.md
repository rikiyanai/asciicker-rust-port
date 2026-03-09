# Claude Browser Testing Framework

This project now includes a Claude-oriented browser testing scaffold under `scripts/testing/`
with screenshot-first visual trails.

## Stack Mapping

1. Layer 1: Skills
- `.claude/skills/browser-testing/SKILL.md`
- Core capabilities: `open_url`, `click_element`, `type_text`, `capture_screenshot`

2. Layer 2: Subagents
- `.claude/agents/menu-agent.md`
- `.claude/agents/water-surface-agent.md`
- `.claude/agents/base-browser-story.md`

3. Layer 3: Slash Commands
- `.claude/commands/test-smoke.md`
- `.claude/commands/test-e2e.md`
- `.claude/commands/test-parallel.md`

4. Layer 4: Task Runner
- `justfile`

## Screenshot-First Artifact Contract

Each test run writes to:

- `artifacts/testing/<run-id>/<story>/trail.jsonl`
- `artifacts/testing/<run-id>/<story>/summary.json`
- `artifacts/testing/<run-id>/<story>/<step>-<action>.png`

Every browser action is followed by a screenshot and a structured log entry.

## Environment Discovery (Current Project)

Phase-1 checks found:

- `node`, `npm`, `npx`: installed
- `playwright` CLI: installed
- `just`: not installed on this machine (install separately if you want to use `just`)
- web login/session flows in this repo: not applicable to current fixture stories

Because the engine is a desktop Bevy app, the framework ships with a local fixture site:

- server command: `node scripts/testing/serve_fixture.mjs --port 4173`
- local URL: `http://127.0.0.1:4173`

You can still point tests to any real web UI via `--base-url`.

## Usage

Without `just`:

```bash
node scripts/testing/smoke.mjs
node scripts/testing/e2e.mjs --feature full
node scripts/testing/parallel.mjs --suite core --workers 3
```

With `just`:

```bash
just install-testing-deps
just test-smoke
just test-e2e feature=full
just test-parallel suite=core workers=3
just test-debug
```

Debug/headed mode:

```bash
node scripts/testing/smoke.mjs --debug 1
```
