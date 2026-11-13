#!/usr/bin/env python3
"""Validate the native-mod manifest, Cargo API pin and mod-only CI."""
import argparse
import re
import tomllib
from pathlib import Path

REVISION = re.compile(r"^[0-9a-f]{40}$")
MOD_ID = re.compile(r"^[a-z0-9_]{1,64}$")
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

    manifest = load(root / "engine" / "horizon-mod.toml")
    workspace = load(root / "Cargo.toml")
    engine = load(root / "engine" / "Cargo.toml")
    mod_id, version = manifest.get("id"), manifest.get("version")
    if not isinstance(mod_id, str) or not MOD_ID.fullmatch(mod_id):
        errors.append("engine/horizon-mod.toml: id must match [a-z0-9_]{1,64}")
    if not isinstance(version, str) or not VERSION.fullmatch(version):
        errors.append("engine/horizon-mod.toml: version must be semantic major.minor.patch")
    if manifest.get("api") != "0.2":
        errors.append('engine/horizon-mod.toml: api must be "0.2"')
    if manifest.get("side") not in ("both", "client", "server"):
        errors.append("engine/horizon-mod.toml: side must be both, client or server")
    if "registries" not in manifest.get("capabilities", []):
        errors.append('engine/horizon-mod.toml: capabilities must include "registries"')

    native = manifest.get("native", {})
    horizon = native.get("horizon")
    mixins = native.get("mixins")
    if native.get("game") != "26.3":
        errors.append('engine/horizon-mod.toml: [native].game must be "26.3"')
    if not isinstance(horizon, str) or not REVISION.fullmatch(horizon):
        errors.append("engine/horizon-mod.toml: [native].horizon must be a full commit SHA")
    if not isinstance(mixins, str) or not REVISION.fullmatch(mixins):
        errors.append("engine/horizon-mod.toml: [native].mixins must be a full commit SHA")

    if "engine" not in workspace.get("workspace", {}).get("members", []):
        errors.append("Cargo.toml: workspace.members must include engine")
    package = engine.get("package", {})
    if version and package.get("version") != version:
        errors.append("engine/Cargo.toml: package.version must match the mod version")

    dependency = engine.get("dependencies", {}).get("horizon-mod-api", {})
    if not isinstance(dependency, dict):
        errors.append("engine/Cargo.toml: horizon-mod-api git dependency is required")
    else:
        if dependency.get("git") != "https://github.com/arismoko/horizon":
            errors.append("engine/Cargo.toml: horizon-mod-api must come from the official Horizon repository")
        if dependency.get("rev") != horizon:
            errors.append("engine/Cargo.toml: horizon-mod-api rev must match [native].horizon")
        if "path" in dependency:
            errors.append("engine/Cargo.toml: do not use a local Horizon path dependency")

    if (root / "horizon-mixin.toml").exists():
        errors.append("horizon-mixin.toml: obsolete manifest must be removed")

    workflow_path = root / WORKFLOW
    try:
        workflow = workflow_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        errors.append(f"{WORKFLOW}: file is missing")
    else:
        if "repository: arismoko/horizon" in workflow:
            errors.append(f"{WORKFLOW}: must not check out or compile the Horizon game")
        if "cargo check --manifest-path engine/Cargo.toml --all-targets" not in workflow:
            errors.append(f"{WORKFLOW}: must compile the native mod crate directly")
        if "cargo generate-lockfile --manifest-path engine/Cargo.toml" not in workflow:
            errors.append(f"{WORKFLOW}: must resolve the pinned native-mod dependencies")
        if "/home/" in workflow or "/home/" in (root / "engine" / "Cargo.toml").read_text(encoding="utf-8"):
            errors.append("CI and Cargo manifests must not depend on a developer's absolute home path")
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
    print("OK: native manifest, Cargo API pin and mod-only CI agree")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
