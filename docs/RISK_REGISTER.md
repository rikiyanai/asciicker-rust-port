# RISK REGISTER

## Project Risks - Tracked and Mitigated

**Last Updated:** 2026-02-20

---

## ACTIVE RISKS

| ID | Risk | Likelihood | Impact | Mitigation | Status |
|----|------|------------|--------|------------|--------|
| R001 | Rendering output doesn't match C++ | Medium | High | Golden file tests | 🔄 Planned |
| R002 | Performance below 60fps | Medium | High | Early benchmarking | 🔄 Planned |
| R004 | Bevy API changes | Low | Medium | Pin versions | 🔄 Planned |
| R005 | Platform-specific issues | Medium | Medium | Test all platforms | 🔄 Planned |

---

## RESOLVED RISKS

| ID | Risk | Resolution | Date |
|----|------|------------|------|
| R003 | Missing file format data | Research complete — all formats documented | 2026-02-20 |
| R006 | Unknown architecture type | Corrected: DOD not OOP | 2026-02-19 |
| R007 | Perspective implementation | Must implement | 2026-02-19 |
| R008 | Mage-core incomplete | Using Bevy instead | 2026-02-19 |
| R009 | Missing constants | Extracted from C++ | 2026-02-20 |

---

## RISK CATEGORIES

| Category | Count | Active |
|----------|-------|--------|
| Rendering | 1 | 1 |
| Performance | 1 | 1 |
| Data | 1 | 0 |
| Platform | 1 | 1 |
| Dependencies | 1 | 1 |

---

## MONITORING PLAN

| Risk | Trigger | Action |
|------|---------|--------|
| Rendering mismatch | Golden file >1% diff | Adjust algorithm |
| Performance <60fps | Profile shows bottleneck | Optimize hot path |
| API changes | Bevy version bump | Review changes |

---

*Risk register last updated: 2026-02-20*
