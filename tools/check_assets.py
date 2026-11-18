#!/usr/bin/env python3
"""Validate the mod's registered ids against assets and the creative tab.

Run from the repository root: python3 tools/check_assets.py
Every registered id must have an items/<id>.json definition, valid model and
texture references, a blockstate when it is a block, and a creative tab entry.
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets" / "projecte"
TAB = ROOT / "data" / "projecte" / "horizon" / "creative_tab" / "projecte.json"
SRC = ROOT / "src"

# The registration manifest. Keep this list in sync with Hooks::blocks/items in
# src/lib.rs; a new id must be added here, to the assets and to the creative tab
# in the same commit.
REGISTERED = """
projecte:advanced_alchemical_book projecte:aeternalis_fuel projecte:aeternalis_fuel_block
projecte:alchemical_barrel projecte:alchemical_chest projecte:alchemical_coal
projecte:alchemical_coal_block projecte:arcana_ring projecte:archangel_smite
projecte:basic_alchemical_book projecte:black_advanced_alchemical_chest projecte:black_alchemical_bag
projecte:black_hole_band projecte:blue_advanced_alchemical_chest projecte:blue_alchemical_bag
projecte:body_stone projecte:brown_advanced_alchemical_chest projecte:brown_alchemical_bag
projecte:catalytic_lens projecte:collector_mk1 projecte:collector_mk2
projecte:collector_mk3 projecte:colossal_star_drei projecte:colossal_star_ein
projecte:colossal_star_omega projecte:colossal_star_sphere projecte:colossal_star_vier
projecte:colossal_star_zwei projecte:compact_sun projecte:condenser_mk1
projecte:condenser_mk2 projecte:condenser_mk3 projecte:cyan_advanced_alchemical_chest
projecte:cyan_alchemical_bag projecte:dark_matter projecte:dark_matter_block
projecte:dark_matter_shield projecte:dark_matter_trident projecte:destruction_catalyst
projecte:divining_rod_1 projecte:divining_rod_2 projecte:divining_rod_3
projecte:dm_axe projecte:dm_boots projecte:dm_chestplate
projecte:dm_furnace projecte:dm_hammer projecte:dm_helmet
projecte:dm_hoe projecte:dm_leggings projecte:dm_pedestal
projecte:dm_pick projecte:dm_shears projecte:dm_shovel
projecte:dm_sword projecte:evertide_amulet projecte:final_star_shard
projecte:gem_boots projecte:gem_chestplate projecte:gem_helmet
projecte:gem_leggings projecte:gem_of_eternal_density projecte:gray_advanced_alchemical_chest
projecte:gray_alchemical_bag projecte:green_advanced_alchemical_chest projecte:green_alchemical_bag
projecte:harvest_goddess_band projecte:high_covalence_dust projecte:hyperkinetic_lens
projecte:ignition_ring projecte:infinite_fuel projecte:infinite_steak
projecte:interdiction_lantern projecte:interdiction_torch projecte:iron_band
projecte:klein_star_drei projecte:klein_star_ein projecte:klein_star_omega
projecte:klein_star_sphere projecte:klein_star_vier projecte:klein_star_zwei
projecte:life_stone projecte:light_blue_advanced_alchemical_chest projecte:light_blue_alchemical_bag
projecte:light_gray_advanced_alchemical_chest projecte:light_gray_alchemical_bag projecte:lime_advanced_alchemical_chest
projecte:lime_alchemical_bag projecte:low_covalence_dust projecte:magenta_advanced_alchemical_chest
projecte:magenta_alchemical_bag projecte:magenta_matter projecte:magenta_matter_block
projecte:magnum_star_drei projecte:magnum_star_ein projecte:magnum_star_omega
projecte:magnum_star_sphere projecte:magnum_star_vier projecte:magnum_star_zwei
projecte:master_alchemical_book projecte:medium_covalence_dust projecte:mercurial_eye
projecte:mind_stone projecte:mobius_fuel projecte:mobius_fuel_block
projecte:nova_cataclysm projecte:nova_catalyst projecte:orange_advanced_alchemical_chest
projecte:orange_alchemical_bag projecte:philosophers_stone projecte:pink_advanced_alchemical_chest
projecte:pink_alchemical_bag projecte:purple_advanced_alchemical_chest projecte:purple_alchemical_bag
projecte:red_advanced_alchemical_chest projecte:red_alchemical_bag projecte:red_matter
projecte:red_matter_block projecte:red_matter_shield projecte:red_matter_trident
projecte:relay_mk1 projecte:relay_mk2 projecte:relay_mk3
projecte:repair_talisman projecte:rm_axe projecte:rm_boots
projecte:rm_chestplate projecte:rm_furnace projecte:rm_hammer
projecte:rm_helmet projecte:rm_hoe projecte:rm_katar
projecte:rm_leggings projecte:rm_morning_star projecte:rm_pick
projecte:rm_shears projecte:rm_shovel projecte:rm_sword
projecte:soul_stone projecte:swiftwolf_rending_gale projecte:tome
projecte:transmutation_interface projecte:transmutation_table projecte:transmutation_tablet
projecte:void_ring projecte:volcanite_amulet projecte:watch_of_flowing_time
projecte:white_advanced_alchemical_chest projecte:white_alchemical_bag projecte:yellow_advanced_alchemical_chest
projecte:yellow_alchemical_bag projecte:zero_ring
""".split()
ids = set(REGISTERED)
problems = []


def model_ok(path, seen=None):
    seen = seen if seen is not None else set()
    if not path.startswith("projecte:") or path in seen:
        return True
    seen.add(path)
    model = ASSETS / "models" / (path[9:] + ".json")
    if not model.is_file():
        problems.append(f"missing model {path}")
        return False
    data = json.loads(model.read_text())
    if "overrides" in data:
        problems.append(f"{path}: Java-only overrides (Horizon ignores them)")
    loader = data.get("loader")
    if isinstance(loader, str) and loader.startswith("neoforge:"):
        problems.append(f"{path}: Java-only loader {loader}")
    # A `minecraft:special` base model carries only display transforms, as
    # vanilla's item/shield.json does; the geometry comes from a part model.
    if "parent" not in data and "elements" not in data and "display" not in data:
        problems.append(f"{path}: model has neither parent, elements nor display")
    parent = data.get("parent", "")
    if isinstance(parent, str):
        model_ok(parent, seen)
    for value in data.get("textures", {}).values():
        if isinstance(value, str) and value.startswith("projecte:"):
            texture = ASSETS / "textures" / (value[9:] + ".png")
            if not texture.is_file():
                problems.append(f"missing texture {value}")
    return True


def definition_models(node):
    """Every model id a 26.3 item definition names, at any depth."""
    found = []
    if isinstance(node, dict):
        for key, value in node.items():
            if key in ("model", "base") and isinstance(value, str):
                found.append(value)
            else:
                found.extend(definition_models(value))
    elif isinstance(node, list):
        for value in node:
            found.extend(definition_models(value))
    return found


# These have dedicated item/generated models: their items/<id>.json must point
# at the item model, never at the block model (which lacked proper UVs).
ITEM_MODEL_REQUIRED = {
    "transmutation_tablet", "dark_matter_shield", "red_matter_shield",
}

LANG = ROOT / "assets" / "projecte" / "lang" / "en_us.json"
lang = json.loads(LANG.read_text())

tab_ids = set(re.findall(r"projecte:[a-z0-9_]+", TAB.read_text()))
for ident in sorted(ids):
    name = ident[9:]
    definition = ASSETS / "items" / (name + ".json")
    if not definition.is_file():
        problems.append(f"{ident}: missing items definition")
        continue
    item_models = definition_models(json.loads(definition.read_text()))
    if name in ITEM_MODEL_REQUIRED:
        if not item_models or any(not m.startswith("projecte:item/") for m in item_models):
            problems.append(f"{ident}: expected item models, got {item_models}")
    for item_model in item_models:
        model_ok(item_model)
    blockstate = ASSETS / "blockstates" / (name + ".json")
    if blockstate.is_file():
        state = json.loads(blockstate.read_text())
        if "forge_marker" in state:
            problems.append(f"{ident}: Java-only forge_marker in blockstate")
        for variant in state.get("variants", {}).values():
            model_ok(variant["model"])
    if ident not in tab_ids:
        problems.append(f"{ident}: missing from creative tab")
    name_key = ident[9:]
    if f"item.projecte.{name_key}" not in lang and f"block.projecte.{name_key}" not in lang:
        problems.append(f"{ident}: missing item./block. entry in en_us.json")

for ident in sorted(tab_ids - ids):
    problems.append(f"{ident}: in creative tab but not registered")

# The special shield renderers return these geometry models from code, so no
# item definition names them directly.
for name in ("dark_matter_shield", "red_matter_shield"):
    model_ok(f"projecte:item/{name}")

print(f"checked {len(ids)} registered ids, {len(tab_ids)} creative tab ids")
for problem in problems:
    print(" -", problem)
sys.exit(1 if problems else 0)
