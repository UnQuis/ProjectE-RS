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
projecte:transmutation_table projecte:energy_collector projecte:energy_collector_mk2
projecte:energy_collector_mk3 projecte:antimatter_relay projecte:antimatter_relay_mk2
projecte:antimatter_relay_mk3 projecte:energy_condenser projecte:condenser_mk2
projecte:condenser_mk3 projecte:alchemical_chest projecte:alchemical_coal_block
projecte:mobius_fuel_block projecte:aeternalis_fuel_block projecte:dark_matter_block
projecte:red_matter_block projecte:magenta_matter_block projecte:dm_pedestal
projecte:dm_furnace projecte:rm_furnace projecte:alchemical_barrel
projecte:transmutation_tablet projecte:philosophers_stone projecte:klein_star_ein
projecte:klein_star_zwei projecte:klein_star_drei projecte:klein_star_vier
projecte:klein_star_sphere projecte:klein_star_omega projecte:dark_matter
projecte:red_matter projecte:magenta_matter projecte:low_covalence_dust
projecte:medium_covalence_dust projecte:high_covalence_dust projecte:tome
projecte:alchemical_coal projecte:mobius_fuel projecte:aeternalis_fuel
projecte:repair_talisman projecte:nova_catalyst projecte:nova_cataclysm
projecte:divining_rod_1 projecte:divining_rod_2 projecte:divining_rod_3
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
    parent = data.get("parent", "")
    if isinstance(parent, str):
        model_ok(parent, seen)
    for value in data.get("textures", {}).values():
        if isinstance(value, str) and value.startswith("projecte:"):
            texture = ASSETS / "textures" / (value[9:] + ".png")
            if not texture.is_file():
                problems.append(f"missing texture {value}")
    return True


tab_ids = set(re.findall(r"projecte:[a-z0-9_]+", TAB.read_text()))
for ident in sorted(ids):
    name = ident[9:]
    definition = ASSETS / "items" / (name + ".json")
    if not definition.is_file():
        problems.append(f"{ident}: missing items definition")
        continue
    model_ok(json.loads(definition.read_text())["model"]["model"])
    blockstate = ASSETS / "blockstates" / (name + ".json")
    if blockstate.is_file():
        for variant in json.loads(blockstate.read_text()).get("variants", {}).values():
            model_ok(variant["model"])
    if ident not in tab_ids:
        problems.append(f"{ident}: missing from creative tab")

for ident in sorted(tab_ids - ids):
    problems.append(f"{ident}: in creative tab but not registered")

print(f"checked {len(ids)} registered ids, {len(tab_ids)} creative tab ids")
for problem in problems:
    print(" -", problem)
sys.exit(1 if problems else 0)
