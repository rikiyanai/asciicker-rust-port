# PROJECT MASTER INDEX

## Asciicker Rust Port - Complete File Inventory

**Last Updated:** 2026-02-20
**Total Files:** 135 (active) + 9 (archived)

---

## PROJECT STRUCTURE

```
asciicker rust port/
├── docs/agents/AGENTS.md           # Agent roster and coordination protocol
├── docs/agents/CLAUDE.md           # Project context for Claude Code
├── ENGINE_ARCHITECTURE.md          # High-level engine architecture overview
├── MASTER_ROADMAP.md               # Single source of truth for roadmap
├── codedoc-a3d-world-format.md     # A3D binary format documentation
├── codedoc-physics-constants.md    # Physics constants documentation
│
└── docs/
    ├── MASTER_INDEX.md             # This file
    ├── INDEX.md                    # Lightweight quick-reference index
    ├── agents/AGENT_PROTOCOL.md    # Agent operating protocol
    │
    ├── PROCESS FILES (5 files)
    │   ├── ROADMAP_STATE.md               # Current state tracker
    │   ├── FAILURE_LOG.md                 # Issues and blocks log
    │   ├── ASSUMPTION_MASTER_CHECKLIST.md # All assumptions verified
    │   ├── RISK_REGISTER.md               # Risk tracking
    │   └── DECISION_LOG.md                # Key decisions made
    │
    ├── GAPS FILES (5 files)
    │   ├── GAPS_ANALYSIS_SUMMARY.md       # Master gaps summary
    │   ├── gaps-game-logic.md
    │   ├── gaps-integration.md
    │   ├── gaps-rendering.md
    │   ├── gaps-systems.md
    │   └── gaps-terrain-world.md
    │
    ├── RESEARCH (21 files)
    │   ├── alexharri_ascii_renderer_technology.md
    │   ├── research-bevy-ascii-rendering.md
    │   ├── research-bevy-ecs-conversion.md
    │   ├── research-bevy-engine.md
    │   ├── research-bevy-magecore-input.md
    │   ├── research-bevy-magecore-integration.md
    │   ├── research-bevy-magecore-texture.md
    │   ├── research-bevy-magecore-wgpu.md
    │   ├── research-bevy-migration.md
    │   ├── research-bevy-render-pipeline.md
    │   ├── research-bug-assumption-audit.md
    │   ├── research-cpp-architecture-analysis.md
    │   ├── research-ecs-architecture.md
    │   ├── research-implementation-deep-dive.md
    │   ├── research-implementation-plan.md
    │   ├── research-implementation-planning.md
    │   ├── research-mage-core.md
    │   ├── research-rendering-deep-dive.md
    │   ├── research-strangler-fig-pattern.md
    │   │
    │   └── research/                           # Subfolder
    │       ├── alexharri-asciicker-integration.md
    │       └── research-testing-strategies.md
    │
    ├── AUDIT (29 files)
    │   ├── RE-AUDIT-MASTER.md
    │   ├── RESEARCH_CHECKPOINT.md
    │   ├── audit-assumptions-verified.md
    │   ├── audit-unknowns-categorized.md
    │   │
    │   ├── audit-phase*.md (4 files)
    │   │   ├── audit-phase1-2-granular.md
    │   │   ├── audit-phase3-4-granular.md
    │   │   ├── audit-phase3-world.md
    │   │   └── audit-phase4-game-logic.md
    │   │
    │   ├── audit-reaudit-*.md (6 files)
    │   │   ├── audit-reaudit-alexharri.md
    │   │   ├── audit-reaudit-critical-rendering.md
    │   │   ├── audit-reaudit-critical.md
    │   │   ├── audit-reaudit-high-rendering.md
    │   │   ├── audit-reaudit-high-visual.md
    │   │   └── audit-reaudit-terrain.md
    │   │
    │   └── audit-unknown-*.md (14 files)
    │       ├── audit-unknown-a3d-format.md
    │       ├── audit-unknown-animation-timing.md
    │       ├── audit-unknown-audio-details.md
    │       ├── audit-unknown-diag-bit.md
    │       ├── audit-unknown-final-batch.md
    │       ├── audit-unknown-glyph-coverage.md
    │       ├── audit-unknown-HEIGHT_SCALE.md
    │       ├── audit-unknown-kdtree-params.md
    │       ├── audit-unknown-mesh-reference.md
    │       ├── audit-unknown-node-merge.md
    │       ├── audit-unknown-perspective-matrix.md
    │       ├── audit-unknown-terrain-expansion.md
    │       ├── audit-unknown-xp-format.md
    │       └── audit-unknown-xp-layer-semantics.md
    │
    ├── PLANS (9 files)
    │   ├── IMPLEMENTATION_PLAN.md
    │   ├── IMPLEMENTATION_BUG_FIX_PLAN.md
    │   ├── implementation-plan-terrain-fix.md
    │   ├── plan-ancestor-cleanup.md
    │   ├── plan-game-logic-gaps.md
    │   ├── plan-integration-decisions.md
    │   ├── plan-rendering-gaps.md
    │   ├── plan-SampleBuffer-bridge.md
    │   └── plan-systems-gaps.md
    │
    ├── CODE DOCS (4 files, docs/ only — 2 more at root)
    │   ├── codedoc-a3d-keycodes.md
    │   ├── codedoc-animation-timing.md
    │   ├── codedoc-camera-parameters.md
    │   └── codedoc-xp-terrain-format.md
    │
    ├── SKILLS (4 files)
    │   ├── engine-render.md
    │   ├── game-mechanics.md
    │   ├── physics-system.md
    │   └── world-loading.md
    │
    ├── ARCH (42 files)
    │   ├── ENGINE_ARCHITECTURE.md   (see also root ENGINE_ARCHITECTURE.md)
    │   ├── HANDOFF_ENGINE_AUDIT.md
    │   │
    │   ├── asciiid_cpp_part1.md
    │   ├── asciiid_cpp_part2.md
    │   ├── asciiid_cpp_part3.md
    │   │
    │   ├── game_cpp_part1.md
    │   ├── game_cpp_part2.md
    │   ├── game_cpp_part3.md
    │   ├── game_app_cpp_part1.md
    │   ├── game_app_cpp_part2.md
    │   ├── game_logic_cpp.md
    │   │
    │   ├── render_cpp_part1.md
    │   ├── render_cpp_part2.md
    │   │
    │   ├── terrain_cpp_part1.md
    │   ├── terrain_cpp_part2.md
    │   │
    │   ├── world_cpp_part1.md
    │   ├── world_cpp_part2.md
    │   │
    │   ├── sprite_cpp.md
    │   ├── editor_cpp.md
    │   ├── input_cpp.md
    │   ├── mainmenu_cpp.md
    │   ├── network_cpp.md
    │   ├── physics_cpp.md
    │   ├── platform_backends_cpp.md
    │   ├── stb_vorbis_cpp.md
    │   ├── water_cpp.md
    │   ├── weather_cpp.md
    │   ├── x11_cpp.md
    │   ├── mswin_cpp.md
    │   ├── gamepad_cpp.md
    │   │
    │   └── batch_*.md (14 files)
    │       ├── batch_api.md
    │       ├── batch_audio.md
    │       ├── batch_color.md
    │       ├── batch_gl.md
    │       ├── batch_headers.md
    │       ├── batch_inventory.md
    │       ├── batch_network.md
    │       ├── batch_sdl.md
    │       ├── batch_small_a.md
    │       ├── batch_small_b.md
    │       ├── batch_terminal.md
    │       ├── batch_undo.md
    │       ├── batch_web.md
    │       └── (note: no audio_cpp.md — audio documented in batch_audio.md)
    │
    ├── engine-port/ (7 files — Bevy/magecore evaluation, 2026-02-18)
    │   ├── 2026-02-18-architecture-mapping.md
    │   ├── 2026-02-18-capability-matrix.md
    │   ├── 2026-02-18-decision-inputs.md
    │   ├── 2026-02-18-plan.md
    │   ├── 2026-02-18-port-options-comparison.md
    │   ├── 2026-02-18-timeline-estimate.md
    │   └── 2026-02-18-verification-checklist.md
    │
    ├── archive/ (9 files — superseded docs)
    │   ├── agent-legacy.md                     # Superseded by AGENTS.md
    │   ├── AUDIT_MANIFEST-legacy.md            # Superseded, no longer active
    │   └── engine-port-magecore/ (7 files)     # Same content as engine-port/
    │       └── 2026-02-18-*.md
    │
    ├── implementation-plan/    (EMPTY directory)
    ├── plans/                  (EMPTY directory)
    └── verification/           (EMPTY directory)
```

