# Documentation Index (Master Hub)

## Start Here
- [AGENTS.md](../AGENTS.md) - Agent entry point and repo guidelines
- [CLAUDE.md](../CLAUDE.md) - Claude-specific memory and instructions
- [Agent Protocol](AGENT_PROTOCOL.md) - Agent startup and evidence rules

## Planning
- [Roadmap](../.planning/ROADMAP.md) - Canonical status ledger (planned - GSD-managed)
- [Project](../.planning/PROJECT.md) - Acceptance outcomes and scope (planned - GSD-managed)

## C++ Architecture Reference (Source Material)
- [Engine Architecture](../ENGINE_ARCHITECTURE.md) - Complete C++ architecture (~1MB)
- [Master Roadmap](../MASTER_ROADMAP.md) - Research phase tracking

### Per-File Documentation (docs/arch/)

#### Game Logic
- [game_cpp_part1.md](arch/game_cpp_part1.md) - Game logic part 1
- [game_cpp_part2.md](arch/game_cpp_part2.md) - Game logic part 2
- [game_cpp_part3.md](arch/game_cpp_part3.md) - Game logic part 3
- [game_app_cpp_part1.md](arch/game_app_cpp_part1.md) - Game application part 1
- [game_app_cpp_part2.md](arch/game_app_cpp_part2.md) - Game application part 2
- [game_logic_cpp.md](arch/game_logic_cpp.md) - Game logic subsystem

#### Rendering
- [render_cpp_part1.md](arch/render_cpp_part1.md) - Rendering pipeline part 1
- [render_cpp_part2.md](arch/render_cpp_part2.md) - Rendering pipeline part 2
- [sprite_cpp.md](arch/sprite_cpp.md) - Sprite system

#### Terrain & World
- [terrain_cpp_part1.md](arch/terrain_cpp_part1.md) - Terrain system part 1
- [terrain_cpp_part2.md](arch/terrain_cpp_part2.md) - Terrain system part 2
- [world_cpp_part1.md](arch/world_cpp_part1.md) - World BSP system part 1
- [world_cpp_part2.md](arch/world_cpp_part2.md) - World BSP system part 2
- [water_cpp.md](arch/water_cpp.md) - Water system
- [weather_cpp.md](arch/weather_cpp.md) - Weather system

#### Core Systems
- [physics_cpp.md](arch/physics_cpp.md) - Physics system
- [input_cpp.md](arch/input_cpp.md) - Input system
- [network_cpp.md](arch/network_cpp.md) - Network system
- [gamepad_cpp.md](arch/gamepad_cpp.md) - Gamepad system
- [editor_cpp.md](arch/editor_cpp.md) - Editor system
- [mainmenu_cpp.md](arch/mainmenu_cpp.md) - Main menu system
- [stb_vorbis_cpp.md](arch/stb_vorbis_cpp.md) - Audio decoding (stb_vorbis)

#### Platform Backends
- [platform_backends_cpp.md](arch/platform_backends_cpp.md) - Platform backends overview
- [x11_cpp.md](arch/x11_cpp.md) - X11 backend
- [mswin_cpp.md](arch/mswin_cpp.md) - Windows backend

#### ASCII Engine (asciiid)
- [asciiid_cpp_part1.md](arch/asciiid_cpp_part1.md) - ASCII engine part 1
- [asciiid_cpp_part2.md](arch/asciiid_cpp_part2.md) - ASCII engine part 2
- [asciiid_cpp_part3.md](arch/asciiid_cpp_part3.md) - ASCII engine part 3

#### Batch Analysis
- [batch_api.md](arch/batch_api.md) - API batch analysis
- [batch_audio.md](arch/batch_audio.md) - Audio batch analysis
- [batch_color.md](arch/batch_color.md) - Color batch analysis
- [batch_gl.md](arch/batch_gl.md) - GL batch analysis
- [batch_headers.md](arch/batch_headers.md) - Headers batch analysis
- [batch_inventory.md](arch/batch_inventory.md) - Inventory batch analysis
- [batch_network.md](arch/batch_network.md) - Network batch analysis
- [batch_sdl.md](arch/batch_sdl.md) - SDL batch analysis
- [batch_small_a.md](arch/batch_small_a.md) - Small files batch A
- [batch_small_b.md](arch/batch_small_b.md) - Small files batch B
- [batch_terminal.md](arch/batch_terminal.md) - Terminal batch analysis
- [batch_undo.md](arch/batch_undo.md) - Undo batch analysis
- [batch_web.md](arch/batch_web.md) - Web batch analysis

#### Handoff
- [HANDOFF_ENGINE_AUDIT.md](arch/HANDOFF_ENGINE_AUDIT.md) - Engine audit handoff

## Skill Packs (C++ Subsystem Knowledge)

