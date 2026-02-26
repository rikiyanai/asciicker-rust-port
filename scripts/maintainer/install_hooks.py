#!/usr/bin/env python3
"""Hook installer — generate and verify maintainer hook configurations.

Outputs hook JSON snippets matching the actual ~/.claude/settings.json
nested format (matcher → hooks array). Can also verify installation.

Usage:
    python3 scripts/maintainer/install_hooks.py              # Print hook configs
    python3 scripts/maintainer/install_hooks.py --verify      # Check if hooks are installed
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
_HOOKS_DIR = _PROJECT_ROOT / "scripts" / "maintainer" / "hooks"

# Hook definitions — actual wrapper scripts, not CLI tools
HOOKS = {
    "session_start": {
        "hook_type": "SessionStart",
        "matcher": None,
        "script": str(_HOOKS_DIR / "session_start_hook.py"),
        "status_message": "Maintainer: start-of-session checks...",
        "description": "Run start-of-session protocol (conductor + hooks + maintainer tests)",
        "verify_fragment": "session_start_hook",
    },
    "claim_guard": {
        "hook_type": "PreToolUse",
        "matcher": "Bash",
        "script": str(_HOOKS_DIR / "claim_guard_hook.py"),
        "status_message": "Maintainer: claim guard...",
        "description": "Warn on unsupported claims in git commit messages",
        "verify_fragment": "claim_guard_hook",
    },
    "claim_guard_content_write": {
        "hook_type": "PreToolUse",
        "matcher": "Write",
        "script": str(_HOOKS_DIR / "claim_guard_content_hook.py"),
        "status_message": "Maintainer: claim guard content...",
        "description": "Block unsupported status claims in written file content",
        "verify_fragment": "claim_guard_content_hook",
    },
    "claim_guard_content_edit": {
        "hook_type": "PreToolUse",
        "matcher": "Edit",
        "script": str(_HOOKS_DIR / "claim_guard_content_hook.py"),
        "status_message": "Maintainer: claim guard content...",
        "description": "Block unsupported status claims in edited file content",
        "verify_fragment": "claim_guard_content_hook",
    },
    "claim_guard_content_multiedit": {
        "hook_type": "PreToolUse",
        "matcher": "MultiEdit",
        "script": str(_HOOKS_DIR / "claim_guard_content_hook.py"),
        "status_message": "Maintainer: claim guard content...",
        "description": "Block unsupported status claims in multi-edit file content",
        "verify_fragment": "claim_guard_content_hook",
    },
    "command_handoff_bash": {
        "hook_type": "PreToolUse",
        "matcher": "Bash",
        "script": str(_HOOKS_DIR / "command_handoff_hook.py"),
        "status_message": "Maintainer: command handoff...",
        "description": "Write per-command handoff artifact for shell commands",
        "verify_fragment": "command_handoff_hook",
    },
    "command_handoff_write": {
        "hook_type": "PreToolUse",
        "matcher": "Write",
        "script": str(_HOOKS_DIR / "command_handoff_hook.py"),
        "status_message": "Maintainer: command handoff...",
        "description": "Write per-command handoff artifact for file writes",
        "verify_fragment": "command_handoff_hook",
    },
    "command_handoff_edit": {
        "hook_type": "PreToolUse",
        "matcher": "Edit",
        "script": str(_HOOKS_DIR / "command_handoff_hook.py"),
        "status_message": "Maintainer: command handoff...",
        "description": "Write per-command handoff artifact for file edits",
        "verify_fragment": "command_handoff_hook",
    },
    "command_handoff_multiedit": {
        "hook_type": "PreToolUse",
        "matcher": "MultiEdit",
        "script": str(_HOOKS_DIR / "command_handoff_hook.py"),
        "status_message": "Maintainer: command handoff...",
        "description": "Write per-command handoff artifact for multi-edits",
        "verify_fragment": "command_handoff_hook",
    },
    "command_handoff_read": {
        "hook_type": "PreToolUse",
        "matcher": "Read",
        "script": str(_HOOKS_DIR / "command_handoff_hook.py"),
        "status_message": "Maintainer: command handoff...",
        "description": "Write per-command handoff artifact for reads",
        "verify_fragment": "command_handoff_hook",
    },
    "command_handoff_grep": {
        "hook_type": "PreToolUse",
        "matcher": "Grep",
        "script": str(_HOOKS_DIR / "command_handoff_hook.py"),
        "status_message": "Maintainer: command handoff...",
        "description": "Write per-command handoff artifact for grep searches",
        "verify_fragment": "command_handoff_hook",
    },
    "command_handoff_glob": {
        "hook_type": "PreToolUse",
        "matcher": "Glob",
        "script": str(_HOOKS_DIR / "command_handoff_hook.py"),
        "status_message": "Maintainer: command handoff...",
        "description": "Write per-command handoff artifact for glob searches",
        "verify_fragment": "command_handoff_hook",
    },
    "command_handoff_task": {
        "hook_type": "PreToolUse",
        "matcher": "Task",
        "script": str(_HOOKS_DIR / "command_handoff_hook.py"),
        "status_message": "Maintainer: command handoff...",
        "description": "Write per-command handoff artifact for delegated tasks",
        "verify_fragment": "command_handoff_hook",
    },
    "startup_gate_bash": {
        "hook_type": "PreToolUse",
        "matcher": "Bash",
        "script": str(_HOOKS_DIR / "startup_gate_hook.py"),
        "status_message": "Maintainer: startup gate...",
        "description": "Enforce start-of-session checks before running commands",
        "verify_fragment": "startup_gate_hook",
    },
    "startup_gate_write": {
        "hook_type": "PreToolUse",
        "matcher": "Write",
        "script": str(_HOOKS_DIR / "startup_gate_hook.py"),
        "status_message": "Maintainer: startup gate...",
        "description": "Enforce start-of-session checks before file writes",
        "verify_fragment": "startup_gate_hook",
    },
    "startup_gate_edit": {
        "hook_type": "PreToolUse",
        "matcher": "Edit",
        "script": str(_HOOKS_DIR / "startup_gate_hook.py"),
        "status_message": "Maintainer: startup gate...",
        "description": "Enforce start-of-session checks before file edits",
        "verify_fragment": "startup_gate_hook",
    },
    "startup_gate_multiedit": {
        "hook_type": "PreToolUse",
        "matcher": "MultiEdit",
        "script": str(_HOOKS_DIR / "startup_gate_hook.py"),
        "status_message": "Maintainer: startup gate...",
        "description": "Enforce start-of-session checks before multi-edits",
        "verify_fragment": "startup_gate_hook",
    },
    "startup_gate_read": {
        "hook_type": "PreToolUse",
        "matcher": "Read",
        "script": str(_HOOKS_DIR / "startup_gate_hook.py"),
        "status_message": "Maintainer: startup gate...",
        "description": "Enforce start-of-session checks before reads",
        "verify_fragment": "startup_gate_hook",
    },
    "startup_gate_grep": {
        "hook_type": "PreToolUse",
        "matcher": "Grep",
        "script": str(_HOOKS_DIR / "startup_gate_hook.py"),
        "status_message": "Maintainer: startup gate...",
        "description": "Enforce start-of-session checks before grep searches",
        "verify_fragment": "startup_gate_hook",
    },
    "startup_gate_glob": {
        "hook_type": "PreToolUse",
        "matcher": "Glob",
        "script": str(_HOOKS_DIR / "startup_gate_hook.py"),
        "status_message": "Maintainer: startup gate...",
        "description": "Enforce start-of-session checks before glob searches",
        "verify_fragment": "startup_gate_hook",
    },
    "startup_gate_task": {
        "hook_type": "PreToolUse",
        "matcher": "Task",
        "script": str(_HOOKS_DIR / "startup_gate_hook.py"),
        "status_message": "Maintainer: startup gate...",
        "description": "Enforce start-of-session checks before delegated tasks",
        "verify_fragment": "startup_gate_hook",
    },
    "claim_guard_transcript_write": {
        "hook_type": "PreToolUse",
        "matcher": "Write",
        "script": str(_HOOKS_DIR / "claim_guard_transcript_hook.py"),
        "status_message": "Maintainer: claim guard (transcript)...",
        "description": "Block tool use when recent assistant output has unsupported claims",
        "verify_fragment": "claim_guard_transcript_hook",
    },
    "claim_guard_transcript_edit": {
        "hook_type": "PreToolUse",
        "matcher": "Edit",
        "script": str(_HOOKS_DIR / "claim_guard_transcript_hook.py"),
        "status_message": "Maintainer: claim guard (transcript)...",
        "description": "Block tool use when recent assistant output has unsupported claims",
        "verify_fragment": "claim_guard_transcript_hook",
    },
    "claim_guard_transcript_multiedit": {
        "hook_type": "PreToolUse",
        "matcher": "MultiEdit",
        "script": str(_HOOKS_DIR / "claim_guard_transcript_hook.py"),
        "status_message": "Maintainer: claim guard (transcript)...",
        "description": "Block tool use when recent assistant output has unsupported claims",
        "verify_fragment": "claim_guard_transcript_hook",
    },
    "claim_guard_transcript_bash": {
        "hook_type": "PreToolUse",
        "matcher": "Bash",
        "script": str(_HOOKS_DIR / "claim_guard_transcript_hook.py"),
        "status_message": "Maintainer: claim guard (transcript)...",
        "description": "Block tool use when recent assistant output has unsupported claims",
        "verify_fragment": "claim_guard_transcript_hook",
    },
    "claim_guard_transcript_read": {
        "hook_type": "PreToolUse",
        "matcher": "Read",
        "script": str(_HOOKS_DIR / "claim_guard_transcript_hook.py"),
        "status_message": "Maintainer: claim guard (transcript)...",
        "description": "Block tool use when recent assistant output has unsupported claims",
        "verify_fragment": "claim_guard_transcript_hook",
    },
    "claim_guard_transcript_grep": {
        "hook_type": "PreToolUse",
        "matcher": "Grep",
        "script": str(_HOOKS_DIR / "claim_guard_transcript_hook.py"),
        "status_message": "Maintainer: claim guard (transcript)...",
        "description": "Block tool use when recent assistant output has unsupported claims",
        "verify_fragment": "claim_guard_transcript_hook",
    },
    "claim_guard_transcript_glob": {
        "hook_type": "PreToolUse",
        "matcher": "Glob",
        "script": str(_HOOKS_DIR / "claim_guard_transcript_hook.py"),
        "status_message": "Maintainer: claim guard (transcript)...",
        "description": "Block tool use when recent assistant output has unsupported claims",
        "verify_fragment": "claim_guard_transcript_hook",
    },
    "claim_guard_transcript_task": {
        "hook_type": "PreToolUse",
        "matcher": "Task",
        "script": str(_HOOKS_DIR / "claim_guard_transcript_hook.py"),
        "status_message": "Maintainer: claim guard (transcript)...",
        "description": "Block tool use when recent assistant output has unsupported claims",
        "verify_fragment": "claim_guard_transcript_hook",
    },
    "session_end": {
        "hook_type": "Stop",
        "matcher": None,
        "script": str(_HOOKS_DIR / "session_end_hook.py"),
        "status_message": "Maintainer: session-end scan...",
        "description": "Run janitor + audit + goal sanity at session end",
        "verify_fragment": "session_end_hook",
    },
    "skill_gate": {
        "hook_type": "PreToolUse",
        "matcher": "Bash",
        "script": str(_HOOKS_DIR / "skill_gate_hook.py"),
        "status_message": "Skill gate: checking required skills...",
        "description": "Block execute-phase if required skills are missing from .agents/skills/",
        "verify_fragment": "skill_gate_hook",
    },
}


def print_hook_configs():
    """Print hook configurations as JSON snippets for manual installation."""
    print("# Maintainer Hook Configurations")
    print("# Add these entries to the appropriate arrays in ~/.claude/settings.json\n")

    for name, hook in HOOKS.items():
        print(f"## {name}")
        print(f"# {hook['description']}")
        print(f"# Hook type: {hook['hook_type']}", end="")
        if hook["matcher"]:
            print(f", matcher: {hook['matcher']}")
        else:
            print()

        entry = {
            "type": "command",
            "command": f"python3 {hook['script']}",
            "statusMessage": hook["status_message"],
        }

        if hook["matcher"]:
            print(f"# Add to hooks.{hook['hook_type']} -> matcher: {hook['matcher']} -> hooks array:")
        else:
            print(f"# Add to hooks.{hook['hook_type']} -> hooks array:")

        print(json.dumps(entry, indent=2))
        print()


def verify_hooks():
    """Check if hooks are present in ~/.claude/settings.json.

    Handles the nested format:
      hooks.PreToolUse[].hooks[].command  (with matcher)
      hooks.Stop[].hooks[].command        (no matcher)
    """
    settings_path = Path.home() / ".claude" / "settings.json"
    if not settings_path.exists():
        print("~/.claude/settings.json not found")
        return False

    try:
        settings = json.loads(settings_path.read_text())
    except (json.JSONDecodeError, OSError) as e:
        print(f"Could not read settings: {e}")
        return False

    hooks_config = settings.get("hooks", {})
    found = 0
    total = len(HOOKS)

    for name, hook_def in HOOKS.items():
        hook_type = hook_def["hook_type"]
        matcher = hook_def["matcher"]
        fragment = hook_def["verify_fragment"]
        type_list = hooks_config.get(hook_type, [])

        is_installed = False
        for group in type_list:
            if not isinstance(group, dict):
                continue
            group_matcher = group.get("matcher")
            if matcher and group_matcher != matcher:
                continue
            # Check nested hooks array
            inner_hooks = group.get("hooks", [])
            for h in inner_hooks:
                if isinstance(h, dict) and fragment in h.get("command", ""):
                    is_installed = True
                    break
            if is_installed:
                break

        status = "installed" if is_installed else "MISSING"
        print(f"  [{status}] {name} ({hook_type})")
        if is_installed:
            found += 1

    print(f"\n{found}/{total} hooks installed")
    return found == total


def apply_hooks() -> bool:
    """Install missing maintainer hooks into ~/.claude/settings.json."""
    settings_path = Path.home() / ".claude" / "settings.json"
    settings_path.parent.mkdir(parents=True, exist_ok=True)

    if settings_path.exists():
        try:
            settings = json.loads(settings_path.read_text())
        except (json.JSONDecodeError, OSError) as e:
            print(f"Could not read settings: {e}")
            return False
        if not isinstance(settings, dict):
            settings = {}
    else:
        settings = {}

    hooks_config = settings.setdefault("hooks", {})
    if not isinstance(hooks_config, dict):
        hooks_config = {}
        settings["hooks"] = hooks_config

    added = 0
    for hook_def in HOOKS.values():
        hook_type = hook_def["hook_type"]
        matcher = hook_def["matcher"]
        fragment = hook_def["verify_fragment"]
        type_list = hooks_config.setdefault(hook_type, [])
        if not isinstance(type_list, list):
            type_list = []
            hooks_config[hook_type] = type_list

        group = None
        for candidate in type_list:
            if not isinstance(candidate, dict):
                continue
            group_matcher = candidate.get("matcher")
            if matcher is None:
                if group_matcher in (None, ""):
                    group = candidate
                    break
            elif group_matcher == matcher:
                group = candidate
                break

        if group is None:
            group = {"hooks": []}
            if matcher is not None:
                group["matcher"] = matcher
            type_list.append(group)

        inner_hooks = group.get("hooks")
        if not isinstance(inner_hooks, list):
            inner_hooks = []
            group["hooks"] = inner_hooks

        already_installed = False
        for inner in inner_hooks:
            if isinstance(inner, dict) and fragment in str(inner.get("command", "")):
                already_installed = True
                break
        if already_installed:
            continue

        inner_hooks.append({
            "type": "command",
            "command": f"python3 {hook_def['script']}",
            "statusMessage": hook_def["status_message"],
        })
        added += 1

    settings_path.write_text(json.dumps(settings, indent=2) + "\n")
    print(f"Applied {added} hook(s) into {settings_path}")
    return verify_hooks()


def main():
    parser = argparse.ArgumentParser(
        description="Hook installer for maintainer tools"
    )
    parser.add_argument(
        "--apply", action="store_true",
        help="Install any missing maintainer hooks into ~/.claude/settings.json"
    )
    parser.add_argument(
        "--verify", action="store_true",
        help="Check if hooks are installed in ~/.claude/settings.json"
    )
    args = parser.parse_args()

    if args.apply:
        ok = apply_hooks()
        sys.exit(0 if ok else 1)
    elif args.verify:
        ok = verify_hooks()
        sys.exit(0 if ok else 1)
    else:
        print_hook_configs()


if __name__ == "__main__":
    main()
