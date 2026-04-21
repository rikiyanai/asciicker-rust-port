# HANDOFF: Comprehensive Engine Architecture Map (Write-to-Disk Edition)

## Mission

Map the ENTIRE Asciicker C++ game engine — every function, every global, every data flow — into a single comprehensive architecture document. The output lives at `docs/worksheets/ENGINE_ARCHITECTURE.md`.

**This is a READ-ONLY research task. Do NOT modify any source files. Do NOT create new code. Do NOT "improve" anything.**

## CRITICAL: Write-to-Disk Strategy

**Previous attempt failed because 27 agents returned results into conversation context, hitting the limit and losing everything.**

**NEW RULE: Every Wave 1 agent MUST write its output to `docs/worksheets/arch/<filename>.md` using the Write tool.** The conversation context is only used for coordination, not for storing analysis results. If context dies, the files on disk survive.

Each agent writes to a specific file path. The assembly agent reads those files. No analysis data lives in conversation context.

---

## Anti-Slacking Rules (READ THESE FIRST)

These rules exist because AI agents consistently fail at thorough documentation tasks. You MUST follow them:

1. **NO SUMMARIES WITHOUT LINE RANGES.** Every function documented must include `filename:start_line-end_line`. If you write "handles rendering" without line numbers, you have failed.

2. **NO SKIPPING "BORING" FILES.** `water.cpp` (70 lines) gets the same treatment as `game.cpp` (11614 lines). Every file. Every function. No exceptions.

3. **NO INVENTED INFORMATION.** If you don't understand what a function does, write "PURPOSE  — needs manual review" with the exact lines. Do NOT guess. Do NOT hallucinate plausible-sounding explanations.

4. **NO EARLY COMPLETION CLAIMS.** You are not done until every file in the File Inventory has a section in the output. Count them. If the output doc has fewer sections than files listed below, you are not done.

5. **NO COMBINING SMALL FILES INTO "MISC".** Each file gets its own section header. Period.

6. **QUOTE ACTUAL CODE for non-obvious things.** When documenting a complex macro, a tricky bitfield, or a non-obvious data structure, include the actual code snippet (3-10 lines). Don't paraphrase it.

7. **FLAG CONFUSION.** If a function's purpose is ambiguous, if naming is inconsistent, if there's dead code — say so explicitly. Don't smooth it over.

8. **CROSS-REFERENCE CALLERS AND CALLEES.** For every public function, list who calls it and what it calls. Not "various places" — actual filenames.

---

## File Inventory (48 files, ~82,000 lines)

### TIER 1: MEGA files (>5000 lines) — 2-3 agents per file

| File | Lines | Split Strategy | Output Files |
|------|-------|----------------|--------------|
| `game.cpp` | 11614 | Agent A: lines 1-4000 (init, player, movement). Agent B: lines 4001-8000 (combat, AI, items). Agent C: lines 8001-11614 (UI, rendering calls, main loop) | `docs/worksheets/arch/game_cpp_part1.md`, `docs/worksheets/arch/game_cpp_part2.md`, `docs/worksheets/arch/game_cpp_part3.md` |
| `asciiid.cpp` | 11584 | Agent A: lines 1-4000 (init, terrain editing). Agent B: lines 4001-8000 (mesh editing, UI panels). Agent C: lines 8001-11584 (MCP commands, render loop, IO) | `docs/worksheets/arch/asciiid_cpp_part1.md`, `docs/worksheets/arch/asciiid_cpp_part2.md`, `docs/worksheets/arch/asciiid_cpp_part3.md` |
| `world.cpp` | 5832 | Agent A: lines 1-3000 (BSP, loading, mesh library). Agent B: lines 3001-5832 (instances, queries, serialization) | `docs/worksheets/arch/world_cpp_part1.md`, `docs/worksheets/arch/world_cpp_part2.md` |
| `stb_vorbis.cpp` | 5684 | 1 agent only. VENDORED code (Sean Barrett). Document only: public API functions, how asciicker calls it, any local modifications. Do NOT line-by-line document vendored internals. | `docs/worksheets/arch/stb_vorbis_cpp.md` |

### TIER 2: LARGE files (2000-5000 lines) — 1-2 agents per file

