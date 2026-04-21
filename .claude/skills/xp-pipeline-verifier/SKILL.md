---
name: xp-pipeline-verifier
description: "Use when validating whether the XP asset loading pipeline is actually complete for real gameplay outcomes; enforces that .xp sprites load correctly and render in-game."
---

# Skill: XP Pipeline Verifier

Verification runbook for the question: "Can the Rust port load and render XP sprites correctly?"

## Non-Negotiable Gates

1. Native-source gate:
- Original Asciicker `.xp` sprites load with correct metadata (angles, frames, projs, layers).
- Sprites render correctly in the Bevy-based ASCII renderer.

2. Format fidelity gate:
- XP binary format parsed identically to C++ `LoadSprite()`.
- CP437 glyphs, fg/bk colors match C++ output.
- Layer semantics preserved (colorkey=0, height=1, visual=2+).

Both gates must pass. Partial pass is not completion.

## Required Evidence

> **Note:** `.planning/` directory will be created by GSD initialization. Until then, `MASTER_ROADMAP.md` at project root serves as the status authority.

Collect and store evidence under `docs/worksheets/verification/` (directory exists in the project):
- `<date>-xp-loading-verification.md`
- `<date>-render-comparison.md`

> **Note:** Full verification requires actual XP sprite files and a working Rust renderer (Phase 2+). Before Phase 2, this skill is limited to documenting the verification plan and confirming the XP format parser compiles.

Each claim must include:
- input file name/path
- exact command run (`cargo test`, `cargo run --example`)
- output artifact or screenshot
- metadata summary (angles, frames, projs, layer count)
- visual comparison against C++ reference output

## Minimum Verification Loop

1. Preflight
```bash
git status --short
# Skip cargo commands if no Cargo.toml exists (pre-Phase 1)
test -f Cargo.toml && cargo build || echo "No Cargo.toml — skip build (pre-implementation)"
test -f Cargo.toml && cargo test || echo "No Cargo.toml — skip test (pre-implementation)"
```

2. XP loading verification
- Load at least 3 different .xp sprites (varying complexity).
- Validate metadata correctness against C++ reference.
- Verify glyph and color data match.

3. Render comparison
- Render loaded sprites through the ASCII pipeline.
- Compare output against C++ engine screenshots.

4. Verdict
- `PASS` only if both gates pass with reproducible evidence.
- Otherwise `NOT DONE` with blockers listed.

## Guardrails

- Do not treat passing unit tests as proof of visual correctness.
- Do not mark completion without explicit metadata + visual evidence.
- Compare against C++ reference output, not just internal consistency.
