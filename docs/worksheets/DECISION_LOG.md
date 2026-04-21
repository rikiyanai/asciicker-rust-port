# DECISION LOG

## Key Decisions - Architecture & Implementation

**Last Updated:** 2026-02-20

---

## ARCHITECTURE DECISIONS

| ID | Decision | Rationale | Date | Status |
|----|----------|-----------|------|--------|
| D001 | Use Bevy as engine foundation | Mage-core incomplete, Bevy has all features | 2026-02-19 | ✅ Final |
| D002 | Full rewrite (no FFI) | Cleaner long-term, avoid C++ complexity | 2026-02-19 | ✅ Final |
| D003 | Use custom ASCII rendering (not crate) | More control, can adapt Mage-core approach | 2026-02-19 | ✅ Final |
| D004 | Perspective REQUIRED | Q/E rotation, toggle feature need perspective | 2026-02-19 | ✅ Final |
| D005 | Implement perspective not isometric | Game features require perspective | 2026-02-19 | ✅ Final |

---

## INTEGRATION DECISIONS

| ID | Decision | Rationale | Date | Status |
|----|----------|-----------|------|--------|
| D010 | Keep auto_mat initially | Faster, proven, add k-d later | 2026-02-19 | ✅ Final |
| D011 | Hybrid approach for Alex Harri | Shape-matching + existing lighting | 2026-02-19 | ✅ Final |
| D012 | Shape-match within RESOLVE phase — k-d tree replaces auto_mat for glyph selection while preserving auto_mat color lookup | Aligns with Alex Harri design | 2026-02-19 | ✅ Final |

---

## IMPLEMENTATION DECISIONS

| ID | Decision | Rationale | Date | Status |
|----|----------|-----------|------|--------|
| D020 | Incremental strangler fig pattern | Reduces risk, allows testing | 2026-02-19 | ✅ Final |
| D021 | Document C++ bugs in Rust | Don't modify original, add validation | 2026-02-19 | ✅ Final |
| D022 | Use golden file testing | Visual regression essential | 2026-02-19 | ✅ Final |
| D023 | Use property-based testing | Algorithm correctness verification | 2026-02-19 | ✅ Final |

---

## DEFERRED DECISIONS

| ID | Decision | Deferred To | Reason |
|----|----------|-------------|--------|
| D030 | k-d tree vs auto_mat | After Phase 2 | Need performance data |
| D031 | Network implementation | After Phase 4 | Optional feature |
| D032 | Editor tools | After Phase 5 | Focus on runtime first |

---

## DECISIONS PENDING

| ID | Decision | Needed By | Notes |
|----|----------|-----------|-------|
| D040 | 2D vs 6D vectors | Before Milestone 5 (Integration) | Needs performance data from benchmarks |
| D041 | Ancestor Cleanup | Before Milestone 3.2 (BSP World) | Needs research — currently STUBBED in C++ |

---

## CHANGE LOG

| Date | Decision | Change | Reason |
|------|----------|--------|--------|
| 2026-02-19 | Perspective | Changed from "optional" to "required" | Q/E rotation needs it |
| 2026-02-19 | Engine | Changed from "Mage-core" to "Bevy" | Mage-core incomplete |
| 2026-02-19 | Architecture | Changed from "OOP" to "DOD" | C++ uses data-oriented |

---

*Decision log last updated: 2026-02-20*