### Engine Subsystems
- [Engine Render](skills/engine-render.md) - render.cpp, sprite.cpp, font1.cpp
- [World Loading](skills/world-loading.md) - world.cpp, terrain.cpp, .a3d format
- [Physics System](skills/physics-system.md) - physics.cpp collision and forces
- [Game Mechanics](skills/game-mechanics.md) - game.cpp character, AI, combat

### Agent Operations
- [Agent Context Doc Health](../.claude/skills/agent-context-doc-health/SKILL.md) - Cross-doc alignment
- [Codebase Audit Maintainer](../.claude/skills/codebase-audit-maintainer/SKILL.md) - Evidence-backed claims
- [Maintainer Reliability](../.claude/skills/maintainer-reliability/SKILL.md) - Claim guard, session hygiene

### Research & Migration
- [ASCII Port Roadmap](../.claude/skills/ascii-port-roadmap/SKILL.md) - Two-track planning
- [XP Pipeline Verifier](../.claude/skills/xp-pipeline-verifier/SKILL.md) - Asset pipeline validation

## Research Documents
- [Bevy Engine Research](research-bevy-engine.md) - Bevy framework analysis
- [Bevy ECS Conversion](research-bevy-ecs-conversion.md) - ECS migration strategy
- [Rendering Deep Dive](research-rendering-deep-dive.md) - C++ rendering analysis
- [Implementation Plan](research-implementation-plan.md) - Comprehensive porting plan
- [Mage Core Research](research-mage-core.md) - MageCore engine analysis
- [C++ Architecture Analysis](research-cpp-architecture-analysis.md) - C++ codebase analysis
- [ECS Architecture](research-ecs-architecture.md) - ECS architecture research
- [Bevy ASCII Rendering](research-bevy-ascii-rendering.md) - Bevy ASCII rendering approach
- [Bevy Render Pipeline](research-bevy-render-pipeline.md) - Bevy render pipeline analysis
- [Bevy Migration](research-bevy-migration.md) - Bevy migration strategy
- [Implementation Planning](research-implementation-planning.md) - Implementation planning notes
- [Bevy MageCore Integration](research-bevy-magecore-integration.md) - MageCore integration with Bevy
- [Bevy MageCore Input](research-bevy-magecore-input.md) - Input system integration
- [Bevy MageCore WGPU](research-bevy-magecore-wgpu.md) - WGPU integration
- [Bevy MageCore Texture](research-bevy-magecore-texture.md) - Texture system integration
- [Bug Assumption Audit](research-bug-assumption-audit.md) - Bug and assumption audit
- [AlexHarri Integration](research/alexharri-asciicker-integration.md) - AlexHarri asciicker integration notes
- [AlexHarri ASCII Renderer](alexharri_ascii_renderer_technology.md) - ASCII renderer technology analysis
- [Research Checkpoint](RESEARCH_CHECKPOINT.md) - Research progress checkpoint
- [Research Testing Strategies](research/research-testing-strategies.md) - Testing strategy research
- [Implementation Deep Dive](research-implementation-deep-dive.md) - Deep dive implementation research
- [Strangler Fig Pattern](research-strangler-fig-pattern.md) - Strangler fig migration pattern research

## Process Documents
- [Failure Log](FAILURE_LOG.md) - Issues and blockers
- [Assumption Checklist](ASSUMPTION_MASTER_CHECKLIST.md) - Verified assumptions
- [Risk Register](RISK_REGISTER.md) - Risk tracking and mitigation
- [Decision Log](DECISION_LOG.md) - Key decisions
- [Gaps Analysis](GAPS_ANALYSIS_SUMMARY.md) - Gap categorization
- [Re-Audit Master](RE-AUDIT-MASTER.md) - Master re-audit tracking
- [Implementation Bug Fix Plan](IMPLEMENTATION_BUG_FIX_PLAN.md) - Bug fix implementation plan
- [Implementation Plan](IMPLEMENTATION_PLAN.md) - Implementation plan
- [Roadmap State](ROADMAP_STATE.md) - Current roadmap state tracking
- [Master Index](MASTER_INDEX.md) - Alternate master index

### Audit Phase Reports
- [Audit: Phase 1-2 Granular](audit-phase1-2-granular.md) - Phases 1-2 granular audit
- [Audit: Phase 3-4 Granular](audit-phase3-4-granular.md) - Phases 3-4 granular audit
- [Audit: Phase 3 World](audit-phase3-world.md) - Phase 3 world system audit
- [Audit: Phase 4 Game Logic](audit-phase4-game-logic.md) - Phase 4 game logic audit

### Gaps Detail
- [Gaps: Rendering](gaps-rendering.md)
- [Gaps: Game Logic](gaps-game-logic.md)
- [Gaps: Terrain & World](gaps-terrain-world.md)
- [Gaps: Systems](gaps-systems.md)
- [Gaps: Integration](gaps-integration.md)

