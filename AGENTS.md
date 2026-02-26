# Agent Entry Point (Canonical)

## Master Memory
- Canonical doc hub: `docs/INDEX.md` (add new high-signal docs there).
- Agent protocol: `docs/AGENT_PROTOCOL.md`.
- Claude-specific memory: `CLAUDE.md`.

---

# Repository Guidelines

## Project Context
This is a Rust/Bevy reimplementation of the Asciicker C++ game engine (~82K lines across 48 files). The project is transitioning from research phase to implementation.

## Module Organization
- Rust source will live in `src/` organized by subsystem (render, sprite, world, terrain, physics, game, audio, network). An early skeleton exists at `asciicker-rust/` (~385 LOC, Bevy 0.18.0, does NOT compile). GSD Phase 1 will establish the canonical project structure.
- Documentation in `docs/` with skill packs in `docs/skills/`.
- C++ architecture reference in `docs/arch/`.
- Research documents in `docs/research/`.
- Planning artifacts in `.planning/` (planned — created by GSD initialization).
- Agent/skill configs in `.claude/`.

## Build, Test, and Development Commands
- `cargo build` builds all targets.
- `cargo test` runs all tests.
- `cargo clippy` runs lints.
- `cargo fmt` formats code.
- `cargo run` runs the game.

## Coding Style & Naming Conventions
- Rust 2021 edition, idiomatic Rust patterns.
- snake_case for functions/variables, PascalCase for types, SCREAMING_SNAKE for constants.
- Prefer `impl` blocks over free functions where ownership is clear.
- Use `#[derive(Debug, Clone)]` on all public types.
- No `unsafe` without explicit justification and comment.

## Testing Guidelines
- Unit tests in `#[cfg(test)]` modules within source files.
- Integration tests in `tests/` directory.
- Test naming: `test_<function>_<scenario>_<expected>`.
- Visual comparison tests use golden file snapshots.
- Minimum 80% coverage target.

## Commit & Pull Request Guidelines
- Conventional Commits: `feat(render):`, `fix(terrain):`, `test:`, `refactor(world):`
- Keep commits focused; one subsystem per commit when possible.
- **Do NOT add AI attribution** to commit messages.

## Git Workflow Guardrails (Mandatory)
- One active delivery branch at a time.
- Zero-stash policy at session end.
- Do not run `git switch` or `git pull` with tracked changes.
- Create a safety tag before risky operations.
- PR-based integration to `main`.

## Subsystem Skill Packs
Before porting or modifying a C++ subsystem, read the relevant skill pack:
- **Render/Sprite**: `docs/skills/engine-render.md` - render pipeline, sprites, palette quantization
- **World/Terrain**: `docs/skills/world-loading.md` - BSP tree, terrain, .a3d format
- **Physics**: `docs/skills/physics-system.md` - collision, forces, constants
- **Game Mechanics**: `docs/skills/game-mechanics.md` - character, AI, combat, equipment

## Agent Context & Doc Health
Before editing roadmaps, plans, or state docs, invoke the `agent-context-doc-health` skill:
- **Skill path**: `.claude/skills/agent-context-doc-health/SKILL.md`
- **Triggers**: Multi-agent doc edits, completion claims, branch switches, session handoffs
- **Canonical source priority**: `AGENTS.md` > `docs/INDEX.md` > `docs/AGENT_PROTOCOL.md` > `.planning/ROADMAP.md` > `.planning/PROJECT.md` > `docs/plans/*` > `.planning/STATE.md` > `CLAUDE.md` > git evidence
- For failure tracking: append to `docs/FAILURE_LOG.md` (status vocab: OPEN, PARTIAL, MONITORING, RESOLVED)

<!-- codex-conductor:start -->
## Conductor Guardrail
Always run `conductor:status` first.

- Command alias: `conductor_status`
- Direct command: `python3 scripts/conductor_tools.py status --auto-setup`
- Behavior: if Conductor is missing, status runs setup and creates the baseline.
<!-- codex-conductor:end -->
