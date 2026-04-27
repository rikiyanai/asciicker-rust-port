# Asciicker Rust Port

Rust/Bevy reimplementation of the Asciicker C++ game engine. ASCII/CP437 visual style, custom CPU rasterizer, 6-stage rendering pipeline.

## Execution Guardrails (Non-Negotiable)

These rules override optimistic exploration behavior and prevent false "done" claims.

1. Explore output is never execution proof.
   - Results from "Explore"/"search"/code inspection are hypotheses only.
   - Do not report PASS/COMPLETE based on structure inspection.
2. Runtime evidence is required for completion claims.
   - A phase/subphase can be marked complete only with command-backed evidence.
   - `cargo test` passing is necessary but not sufficient for visual systems.
3. If user reports runtime breakage, reproduction comes first.
   - Stop feature expansion.
   - Reproduce the reported bug with a test.
   - Fix and re-run the same test before claiming progress.
4. No skipped acceptance items without explicit user approval.
   - Maintain a numbered checklist.
   - Any skipped item must be called out as "NOT DONE" with reason.
5. Do not compress failures into "all green" summaries.
   - Report failing tests, broken flows, and missing wiring explicitly.
   - Keep phase status aligned with test results and roadmap/state docs.
   - Treat `.planning/ROADMAP.md` as canonical status authority.
   - Treat `docs/plan-*.md` as execution detail only; they cannot override roadmap status.
   - Treat `.planning/STATE.md` as session context only (not completion evidence).
   - For multi-doc alignment, invoke the `agent-context-doc-health` skill.
6. Verify plan currency before executing.
   - Before starting any phase from a roadmap doc, check git log for commits
     that reference that phase. It may already be implemented.
   - If a phase is already implemented, report "ALREADY DONE - commit <hash>"
     instead of re-implementing.
   - Roadmap `status: draft` does NOT mean unimplemented - verify against code.

## Git State Guardrails (Non-Negotiable)

1. One active delivery branch at a time.
2. Zero-stash policy at session end.
3. Never switch/pull on tracked-dirty worktree.
4. Safety tag before risky operations (rebase/cherry-pick/large merge).
5. PR-first integration.

## Karpathy Guidelines (Mandatory for GSD)

**RULE: Before executing ANY `/gsd:*` command, invoke the `karpathy-guidelines` skill first.**

The four principles (Think Before Coding, Simplicity First, Surgical Changes, Goal-Driven Execution) must be active context for every GSD-driven task.

## Scope Challenge (Mandatory)

**RULE: Push back on requests that are too large, ambiguous, or contradictory.**

Before accepting any non-trivial request:
1. **Evaluate scope** - Is this achievable in one session? One context window?
2. **Decompose or reject** - If >3 distinct deliverables, use `EnterPlanMode` or `AskUserQuestion` to narrow scope.
3. **Surface contradictions** - If requirements conflict, say so.
4. **Name what you won't do** - Explicitly state OUT_OF_SCOPE items.

This applies to user requests AND self-generated subtasks. If a subagent's workload exceeds 400 lines or 3 files, it must decompose or escalate.

## Build & Test Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build
cargo run                      # Run game (debug)
cargo run --release            # Run game (release)

# Tests
cargo test                     # All tests
cargo test --lib               # Unit tests only
cargo test --test integration  # Integration tests
cargo test -- --nocapture      # With stdout

# Linting & Formatting
cargo fmt                      # Format code
cargo fmt -- --check           # Check formatting
cargo clippy                   # Lint
cargo clippy -- -D warnings    # Lint (strict)

# Documentation
cargo doc --open               # Generate and view docs

