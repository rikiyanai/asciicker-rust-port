#!/usr/bin/env python3
"""PreToolUse:Bash hook — blocks gsd execute-phase unless required skills exist.

Reads `required_skills:` line from the phase PLAN.md and verifies each
skill has a SKILL.md in .agents/skills/ (GSD executor discovery path).

Exit 0 = allow, exit 2 = block with message.
"""
import json
import os
import re
import sys
from pathlib import Path


def main():
    # Hook receives tool input as JSON on stdin
    try:
        hook_input = json.load(sys.stdin)
    except (json.JSONDecodeError, EOFError):
        sys.exit(0)  # Can't parse, don't block

    # Only intercept gsd execute-phase commands
    command = hook_input.get("command", "") if isinstance(hook_input, dict) else ""
    if "execute-phase" not in command and "gsd-tools" not in command:
        sys.exit(0)

    # Extract phase number from command
    phase_match = re.search(r'execute-phase\s+"?([0-9.]+)"?', command)
    if not phase_match:
        sys.exit(0)  # Not a phase execution command

    phase_num = phase_match.group(1)

    # Find phase directory
    project_root = Path(os.environ.get("PROJECT_ROOT", "."))
    if not project_root.exists():
        project_root = Path(".")

    phases_dir = project_root / ".planning" / "phases"
    phase_dir = None
    if phases_dir.exists():
        for d in phases_dir.iterdir():
            if d.is_dir() and d.name.startswith(phase_num):
                phase_dir = d
                break

    if phase_dir is None:
        sys.exit(0)  # Phase not found, let the main tool handle error

    # Read PLAN.md and extract required_skills
    plan_path = phase_dir / "PLAN.md"
    if not plan_path.exists():
        sys.exit(0)

    plan_text = plan_path.read_text(encoding="utf-8")
    skills_match = re.search(r'^required_skills:\s*(.+)$', plan_text, re.MULTILINE)
    if not skills_match:
        sys.exit(0)  # No required_skills field, allow

    required = [s.strip() for s in skills_match.group(1).split(",") if s.strip()]
    if not required:
        sys.exit(0)

    # Check that each skill exists in .agents/skills/
    skills_dir = project_root / ".agents" / "skills"
    missing = []
    for skill in required:
        skill_path = skills_dir / skill / "SKILL.md"
        if not skill_path.exists():
            missing.append(skill)

    if missing:
        print(
            f"BLOCKED: Phase {phase_num} requires skills not found in .agents/skills/: "
            f"{', '.join(missing)}. "
            f"Create .agents/skills/<skill>/SKILL.md for each.",
            file=sys.stderr,
        )
        sys.exit(2)

    # Check maintainer hooks are installed
    hooks_script = project_root / "scripts" / "maintainer" / "install_hooks.py"
    if hooks_script.exists():
        import subprocess
        result = subprocess.run(
            [sys.executable, str(hooks_script), "--verify"],
            capture_output=True, text=True, timeout=10,
        )
        if result.returncode != 0:
            print(
                f"WARNING: Maintainer hooks verification failed. "
                f"Run: python3 scripts/maintainer/install_hooks.py --verify",
                file=sys.stderr,
            )
            # Warn only, don't block on hook verification

    # All checks passed — print reminder about skill invocation
    print(
        f"Skill gate: Phase {phase_num} requires [{', '.join(required)}]. "
        f"Ensure these are invoked via Skill tool before execution.",
        file=sys.stderr,
    )
    sys.exit(0)


if __name__ == "__main__":
    main()
