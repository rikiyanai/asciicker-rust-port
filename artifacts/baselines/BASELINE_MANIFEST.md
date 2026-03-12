# Regression Baseline Manifest

Canonical renderer regression baseline until manual sign-off:

- Commit snapshot: `3a621b818c05689a57835548fcdd3552dd3a6b56`
- Baseline capture directory: `artifacts/baselines/backup-3a621b8-run2`
- Baseline trace: `artifacts/baselines/backup-3a621b8-run2/trace.jsonl`
- Reference comparison artifact: `artifacts/comparisons/backup-3a621b8-vs-current-head-detailed.json`
- User-approved working orbit comparison capture: `artifacts/baselines/orbit-2026-03-11-current`
- First post-fallback-policy replay against that orbit trace: `artifacts/baselines/orbit-2026-03-11-postfallback-debug`
- Second post-fallback-policy replay against that orbit trace: `artifacts/baselines/orbit-2026-03-11-postfallback2-debug`
- Experimental adaptive-threshold replay against that orbit trace: `artifacts/baselines/orbit-2026-03-11-adaptive-threshold-debug`
- Stabilized low-chaos default replay against that orbit trace: `artifacts/baselines/orbit-2026-03-11-stabilized-debug`
- Semantic-gated replay against that orbit trace: `artifacts/baselines/orbit-2026-03-11-semantic-gated-debug`
- Post-audit replay with background-luma sampling and resolve-owned semantic eligibility: `artifacts/baselines/orbit-2026-03-11-post-audit-fixes-debug`
- Stitched three-mode variant replay target: `artifacts/baselines/orbit-2026-03-11-variant-three-mode`

Rules:

1. Do not replace this baseline automatically.
2. New renderer changes should be compared against this baseline until the user explicitly signs off on an improvement.
3. If a later baseline is captured, store it under a new directory and leave this one intact.
4. Any “improvement” claim should cite:
   - the comparison run used
   - the exact artifact path
   - the user sign-off point
5. The `orbit-2026-03-11-current` capture is the active working regression run for renderer occupancy/contrast tuning, but it does not replace the locked `3a621b8` canonical baseline.
6. The stitched three-mode variant replay is the new visual-comparison workflow for manual review: `original_only` -> `combined` -> `harri_priority` on the same trace with a capture-only bottom panel.