# Debug Tooling (optional, not in default build)
cargo run --features inspector          # Runtime egui inspector for Resources
cargo run --features schedule_dump -- dump-update-schedule schedule.dot  # Dump system schedule
dot -Tsvg schedule.dot -o schedule.svg  # Render DOT to SVG (requires graphviz)
```

## Context Hygiene

### /prepcompact Workflow
At logical task boundaries, run `/prepcompact` to save a structured snapshot:
- After exploration phase, before writing code
- After completing a milestone or commit
- Before switching between subsystems (rendering vs game logic vs terrain)
- After debugging sessions

### Task-Type Routing

| Situation | Action |
|-----------|--------|
| Debugging with persistent state | `/gsd:debug` - scientific method + file-based checkpoints |
| Multi-step complex feature | `/gsd:plan-phase` - GSD planning with verification loop |
| New feature implementation | `/plan` - design approach, get approval before coding |
| Bug fix or new feature code | `/tdd` - write tests first, then implement |
| After writing/modifying code | `code-reviewer` agent - catch issues immediately |
| Security-sensitive changes | `/security-review` |
| Build failures | `build-error-resolver` agent - minimal surgical fixes |
| Context getting large | `/prepcompact` - snapshot, then `/compact` |
| Resuming previous work | `/pickup` - reload context snapshot |
| Rendering system work | Read `docs/skills/engine-render.md` - traps, data contracts, callgraph |
| World/terrain/BSP work | Read `docs/skills/world-loading.md` - .a3d format, BSP internals |
| Physics work | Read `docs/skills/physics-system.md` - collision, forces, constants |
| Game mechanics work | Read `docs/skills/game-mechanics.md` - character, AI, combat |
| Logging a regression/failure | Append to `docs/FAILURE_LOG.md` - append-only, use status vocab (OPEN/PARTIAL/MONITORING/RESOLVED) |
| Doc/roadmap/plan edits | `/agent-context-doc-health` - drift audit, cross-doc alignment |
| Session handoff or resume | `/agent-context-doc-health` - standardized handoff template |
| Claiming phase complete | `/agent-context-doc-health` - requires commit + verification proof |
| Planning engine port | `/ascii-port-roadmap` - two-track plan, compatibility contracts |

## Project Structure

> **NOTE:** An early skeleton exists at `asciicker-rust/` (Bevy 0.18.0, ~385 LOC, does NOT compile — missing modules: systems, world, loaders, math). It defines some components and stubs but has zero tests and no functional systems. Treat it as ~10-20% scaffolding effort, not working code.
> The target layout below supersedes the skeleton's ad-hoc structure.

```
Cargo.toml                     # Workspace root (to create)
src/
  main.rs                      # Entry point
  lib.rs                       # Library root
  render/                      # CPU rasterizer (port of render.cpp)
    mod.rs                     # 6-stage pipeline
    sample_buffer.rs           # 2x supersampled depth/color buffer
    rasterizer.rs              # Bresenham lines, barycentric triangles
    quantize.rs                # RGB555 -> xterm-256 color quantization
    material.rs                # auto_mat shade tables
  sprite/                      # Sprite system (port of sprite.cpp)
    mod.rs                     # Sprite loading, management
    xp_loader.rs               # .xp format parser
    cp437.rs                   # CP437 glyph handling
    font.rs                    # Font system (port of font1.cpp)
  world/                       # World system (port of world.cpp)
    mod.rs                     # BSP tree, instance management
    bsp.rs                     # BSP construction (SAH)
    mesh.rs                    # Mesh loading (.akm format)
    raycast.rs                 # Plucker coordinate raycasting
  terrain/                     # Terrain system (port of terrain.cpp)
    mod.rs                     # Quadtree heightmap
    patch.rs                   # 5x5 vertex, 8x8 material cells
    shadow.rs                  # Terrain shadow computation
  physics/                     # Physics system (port of physics.cpp)
    mod.rs                     # Sphere collision, TOI sweep
    collision.rs               # Face/edge/vertex tests
    forces.rs                  # Gravity, buoyancy, impulse
  game/                        # Game logic (port of game.cpp)
    mod.rs                     # Game state, main loop
    character.rs               # Character state machine
    equipment.rs               # 5D equipment sprite lookup
    combat.rs                  # Melee/ranged combat
    ai.rs                      # NPC pathfinding
    input.rs                   # Input accumulation
  audio/                       # Audio system
    mod.rs                     # Bevy audio integration
  network/                     # Networking (future)
    mod.rs
docs/                          # Documentation hub
  INDEX.md                     # Master doc index
  skills/                      # Skill packs (C++ subsystem knowledge)
  arch/                        # C++ architecture docs (reference)
  research/                    # Research documents
  plans/                       # Execution plans
.planning/                     # GSD planning infrastructure
  ROADMAP.md                   # Canonical status ledger
  PROJECT.md                   # Acceptance outcomes and scope
  STATE.md                     # Session context