| File | Lines | Notes | Output File |
|------|-------|-------|-------------|
| `render.cpp` | 4579 | THE CORE RENDERER. 2 agents: A=rasterizer+shaders (first half), B=compositing+output (second half) | `docs/worksheets/arch/render_cpp_part1.md`, `docs/worksheets/arch/render_cpp_part2.md` |
| `game_app.cpp` | 3765 | 2 agents: A=app lifecycle+window (first half), B=event loop+platform (second half) | `docs/worksheets/arch/game_app_cpp_part1.md`, `docs/worksheets/arch/game_app_cpp_part2.md` |
| `terrain.cpp` | 3310 | 2 agents: A=quadtree+heightmap (first half), B=raycasting+editing (second half) | `docs/worksheets/arch/terrain_cpp_part1.md`, `docs/worksheets/arch/terrain_cpp_part2.md` |
| `mainmenu.cpp` | 2844 | 1 agent. Menu system, character creation, UI flow | `docs/worksheets/arch/mainmenu_cpp.md` |
| `x11.cpp` | 2373 | 1 agent. X11 platform backend. Document public interface + key event handling | `docs/worksheets/arch/x11_cpp.md` |
| `physics.cpp` | 2351 | 1 agent. Collision detection, gravity, movement physics | `docs/worksheets/arch/physics_cpp.md` |
| `gamepad.cpp` | 2318 | 1 agent. Controller input mapping, dead zones, button state | `docs/worksheets/arch/gamepad_cpp.md` |
| `sprite.cpp` | 2023 | 1 agent. XP sprite loading, palette quantization, atlas management | `docs/worksheets/arch/sprite_cpp.md` |
| `mswin.cpp` | 1942 | 1 agent. Windows platform backend | `docs/worksheets/arch/mswin_cpp.md` |

### TIER 3: MEDIUM files (500-2000 lines) — 1 agent per batch, write to batch file

| Batch | Files | Output File |
|-------|-------|-------------|
| Platform | `sdl.cpp` (1446) | `docs/worksheets/arch/batch_sdl.md` |
| Audio | `audio.cpp` (1373) + `audio.h` (123) | `docs/worksheets/arch/batch_audio.md` |
| Network | `game_svr.cpp` (1236) + `network.cpp` (979) + `network.h` (209) | `docs/worksheets/arch/batch_network.md` |
| Web | `game_web.cpp` (973) + `game_web.html` | `docs/worksheets/arch/batch_web.md` |
| Terminal | `term.cpp` (919) + `terminal.cpp` (440) + `term.h` (74) | `docs/worksheets/arch/batch_terminal.md` |
| Undo | `urdo.cpp` (897) + `urdo.h` (81) | `docs/worksheets/arch/batch_undo.md` |
| Inventory | `inventory.cpp` (759) + `inventory.h` (233) | `docs/worksheets/arch/batch_inventory.md` |
| API | `game_api.cpp` (685) + `game_api.h` (104) | `docs/worksheets/arch/batch_api.md` |
| GL | `imgui_impl_opengl3.cpp` (588) + `gl45_emu.cpp` (365) + `gl45_emu.h` (74) | `docs/worksheets/arch/batch_gl.md` |
| Color | `rgba8.cpp` (578) + `rgba8.h` (32) | `docs/worksheets/arch/batch_color.md` |

### TIER 4: SMALL files (<500 lines) — 1 agent per batch

| Batch | Files | Output File |
|-------|-------|-------------|
| Small-A | `weather.cpp` (449), `weather.h` (98), `font1.cpp` (388), `font1.h` (~), `sprite_validate.cpp` (340), `enemygen.cpp` (292), `enemygen.h` (70), `screen.cpp` (93) | `docs/worksheets/arch/batch_small_a.md` |
| Small-B | `texheap.cpp` (207), `texheap.h` (116), `water.cpp` (70), `world_patch.cpp` (59), `input.cpp` (72), `fast_rand.h` (16), `stdafx.h` (15) | `docs/worksheets/arch/batch_small_b.md` |

### TIER 5: Header-only architecture files — 1 agent

| Files | Output File |
|-------|-------------|
| `game.h` (567), `world.h` (219), `render.h` (137), `terrain.h` (218), `sprite.h` (111), `sprite_constants.h` (93), `matrix.h` (515), `platform.h` (488), `lexer.h` (1224), `physics.h` (122), `upng.h` (125) | `docs/worksheets/arch/batch_headers.md` |

