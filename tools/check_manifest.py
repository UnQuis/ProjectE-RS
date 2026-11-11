#!/usr/bin/env python3
"""Validate mod metadata, Cargo workspace wiring and the pinned Horizon revision."""
import argparse
import re
import tomllib
from pathlib import Path

REVISION = re.compile(r"^[0-9a-f]{40}$")
MOD_ID = re.compile(r"^[a-z0-9_.-]+$")
VERSION = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")
WORKFLOW = ".github/workflows/rust.yml"


def validate(root: Path) -> list[str]:
    errors: list[str] = []

    def load(path: Path):
        try:
            return tomllib.loads(path.read_text(encoding="utf-8"))
        except FileNotFoundError:
            errors.append(f"{path.relative_to(root)}: file is missing")
        except (tomllib.TOMLDecodeError, UnicodeError) as exc:
            errors.append(f"{path.relative_to(root)}: invalid TOML: {exc}")
        return {}

    mixin = load(root / "horizon-mixin.toml")
    workspace = load(root / "Cargo.toml")
    engine = load(root / "engine" / "Cargo.toml")
    mod = mixin.get("mod", {})
    mod_id, version = mod.get("id"), mod.get("version")
    if not isinstance(mod_id, str) or not MOD_ID.fullmatch(mod_id):
        errors.append("horizon-mixin.toml: [mod].id must be a lowercase resource namespace")
    if not isinstance(version, str) or not VERSION.fullmatch(version):
        errors.append("horizon-mixin.toml: [mod].version must be semantic major.minor.patch")

    api_path = mixin.get("api", {}).get("path")
    if not isinstance(api_path, str) or not api_path:
        errors.append("horizon-mixin.toml: [api].path is required")
    else:
        resolved = (root / api_path).resolve()
        if root.resolve() not in resolved.parents and resolved != root.resolve():
            errors.append("horizon-mixin.toml: [api].path escapes the repository")
        elif not (resolved / "Cargo.toml").is_file():
            errors.append(f"horizon-mixin.toml: [api].path has no Cargo.toml: {api_path}")

    revision = mixin.get("compatibility", {}).get("horizon_revision")
    if not isinstance(revision, str) or not REVISION.fullmatch(revision):
        errors.append("horizon-mixin.toml: [compatibility].horizon_revision must be a full commit SHA")

    if "engine" not in workspace.get("workspace", {}).get("members", []):
        errors.append("Cargo.toml: workspace.members must include engine")
    package = engine.get("package", {})
    if version and package.get("version") != version:
        errors.append("engine/Cargo.toml: package.version must match the mod version")

    dependencies = engine.get("dependencies", {})
    dependency = next(
        (
            value for value in dependencies.values()
            if isinstance(value, dict) and value.get("package") == "horizon-client"
        ),
        dependencies.get("horizon-client", {}),
    )
    dep_path = dependency.get("path") if isinstance(dependency, dict) else None
    if not isinstance(dep_path, str) or not dep_path:
        errors.append("engine/Cargo.toml: the horizon-client package must use a local path dependency")
    elif dep_path.startswith("/"):
        normalized = dep_path.replace("\\", "/")
        if not normalized.endswith("/Horizon/crates/horizon-client"):
            errors.append("engine/Cargo.toml: absolute Horizon path must point at the local Horizon checkout")
    elif not dep_path.replace("\\", "/").endswith("../../horizon/crates/horizon-client"):
        errors.append("engine/Cargo.toml: unexpected relative Horizon path")

    workflow_path = root / WORKFLOW
    try:
        workflow = workflow_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        errors.append(f"{WORKFLOW}: file is missing")
    else:
        if "repository: arismoko/horizon" not in workflow:
            errors.append(f"{WORKFLOW}: must check out arismoko/horizon, which contains the pinned typed DataMap API")
        refs = re.findall(r"(?m)^\s*ref:\s*([0-9a-f]{40})\s*$", workflow)
        if revision and revision not in refs:
            errors.append(f"{WORKFLOW}: checkout ref must match pinned Horizon revision {revision}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    errors = validate(args.root.resolve())
    for error in errors:
        print(f"FAIL: {error}")
    if errors:
        return 1
    print("OK: mod metadata, Cargo workspace and Horizon pin agree")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
