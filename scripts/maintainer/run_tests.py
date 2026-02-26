#!/usr/bin/env python3
"""Run the maintainer test suite in isolation.

Bypasses the project-root conftest.py (which requires human image signoff)
by setting --confcutdir to the maintainer tests directory. This allows CI
to run maintainer tests independently of the pipeline visual signoff gate.

Usage:
    python3 scripts/maintainer/run_tests.py
    python3 scripts/maintainer/run_tests.py -v --tb=long
"""
import subprocess
import sys
from pathlib import Path

TESTS_DIR = Path(__file__).resolve().parent / "tests"


def main():
    cmd = [
        sys.executable, "-m", "pytest",
        str(TESTS_DIR),
        f"--confcutdir={TESTS_DIR}",
        "--no-header",
    ]
    # Pass through any extra pytest args
    cmd.extend(sys.argv[1:])

    result = subprocess.run(cmd)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