---

## Total Output Files: 34 files in `docs/worksheets/arch/`

Orchestrator tracks progress by checking which files exist:
```bash
ls -la docs/worksheets/arch/*.md | wc -l   # should reach 34 after Wave 1
```

---

## Per-Agent Output Format (MANDATORY)

Every agent MUST produce output in EXACTLY this format for EVERY function in their assigned file range. The agent writes this content to its assigned output file using the Write tool.

Each output file MUST start with:
```markdown
# [filename] Analysis — Lines [start]-[end]
# Generated: [date]
# Agent: [description]
```

Then for every function:

```markdown
### `function_name` (filename:start_line-end_line)

**Signature:** `return_type function_name(param_type param_name, ...)`
**Purpose:** One sentence. What does this function DO, not what it IS.
**Called by:** list of `filename:function_name` callers (use grep, not guessing)
**Calls:** list of significant functions this calls
**Globals read:** list of global variables accessed
**Globals mutated:** list of global variables written to
**Side effects:** file I/O, network, memory allocation, OpenGL state changes
**Notes:** anything non-obvious, dead code, TODOs, bugs, confusion
```

For global variables and data structures at file scope:

```markdown
### Global: `variable_name` (filename:line)

**Type:** exact type declaration
**Purpose:** what it stores
**Initialized by:** which function sets it up
**Read by:** list of functions
**Written by:** list of functions
**Thread safety:** none / mutex / atomic / single-thread-only
```

For structs/enums/typedefs:

```markdown
### Struct: `TypeName` (filename:start_line-end_line)

**Fields:**
- `field_name` (type) — purpose
- ...
**Used by:** which files/functions use this type
**Size notes:** any packing, alignment, array sizing
```

---

## Execution Plan

### Wave 1: Parallel file analysis (launch ALL simultaneously)

Launch agents for each tier. Total: ~27 analysis agents. ALL run in background with `run_in_background: true`.

**CRITICAL AGENT INSTRUCTIONS — include verbatim in every agent prompt:**

```
YOU MUST WRITE YOUR OUTPUT TO THE FILE: [assigned output path]

Use the Write tool to save your complete analysis to the assigned file path.
Do NOT return your analysis as conversation text — write it to disk.

Read every line in your assigned range. Document every function, every global,
every struct, every enum, every macro, every typedef. If you skip something,
your output is invalid.

Use Grep to find callers across the entire codebase, not just your assigned file.

Do NOT summarize. Do NOT say "various helper functions handle X". Name each
function individually. If there are 47 functions, there must be 47 entries.

When you encounter something you don't understand, write ":" followed
by what confused you. Do NOT invent a plausible explanation.

When done, your final message should be: "WROTE [N] function entries, [M] global
entries, [K] struct entries to [output_path]"
```

### CHECKPOINT 1: Verify Wave 1 completion

After launching all Wave 1 agents, the orchestrator periodically checks:
```bash
ls docs/worksheets/arch/*.md | wc -l              # target: 34
wc -l docs/worksheets/arch/*.md                    # each file should be substantial
```

Do NOT proceed to Wave 2 until all 34 files exist and are non-trivial (>20 lines each).

If context is getting large at this point, run `/prepcompact` before Wave 2.

### Wave 2: Assembly (1 agent)

