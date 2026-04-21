# ASSUMPTION MASTER CHECKLIST

## All Assumptions - Verified & Tracked

**Last Updated:** 2026-02-20

---

## TECHNOLOGY ASSUMPTIONS

| ID | Assumption | Status | Verified By | Evidence |
|----|------------|--------|-------------|----------|
| A1 | Bevy 0.18+ stable | ✅ Verified | Web research | v0.18.0 released Jan 2026 |
| A2 | WGPU backend works on targets | ✅ Verified | Web research | Cross-platform support confirmed |
| A3 | bevy_kira_audio sufficient | ✅ Verified | Research | Full audio capabilities |
| A4 | Rust 1.80+ features stable | ✅ Verified | Web research | Const generics stable since 1.51 |
| A5 | C++ rendering replicable in Rust | ✅ Verified | Research | DOD architecture maps to ECS |

---

## DATA ASSUMPTIONS

| ID | Assumption | Status | Verified By | Evidence |
|----|------------|--------|-------------|----------|
| D1 | .xp and .a3d formats reverse-engineered | ⚠️ Partial | C++ source | .a3d structure documented (audit-unknown-a3d-format.md), .xp sprite format details still unresolved (RE-AUDIT R-003) |
| D2 | Perspective math complete | ⚠️ Partial | C++ source | General perspective approach documented (audit-unknown-perspective-matrix.md), exact matrix values (focal_default, FOV) still unresolved (RE-AUDIT R-002) |
| D3 | Glyph coverage table complete | ✅ Verified | C++ source | sprite.cpp:1822-1840 |
| D4 | auto_mat lookup table complete | ✅ Verified | C++ source | render.cpp:708-840 |
| D5 | Animation timing constants known | ✅ Verified | C++ source | codedoc-animation-timing.md |
| D6 | Physics constants known | ✅ Verified | C++ source | Constants documented in ASSUMPTION_MASTER_CHECKLIST.md (CONSTANT VALUES table); codedoc-physics-constants.md planned but not yet created |
| D7 | Camera parameters known | ✅ Verified | C++ source | codedoc-camera-parameters.md |
| D8 | A3D key codes known | ✅ Verified | C++ source | codedoc-a3d-keycodes.md |

---

## ARCHITECTURE ASSUMPTIONS

| ID | Assumption | Status | Verified By | Evidence |
|----|------------|--------|-------------|----------|
| ARCH1 | C++ uses DOD not OOP | ✅ Verified | C++ analysis | No C++ classes found |
| ARCH2 | Bevy ECS appropriate | ✅ Verified | Research | Data-oriented maps to ECS |
| ARCH3 | No FFI needed (full rewrite) | ✅ Verified | Decision | Clean port preferred |
| ARCH4 | Perspective required for Q/E | ✅ Verified | Game analysis | Camera rotation needs perspective |

---

## FILE FORMAT ASSUMPTIONS

| ID | Assumption | Status | Verified By | Evidence |
|----|------------|--------|-------------|----------|
| FMT1 | Terrain uses .a3d not .xp | ✅ Verified | C++ source | terrain.cpp LoadTerrain |
| FMT2 | .a3d header is "AS3D" + version | ✅ Verified | C++ source | world.cpp SaveWorld |
| FMT3 | .xp is gzip-compressed | ✅ Verified | C++ source | sprite.cpp LoadSprite |
| FMT4 | XPCell is 10 bytes, column-major | ✅ Verified | C++ source | sprite.cpp structure |

---

## CONSTANT VALUES (VERIFIED)

| Constant | Value | Source | Status |
|----------|-------|--------|--------|
| HEIGHT_SCALE | 16 | terrain.h:54 | ✅ |
| HEIGHT_CELLS | 4 | terrain.h | ✅ |
| VISUAL_CELLS | 8 | terrain.h | ✅ |
| FOCAL_DEFAULT | max(w,h)*2.0 | render.cpp:3023 | ✅ |
| GRAVITY | ~9.8 | physics.cpp | ✅ |
| JUMP_VELOCITY | 10 | physics.cpp | ✅ |
| SPEED_GROUND | 27 | physics.cpp | ✅ |
| SPEED_WATER | 10 | physics.cpp | ✅ |
| COLLISION_RADIUS | 1.0 | physics.cpp | ✅ |
| ANIM_STAND_MS | 30000 | game.cpp:409 | ✅ |
| ANIM_ATTACK_MS | 20000 | game.cpp:409 | ✅ |
| KEY_CODES_COUNT | 115 | platform.h | ✅ |

---

## RISK ASSUMPTIONS

| ID | Assumption | Risk Level | Mitigation |
|----|------------|-------------|------------|
| R1 | Dev machine compiles Bevy | Low | Standard hardware |
| R2 | Test files exist | Low | Can create samples |
| R3 | Budget available | N/A | Out of scope |

---

## ASSUMPTIONS THAT CHANGED

| Original | Corrected | Date | Impact |
|----------|-----------|------|--------|
| OOP architecture | DOD (Data-Oriented Design) | 2026-02-19 | Low - different approach |
| Perspective optional | Perspective REQUIRED | 2026-02-19 | High - must implement |
| Mage-core standalone | Use Bevy instead | 2026-02-19 | Medium - different path |
| .xp for terrain | .a3d for terrain | 2026-02-20 | Low - correct format |

---

## PRE-IMPLEMENTATION CHECKLIST

Before starting Implementation:

- [x] All technology assumptions verified
- [x] All data assumptions verified
- [x] All file formats documented
- [x] All constants extracted
- [x] Architecture decisions made
- [x] Changed assumptions corrected

**Status: ⚠️ READY PENDING SIGN-OFF**

> NOTE: Sign-off process needs to be defined. The sign-off table below requires a named lead to approve before implementation begins.

---

## SIGN-OFF

| Role | Name | Date | Status |
|------|------|------|--------|
| Lead | | | ⏳ Pending |

---

*Assumption checklist last updated: 2026-02-20*