---

## FILE CATEGORIES SUMMARY

| Category | Count | Location | Description |
|----------|-------|----------|-------------|
| Root files | 6 | `/*.md` | Entry points, architecture, code docs |
| Process | 5 | `docs/` | State, failures, assumptions, risks, decisions |
| Gaps | 6 | `docs/` | Gap analysis documents (master + 5 topic files) |
| Research | 21 | `docs/` + `docs/research/` | Technology analysis, integration |
| Audit | 29 | `docs/` | Unknowns, re-audits, phase audits, checkpoints |
| Plans | 9 | `docs/` | Implementation, bug fixes, granular tasks |
| Code Docs | 4 | `docs/` | C++ format/constant documentation (+ 2 at root) |
| Skills | 4 | `docs/skills/` | Skill packs for engine subsystems |
| Arch | 42 | `docs/arch/` | Function-level C++ documentation |
| Engine-port | 7 | `docs/engine-port/` | Bevy/magecore port evaluation (2026-02-18) |
| Other | 3 | `docs/` | MASTER_INDEX, INDEX, AGENT_PROTOCOL |
| **ACTIVE TOTAL** | **135** | | Complete project documentation |
| Archive | 9 | `docs/archive/` | Superseded documents (not counted above) |

---

## KNOWN OMISSIONS IN ARCH

The following C++ source files do NOT have dedicated arch docs:

- `audio.cpp` — audio is covered in `docs/arch/batch_audio.md` and `docs/arch/stb_vorbis_cpp.md`

---

## DOCUMENT STATUS LEGEND

| Symbol | Meaning |
|--------|---------|
| Complete/Verified | Confirmed accurate |
| In Progress | Being updated |
| Pending | Not yet written |
| Blocked/Failed | Blocked or obsolete |

---

## QUICK REFERENCE

| Need... | Look Here |
|---------|-----------|
| Current status | `MASTER_ROADMAP.md` (root) |
| Implementation plan | `docs/IMPLEMENTATION_PLAN.md` |
| All assumptions | `docs/ASSUMPTION_MASTER_CHECKLIST.md` |
| Known issues | `docs/FAILURE_LOG.md` |
| Technical details | `docs/arch/*.md` |
| C++ format docs | `docs/codedoc-*.md` and root `codedoc-*.md` |
| Gap analysis | `docs/GAPS_ANALYSIS_SUMMARY.md` |
| Agent protocol | `docs/agents/AGENT_PROTOCOL.md` |
| Agent roster | `docs/agents/AGENTS.md` |
| Risk tracking | `docs/RISK_REGISTER.md` |

---

*Last Index Update: 2026-02-20*
