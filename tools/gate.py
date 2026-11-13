#!/usr/bin/env python3
"""ProjectE-RS local quality gate."""
import argparse
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_CHECKS = (
    ("manifest", "tools/check_manifest.py"),
    ("resources", "tools/check_resources.py"),
    ("gameplay", "tools/check_gameplay.py"),
)


def run_step(name, command):
    started = time.monotonic()
    print(f"[{name}] {' '.join(map(str, command))}", flush=True)
    result = subprocess.run(command, cwd=ROOT)
    elapsed = time.monotonic() - started
    print(f"[{name}] {'PASS' if result.returncode == 0 else 'FAIL'} ({elapsed:.1f}s)", flush=True)
    return result.returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python-only", action="store_true", help="skip Cargo; run Python validators and tests")
    args = parser.parse_args()
    steps = [(name, [sys.executable, script]) for name, script in PYTHON_CHECKS]
    steps.append(("python tests", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-v"]))
    if not args.python_only:
        steps.extend([
            ("rustfmt", ["cargo", "fmt", "--manifest-path", "engine/Cargo.toml", "--", "--check"]),
            ("clippy", ["cargo", "clippy", "--manifest-path", "engine/Cargo.toml", "--all-targets", "--", "-D", "warnings"]),
            ("cargo check", ["cargo", "check", "--manifest-path", "engine/Cargo.toml", "--all-targets"]),
            ("cargo test", ["cargo", "test", "--manifest-path", "engine/Cargo.toml", "--all-targets"]),
        ])
    for name, command in steps:
        if run_step(name, command):
            return 1
    print("ProjectE-RS quality gate passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
