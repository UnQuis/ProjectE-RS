#!/usr/bin/env python3
"""Validate resource JSON, namespaced paths, recipes and EMC Data Maps."""
import argparse
import json
import re
from pathlib import Path

NAMESPACE = re.compile(r"^[a-z0-9_.-]+$")
RESOURCE_PATH = re.compile(r"^[a-z0-9/._-]+$")
IDENTIFIER = re.compile(r"^#?[a-z0-9_.-]+:[a-z0-9/._-]+$")
DATA_ROOTS = ("data", "engine/data")
ASSET_ROOTS = ("assets", "engine/assets")


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)


def valid_id(value) -> bool:
    return isinstance(value, str) and IDENTIFIER.fullmatch(value) is not None


def validate_recipe(data, path: Path) -> list[str]:
    errors = []
    if not isinstance(data, dict):
        return [f"{path}: recipe root must be an object"]
    recipe_type = data.get("type")
    if not valid_id(recipe_type):
        errors.append(f"{path}: recipe.type must be a namespaced resource id")
    result = data.get("result")
    if not isinstance(result, dict) or not valid_id(result.get("id", result.get("item"))):
        errors.append(f"{path}: result must contain a namespaced id or item")
    elif "count" in result and (type(result["count"]) is not int or result["count"] < 1):
        errors.append(f"{path}: result.count must be a positive integer")

    if recipe_type == "minecraft:crafting_shaped":
        pattern, key = data.get("pattern"), data.get("key")
        if not isinstance(pattern, list) or not 1 <= len(pattern) <= 3 or any(
            not isinstance(row, str) or not 1 <= len(row) <= 3 for row in pattern
        ):
            errors.append(f"{path}: shaped pattern must have 1-3 rows, each 1-3 characters wide")
        if not isinstance(key, dict):
            errors.append(f"{path}: shaped recipe key must be an object")
        elif isinstance(pattern, list) and all(isinstance(row, str) for row in pattern):
            used = set("".join(pattern)) - {" "}
            if used != set(key):
                errors.append(f"{path}: shaped key mismatch (used={sorted(used)}, declared={sorted(key)})")
            for symbol, ingredient in key.items():
                if not isinstance(symbol, str) or len(symbol) != 1 or symbol == " ":
                    errors.append(f"{path}: invalid shaped key symbol {symbol!r}")
                if not isinstance(ingredient, dict) or not (
                    valid_id(ingredient.get("item")) or valid_id(ingredient.get("tag"))
                ):
                    errors.append(f"{path}: key {symbol!r} must define an item or tag id")
    elif recipe_type == "minecraft:crafting_shapeless":
        ingredients = data.get("ingredients")
        if not isinstance(ingredients, list) or not ingredients:
            errors.append(f"{path}: shapeless recipe needs a non-empty ingredients array")
        else:
            for index, ingredient in enumerate(ingredients):
                valid = valid_id(ingredient) if isinstance(ingredient, str) else (
                    isinstance(ingredient, dict) and (
                        valid_id(ingredient.get("item")) or valid_id(ingredient.get("tag"))
                    )
                )
                if not valid:
                    errors.append(f"{path}: ingredient {index} is not an item/tag resource id")
    return errors


def validate_emc_map(data, path: Path) -> list[str]:
    errors = []
    if not isinstance(data, dict):
        return [f"{path}: EMC Data Map root must be an object"]
    extra = set(data) - {"replace", "values", "remove"}
    if extra:
        errors.append(f"{path}: unsupported keys: {', '.join(sorted(extra))}")
    if "replace" in data and type(data["replace"]) is not bool:
        errors.append(f"{path}: replace must be boolean")
    values = data.get("values", {})
    if not isinstance(values, dict):
        errors.append(f"{path}: values must be an object")
    else:
        for item, amount in values.items():
            if not valid_id(item):
                errors.append(f"{path}: invalid EMC item/tag id {item!r}")
            if type(amount) is not int or amount < 0:
                errors.append(f"{path}: EMC value for {item} must be a non-negative integer")
    removed = data.get("remove", [])
    if not isinstance(removed, list):
        errors.append(f"{path}: remove must be an array")
    else:
        for item in removed:
            if not valid_id(item):
                errors.append(f"{path}: invalid removed EMC item/tag id {item!r}")
    return errors


def validate(root: Path) -> list[str]:
    errors = []
    roots = [root / name for name in (*DATA_ROOTS, *ASSET_ROOTS)]
    files = sorted({file for base in roots if base.is_dir() for file in base.rglob("*.json")})
    for path in files:
        relative = path.relative_to(root)
        parts = relative.parts
        is_data = parts[0] == "data" or parts[:2] == ("engine", "data")
        offset = 1 if parts[0] in {"data", "assets"} else 2
        if len(parts) < offset + 2:
            errors.append(f"{relative}: resource path must include namespace, type and file")
        else:
            namespace, category = parts[offset], parts[offset + 1]
            resource = "/".join(parts[offset + 2:])
            if not NAMESPACE.fullmatch(namespace):
                errors.append(f"{relative}: invalid namespace {namespace!r}")
            if not RESOURCE_PATH.fullmatch(resource.removesuffix(".json")):
                errors.append(f"{relative}: resource path contains invalid characters")
            if not category or not re.fullmatch(r"[a-z0-9_.-]+", category):
                errors.append(f"{relative}: invalid resource category {category!r}")
        try:
            data = read_json(path)
        except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
            errors.append(f"{relative}: invalid JSON: {exc}")
            continue
        if is_data and "data_maps" in parts and path.name == "emc.json":
            errors.extend(validate_emc_map(data, relative))
        if is_data and any(part in {"recipe", "recipes"} for part in parts):
            errors.extend(validate_recipe(data, relative))
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
    print("OK: resource JSON and known recipe/EMC schemas are valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