```

## Key Entry Points

> **NOTE:** These entry points are the TARGET architecture. A skeleton exists at `asciicker-rust/` but does not compile. GSD Phase 1 will establish the canonical project structure.

| File | Purpose | Skill Pack |
|------|---------|------------|
| `src/render/mod.rs` | 6-stage ASCII rasterizer | [`engine-render`](docs/skills/engine-render.md) |
| `src/sprite/xp_loader.rs` | .xp sprite loading | [`engine-render`](docs/skills/engine-render.md) |
| `src/world/bsp.rs` | BSP tree, .a3d loader | [`world-loading`](docs/skills/world-loading.md) |
| `src/terrain/mod.rs` | Quadtree heightmaps | [`world-loading`](docs/skills/world-loading.md) |
| `src/physics/mod.rs` | Sphere collision | [`physics-system`](docs/skills/physics-system.md) |
| `src/game/mod.rs` | Game loop, character systems | [`game-mechanics`](docs/skills/game-mechanics.md) |

## Binary Formats (Shared with C++ Engine)

- **.xp** - Sprite: gzip compressed, layers of cells (glyph uint16 + fg RGB + bk RGB)
- **.a3d** - World: header + mesh library + terrain patches + instances + BSP
- **.akm** - Mesh: Blender export via io_asciicker addon

## C++ Reference

Original C++ codebase: `/Users/r/Downloads/asciicker-Y9-2/`
Research documents: `/Users/r/Projects/asciicker rust port/docs/`
Architecture docs: `/Users/r/Projects/asciicker rust port/docs/arch/`

When porting a subsystem:
1. Read the relevant skill pack (docs/skills/) for entrypoints, invariants, known traps
2. Read the architecture doc (docs/arch/) for function-level detail
3. Read the research doc (docs/research/) for analysis and decisions
4. Cross-reference with C++ source in `/Users/r/Downloads/asciicker-Y9-2/`

## Bevy Engine Notes

- **Version:** Bevy 0.18+ (released Jan 2026; pin exact version in Cargo.toml)
- **ECS:** Use Bevy's native ECS for all game logic (replaces C++ DOD structs)
- **Rendering:** Custom CPU rasterizer outputs to Bevy Image texture
- **Audio:** bevy_kira_audio for 16-track mixer
- **Input:** Bevy built-in input system
- **Windowing:** Bevy built-in (winit backend)

## Rust-Specific Guidelines

### Think Before Coding
Trace C++ call graphs with skill packs before porting. State ownership and lifetime assumptions explicitly. Rust's borrow checker will enforce what C++ left implicit.

### Simplicity First
Port C++ patterns directly where they map to Rust idioms. Don't over-abstract. Match the C++ data layout where performance matters. Use `#[derive]` liberally.

### Immutability by Default
Rust enforces this naturally. Use `&mut` only where mutation is required. Prefer returning new values over in-place mutation.

### Surgical Changes
Touch only what you must. One subsystem at a time. Get each module compiling and tested before moving to the next.

### Commit Style
Conventional Commits with scope: `feat(render):`, `fix(terrain):`, `test(physics):`, `refactor(world):`
**Do NOT add AI attribution** to commit messages.

## Documentation Hub

| Doc | Location | Status |
|-----|----------|--------|
| Agent entry point | `docs/agents/AGENTS.md` | Canonical rules |
| Agent protocol | `docs/agents/AGENT_PROTOCOL.md` | Startup and evidence rules |
| Doc index | `docs/INDEX.md` | Master hub |
| Render skill | `docs/skills/engine-render.md` | C++ render subsystem reference |
| World skill | `docs/skills/world-loading.md` | C++ world/terrain reference |
| Physics skill | `docs/skills/physics-system.md` | C++ physics reference |
| Game skill | `docs/skills/game-mechanics.md` | C++ game logic reference |
| Doc health | `.claude/skills/agent-context-doc-health/SKILL.md` | Cross-doc alignment |
| Port roadmap | `.claude/skills/ascii-port-roadmap/SKILL.md` | Migration planning |
| Maintainer | `.claude/skills/maintainer-reliability/SKILL.md` | Claim guard, session hygiene |
| Failure Log | `docs/FAILURE_LOG.md` | Append-only failure record |