Launch 1 assembly agent that:
1. Reads all 34 files from `docs/worksheets/arch/` using the Read tool
2. Merges them into a single `docs/worksheets/ENGINE_ARCHITECTURE.md` using Write tool
3. Adds a Table of Contents
4. Adds a "Cross-Cutting Concerns" section covering:
   - Memory management patterns (malloc/free vs new/delete vs static)
   - Global state dependency graph (which globals depend on which)
   - Init order (what must be initialized before what)
   - The main loop structure (game_app.cpp -> game.cpp -> render.cpp flow)
   - Platform abstraction (what's behind platform.h)
   - Thread model (single-threaded? any threading?)
5. Adds a "Data Flow" section:
   - Input -> Game State -> Render pipeline
   - Network -> Game State sync
   - File loading -> World/Terrain/Sprite pipelines
   - Audio trigger points

**Assembly agent output file:** `docs/worksheets/ENGINE_ARCHITECTURE.md`

### CHECKPOINT 2: Verify assembly

```bash
wc -l docs/worksheets/ENGINE_ARCHITECTURE.md      # target: >3000 lines
grep -c "^### " docs/worksheets/ENGINE_ARCHITECTURE.md  # count function entries
```

### Wave 3: Triple verification (3 agents in parallel)

Launch 3 independent verifier agents. Each reads `docs/worksheets/ENGINE_ARCHITECTURE.md` + source files.

**Verifier A — Completeness Check** (output: `docs/worksheets/arch/verify_completeness.md`):
- For EVERY .cpp and .h file in the inventory, grep for function definitions
- Count functions found in source vs functions documented
- Report: "file.cpp: 47 functions in source, 45 documented, MISSING: func_at_line_123, func_at_line_456"
- FAIL if any file has <90% coverage

**Verifier B — Accuracy Spot-Check** (output: `docs/worksheets/arch/verify_accuracy.md`):
- Randomly select 20 documented functions across different files
- Read the actual source code for each
- Verify: does the documentation match what the code actually does?
- Report: "function_name: ACCURATE / INACCURATE (explanation)"
- FAIL if >3 inaccuracies found

**Verifier C — Cross-Reference Integrity** (output: `docs/worksheets/arch/verify_crossrefs.md`):
- Select 15 functions that claim "Called by: X, Y, Z"
- Grep the codebase to verify those callers actually exist
- Select 15 globals that claim "Written by: A, B"
- Grep to verify
- Report: "global_name claimed written by X — VERIFIED / NOT FOUND"
- FAIL if >5 cross-references are wrong

---

## Output Structure

Final `docs/worksheets/ENGINE_ARCHITECTURE.md` must have:

```
# Asciicker Engine Architecture (auto-generated YYYY-MM-DD)

## Table of Contents
## Methodology & Coverage Statistics
## Cross-Cutting Concerns
  ### Memory Management
  ### Global State Graph
  ### Init Order
  ### Main Loop
  ### Platform Abstraction
  ### Threading Model
## Data Flow
  ### Input Pipeline
  ### Render Pipeline
  ### Network Pipeline
  ### Audio Pipeline
  ### File Loading Pipeline

## File-by-File Reference
  ### game.cpp (11614 lines)
    #### Functions
    #### Globals
    #### Data Structures
  ### render.cpp (4579 lines)
    ...
  [one section per file, ordered by tier]

## Verification Report
  ### Completeness (Verifier A)
  ### Accuracy (Verifier B)
  ### Cross-References (Verifier C)
  ### Coverage: X/Y functions documented (Z%)
```

---

## Context Management Strategy

This audit generates massive output. To prevent context death:

1. **All analysis goes to disk** — agents write files, not conversation text
2. **Orchestrator only tracks file existence** — use `ls` and `wc -l`, don't read file contents
3. **Compact between waves** — run `/prepcompact` after Wave 1 checkpoint, before Wave 2
4. **Assembly agent reads from disk** — it reads `docs/worksheets/arch/*.md`, not conversation history
5. **Verifiers read from disk** — they read `docs/worksheets/ENGINE_ARCHITECTURE.md`, not conversation

If you see context getting large (>70% used), compact immediately. The work is safe on disk.

---

## What NOT To Do

- Do NOT modify any source files
- Do NOT add comments to source code
- Do NOT refactor or "clean up" anything
- Do NOT create helper scripts
- Do NOT rely on existing docs (CLAUDE.md, AGENTS.md, etc.) — read source directly
- Do NOT use file summaries from previous sessions
- Do NOT batch small functions as "utility functions include X, Y, Z" — document each one
- Do NOT skip vendored files that are MODIFIED (check git diff for local changes)
- Do NOT claim completion until verifiers pass
- Do NOT produce output shorter than 3000 lines total across all files
- Do NOT return analysis in conversation text — WRITE TO DISK
- Do NOT read agent output files into conversation context until assembly phase

---

## Quick Recovery Checklist

If context dies again mid-run:
1. `ls docs/worksheets/arch/*.md | wc -l` — see how many files were written
2. `wc -l docs/worksheets/arch/*.md` — see which are substantial vs stubs
3. Only re-run agents whose output files are missing or <20 lines
4. Proceed to Wave 2 once all 34 files are confirmed
