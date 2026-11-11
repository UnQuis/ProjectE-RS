#!/usr/bin/env python3
"""Audit literal EMC values, recipe tables and machine tier ladders in Rust sources."""
import argparse
import json
import re
from pathlib import Path

RESOURCE_ID = re.compile(r"^[a-z0-9_.-]+:[a-z0-9/._-]+$")


def block_after(source: str, marker: str):
    start = source.find(marker)
    if start < 0:
        return None
    opening = source.find("{", start)
    if opening < 0:
        return None
    depth = 0
    for index in range(opening, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[opening + 1:index]
    return None


def rust_string_constants(source: str) -> dict[str, str]:
    return dict(re.findall(
        r"\bconst\s+([A-Z][A-Z0-9_]*)\s*:\s*&str\s*=\s*\"([^\"]+)\"\s*;",
        source,
    ))


def parse_emc(root: Path) -> tuple[dict[str, int], list[str]]:
    errors = []
    path = root / "engine" / "data" / "projecte" / "data_maps" / "item" / "emc.json"
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        return {}, [f"{path.relative_to(root)}: could not read EMC Data Map: {exc}"]
    if not isinstance(data, dict):
        return {}, [f"{path.relative_to(root)}: EMC Data Map root must be an object"]
    raw_values = data.get("values")
    if not isinstance(raw_values, dict):
        return {}, [f"{path.relative_to(root)}: EMC Data Map values must be an object"]
    values: dict[str, int] = {}
    for entry, amount in raw_values.items():
        item = entry[1:] if isinstance(entry, str) and entry.startswith("#") else entry
        if not isinstance(item, str) or not RESOURCE_ID.fullmatch(item):
            errors.append(f"{path.relative_to(root)}: invalid EMC item/tag id {entry!r}")
            continue
        if type(amount) is not int or amount <= 0:
            errors.append(f"{path.relative_to(root)}: EMC value for {entry} must be a positive integer")
            continue
        if not entry.startswith("#"):
            values[entry] = amount
    if not values:
        errors.append(f"{path.relative_to(root)}: no direct EMC item values were parsed")
    return values, errors


def parse_recipes(source: str) -> tuple[list[dict], list[str], list[str]]:
    errors, recipes = [], []
    constants = rust_string_constants(source)
    for kind in ("ShapedRecipe", "ShapelessRecipe"):
        for match in re.finditer(kind + r"\s*\{([^{}]*)\}", source, re.S):
            body = match.group(1)
            name_match = re.search(r'\bname:\s*"([^"]+)"', body)
            if not name_match:
                continue
            name = name_match.group(1)
            result_match = re.search(r"\bresult:\s*([A-Z][A-Z0-9_]*)", body)
            count_match = re.search(r"\bcount:\s*(\d+)", body)
            if not re.fullmatch(r"[a-z0-9_.-]+", name):
                errors.append(f"recipes.rs: invalid recipe name {name!r}")
            if not result_match or result_match.group(1) not in constants:
                errors.append(f"recipes.rs: {name} has an unresolved result constant")
            if not count_match or int(count_match.group(1)) < 1:
                errors.append(f"recipes.rs: {name} must produce a positive item count")
            result = constants.get(result_match.group(1), "") if result_match else ""
            recipe = {"name": name, "kind": kind, "result": result}
            if kind == "ShapedRecipe":
                pattern_match = re.search(r"\bpattern:\s*\[([^\]]*)\]", body, re.S)
                rows = re.findall(r'"([^"]*)"', pattern_match.group(1)) if pattern_match else []
                key_match = re.search(r"\bkey:\s*&\[(.*?)\]", body, re.S)
                keys = re.findall(r"\(\s*'([^']+)'\s*,\s*([A-Z][A-Z0-9_]*)\s*\)", key_match.group(1)) if key_match else []
                used = set("".join(rows)) - {" "}
                declared = {symbol for symbol, _ in keys}
                if len(rows) != 3 or any(len(row) != 3 for row in rows):
                    errors.append(f"recipes.rs: {name} must use a 3x3 pattern")
                if used != declared:
                    errors.append(f"recipes.rs: {name} pattern/key mismatch (used={sorted(used)}, declared={sorted(declared)})")
                ingredients = [constant for _, constant in keys]
            else:
                ingredients_match = re.search(r"\bingredients:\s*&\[(.*?)\]", body, re.S)
                ingredients = re.findall(r"\b([A-Z][A-Z0-9_]*)\b", ingredients_match.group(1)) if ingredients_match else []
                if not ingredients:
                    errors.append(f"recipes.rs: {name} has no ingredients")
            recipe["ingredients"] = []
            for constant in ingredients:
                if constant not in constants:
                    errors.append(f"recipes.rs: {name} references unknown ingredient constant {constant}")
                else:
                    recipe["ingredients"].append(constants[constant])
            recipes.append(recipe)

    names = [recipe["name"] for recipe in recipes]
    if len(names) != len(set(names)):
        errors.append("recipes.rs: recipe names must be unique across shaped and shapeless tables")
    catalyst_match = re.search(r"CATALYST_RECIPES:\s*&\[&str\]\s*=\s*&\[(.*?)\]", source, re.S)
    catalysts = re.findall(r'"([^"]+)"', catalyst_match.group(1)) if catalyst_match else []
    if catalyst_match:
        unknown = sorted(set(catalysts) - set(names))
        if unknown:
            errors.append("recipes.rs: catalyst list references missing recipes: " + ", ".join(unknown))
        if len(catalysts) != len(set(catalysts)):
            errors.append("recipes.rs: duplicate catalyst recipe name")
    return recipes, errors, catalysts


def parse_array(source: str, name: str, strings=False):
    match = re.search(r"\bconst\s+" + re.escape(name) + r"\s*:\s*\[[^\]]+\]\s*=\s*\[([^\]]*)\]", source, re.S)
    if not match:
        return None
    body = match.group(1)
    return re.findall(r'"([^"]+)"', body) if strings else [int(v.replace("_", "")) for v in re.findall(r"\d[\d_]*", body)]


def validate(root: Path) -> list[str]:
    errors = []
    engine = root / "engine" / "src"
    emc_path = engine / "emc.rs"
    if not emc_path.is_file():
        return ["engine/src/emc.rs is missing"]
    emc_source = emc_path.read_text(encoding="utf-8")
    values, found = parse_emc(root)
    errors.extend(found)
    if "impl DataMap for Emc" not in emc_source or "item.data(host, &MAP)" not in emc_source:
        errors.append("emc.rs: runtime EMC must use the typed Horizon DataMap read")
    if "data_maps/projecte:item/emc" not in emc_source:
        errors.append("emc.rs: the built-in projecte:emc datapack resource is not emitted")
    checked = block_after(emc_source, "pub fn checked_cost")
    if checked is not None and ("checked_mul" not in checked or "i64::try_from" not in checked):
        errors.append("emc.rs: checked_cost must use checked_mul and checked i64 conversion")

    recipes_path = engine / "recipes.rs"
    if recipes_path.is_file():
        recipes, found, _ = parse_recipes(recipes_path.read_text(encoding="utf-8"))
        errors.extend(found)
        for recipe in recipes:
            for item in [recipe["result"], *recipe["ingredients"]]:
                if item not in values:
                    errors.append(f"recipes.rs: {recipe['name']} uses {item} without an explicit EMC value")

    energy_path = engine / "energy.rs"
    if energy_path.is_file():
        source = energy_path.read_text(encoding="utf-8")
        stars = parse_array(source, "KLEIN_STARS", strings=True)
        capacity_body = block_after(source, "pub(crate) fn capacity")
        capacities = {key: int(value.replace("_", "")) for key, value in re.findall(r'"([^"]+)"\s*=>\s*([\d_]+)', capacity_body or "")}
        if stars is None:
            errors.append("energy.rs: KLEIN_STARS array is missing")
        else:
            if len(stars) != len(set(stars)):
                errors.append("energy.rs: duplicate Klein Star id")
            if set(stars) != set(capacities):
                errors.append("energy.rs: KLEIN_STARS and capacity() ids differ")
            numbers = [capacities[item] for item in stars if item in capacities]
            if len(numbers) != len(stars) or any(value <= 0 for value in numbers) or numbers != sorted(numbers) or len(set(numbers)) != len(numbers):
                errors.append("energy.rs: Klein Star capacities must be positive and strictly increasing")

    machines_path = engine / "machines.rs"
    if machines_path.is_file():
        source = machines_path.read_text(encoding="utf-8")
        groups = {
            "collector": ("COLLECTORS", "COLLECTOR_CAPACITY", "COLLECTOR_RATE_PER_SECOND"),
            "relay": ("RELAYS", "RELAY_CAPACITY", "TRANSFER_LIMIT", "RELAY_BONUS_PER_TICK_MILLI"),
        }
        for label, names in groups.items():
            arrays = [parse_array(source, name, strings=name in {"COLLECTORS", "RELAYS"}) for name in names]
            if any(array is None for array in arrays):
                errors.append(f"machines.rs: {label} tier table is incomplete ({', '.join(names)})")
                continue
            if any(len(array) != len(arrays[0]) for array in arrays[1:]):
                errors.append(f"machines.rs: {label} tier arrays have different lengths")
            for name, array in zip(names[1:], arrays[1:]):
                if any(value <= 0 for value in array) or array != sorted(array) or len(set(array)) != len(array):
                    errors.append(f"machines.rs: {name} must contain positive, strictly increasing values")
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
    print("OK: EMC, recipes and implemented machine tiers are internally consistent")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
