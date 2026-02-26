# Failure Log

Canonical append-only failure tracking.

### FL-001: Spatial resolution collapse in standard processor

**Status:** OPEN
**Date Opened:** 2026-02-18
**Category:** pipeline
**Description:** Standard processor uses 12px CP437 grid. Source cells at 32px map to only 4 glyphs. Catastrophic information loss.
**Root Cause:** Fixed 12x12 font atlas cannot represent detail from larger source cells.
**Evidence:**
- Commit 8401edb — initial investigation
- werewolf.png: 86% of pixels have quantization error >50
**Related:** FL-002, FL-003

### FL-002: ANSI palette has no brown

**Status:** PARTIAL
**Date Opened:** 2026-02-18
**Category:** pipeline
**Description:** 16-color ANSI palette lacks brown. Warm-toned sprites (werewolf, deer) suffer severe color distortion.
**Evidence:**
- Color analysis: nearest ANSI color for #8B4513 is red (#AA0000), delta=72

### FL-003: Cell alignment padding creates magenta borders

**Status:** MONITORING
**Date Opened:** 2026-02-18
**Category:** pipeline
**Description:** Non-12px cells padded to 12px multiples create visible magenta borders in output.
**Root Cause:** Ceiling division in slicer pads 32px to 36px, 50px to 60px.
**Evidence:**
- Commit cb0539e — diagnostic run confirmed padding artifacts
**Resolution:** Ceiling-division fix applied in commit 3a4906b
**Date Resolved:** 2026-02-18