### Plans
- [Plan: Rendering Gaps](plan-rendering-gaps.md)
- [Plan: Game Logic Gaps](plan-game-logic-gaps.md)
- [Plan: Systems Gaps](plan-systems-gaps.md)
- [Plan: Integration Decisions](plan-integration-decisions.md)
- [Plan: Ancestor Cleanup](plan-ancestor-cleanup.md)
- [Plan: SampleBuffer Bridge](plan-SampleBuffer-bridge.md)
- [Plan: Terrain Fix](implementation-plan-terrain-fix.md)

### Audit Documents
- [Audit: Assumptions Verified](audit-assumptions-verified.md)
- [Audit: Unknowns Categorized](audit-unknowns-categorized.md)
- [Audit: Re-audit Critical](audit-reaudit-critical.md)
- [Audit: Re-audit Critical Rendering](audit-reaudit-critical-rendering.md)
- [Audit: Re-audit High Rendering](audit-reaudit-high-rendering.md)
- [Audit: Re-audit High Visual](audit-reaudit-high-visual.md)
- [Audit: Re-audit Terrain](audit-reaudit-terrain.md)
- [Audit: Re-audit AlexHarri](audit-reaudit-alexharri.md)

### Audit Unknowns (Individual Investigations)
- [Unknown: HEIGHT_SCALE](audit-unknown-HEIGHT_SCALE.md)
- [Unknown: Terrain Expansion](audit-unknown-terrain-expansion.md)
- [Unknown: XP Format](audit-unknown-xp-format.md)
- [Unknown: XP Layer Semantics](audit-unknown-xp-layer-semantics.md)
- [Unknown: Node Merge](audit-unknown-node-merge.md)
- [Unknown: Perspective Matrix](audit-unknown-perspective-matrix.md)
- [Unknown: A3D Format](audit-unknown-a3d-format.md)
- [Unknown: Audio Details](audit-unknown-audio-details.md)
- [Unknown: KDTree Params](audit-unknown-kdtree-params.md)
- [Unknown: Diag Bit](audit-unknown-diag-bit.md)
- [Unknown: Glyph Coverage](audit-unknown-glyph-coverage.md)
- [Unknown: Mesh Reference](audit-unknown-mesh-reference.md)
- [Unknown: Animation Timing](audit-unknown-animation-timing.md)
- [Unknown: Final Batch](audit-unknown-final-batch.md)

## Code Documentation (Extracted Constants)
- [A3D Keycodes](codedoc-a3d-keycodes.md)
- [A3D World Format](../codedoc-a3d-world-format.md) - A3D binary format specification
- [Animation Timing](codedoc-animation-timing.md)
- [Camera Parameters](codedoc-camera-parameters.md)
- [Physics Constants](../codedoc-physics-constants.md) - Extracted physics constants
- [XP Terrain Format](codedoc-xp-terrain-format.md)

## Engine Port Reference (Mage Core Analysis)
- [Architecture Mapping](engine-port/2026-02-18-architecture-mapping.md) - C++→Rust architecture mapping
- [Capability Matrix](engine-port/2026-02-18-capability-matrix.md) - Feature coverage analysis
- [Decision Inputs](engine-port/2026-02-18-decision-inputs.md) - Engine decision analysis
- [Plan](engine-port/2026-02-18-plan.md) - Porting plan
- [Port Options Comparison](engine-port/2026-02-18-port-options-comparison.md) - Rendering approach comparison
- [Timeline Estimate](engine-port/2026-02-18-timeline-estimate.md) - Implementation timeline
- [Verification Checklist](engine-port/2026-02-18-verification-checklist.md) - Verification gates

## Archive (Legacy Documents)
- [docs/archive/](archive/) - Legacy and superseded documents
- [AUDIT_MANIFEST-legacy.md](archive/AUDIT_MANIFEST-legacy.md) - Legacy audit manifest
- [agent-legacy.md](archive/agent-legacy.md) - Legacy agent docs

### Archived Engine Port (MageCore)
- [docs/archive/engine-port-magecore/](archive/engine-port-magecore/) - Archive backup copies of engine port docs (active copies restored to [docs/engine-port/](engine-port/))
- [Architecture Mapping](archive/engine-port-magecore/2026-02-18-architecture-mapping.md)
- [Capability Matrix](archive/engine-port-magecore/2026-02-18-capability-matrix.md)
- [Decision Inputs](archive/engine-port-magecore/2026-02-18-decision-inputs.md)
- [Plan](archive/engine-port-magecore/2026-02-18-plan.md)
- [Port Options Comparison](archive/engine-port-magecore/2026-02-18-port-options-comparison.md)
- [Timeline Estimate](archive/engine-port-magecore/2026-02-18-timeline-estimate.md)
- [Verification Checklist](archive/engine-port-magecore/2026-02-18-verification-checklist.md)

## Verification Evidence
- [docs/verification/](verification/) - Evidence artifacts directory (empty - populated by verification runs)
