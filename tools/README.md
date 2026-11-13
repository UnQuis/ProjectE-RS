# ProjectE-RS validation tools

The Python tools follow Horizon's repository pattern: focused validators in
tools/, regression tests in tests/, and one gate that composes the checks.
They validate this mod's own metadata and gameplay declarations; they do not
import Horizon implementation code.

## Commands

    python3 tools/gate.py --python-only
    python3 tools/gate.py

The first command runs static checks and Python tests in a plain clone. The
full gate additionally runs rustfmt, Clippy, cargo check and cargo test; it
requires the pinned Horizon checkout staged like the native launcher/CI.

| Tool | Responsibility |
| --- | --- |
| check_manifest.py | Native mod id/version, Horizon and mixins pins, Cargo API dependency, and mod-only CI |
| check_resources.py | Strict JSON parsing, duplicate-key rejection, resource paths, recipes and EMC Data Maps |
| check_gameplay.py | EMC table uniqueness/positivity, checked arithmetic, recipe patterns/catalysts, Klein Star capacities and machine tier ladders |
| gate.py | Runs validators and tests, then optionally the Rust quality checks |

Resource validation accepts data/ and assets/ at repository root or under
engine/. Empty resource roots are valid while content is still code-defined.
Gameplay checks for modules not implemented yet are skipped. No third-party
Python packages are required; Python 3.11+ is enough.
