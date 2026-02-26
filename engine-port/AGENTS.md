# Agent Entry Point
Follow this repository rule before doing any task.

<!-- codex-conductor:start -->
## Conductor Guardrail
Always run `conductor:status` first.

- Command alias: `conductor_status`
- Direct command: `python3 scripts/conductor_tools.py status --auto-setup`
- Behavior: if Conductor is missing, status runs setup and creates the baseline.
<!-- codex-conductor:end -->
