//! Declarative crafting recipes, emitted as data-pack files.
//!
//! Two shapes are kept apart on purpose: a recipe whose layout is part of the
//! gameplay (a checkered ring, a 3x3 block) is `ShapedRecipe`, while a recipe
//! whose layout carries no meaning is `ShapelessRecipe`.
//!
//! Some recipes need the Philosopher's Stone as a *catalyst*: ProjectE's stone
//! is never used up, so vanilla crafting cannot express it. The data-only
//! recipe lists the stone as an ingredient; [`is_catalyst_recipe`] names the
//! recipes for which the crafting listener gives the stone back. See
//! `API_WORKAROUNDS.md` for why this is a workaround and what it costs.

/// A shaped (3x3 pattern) crafting recipe.
pub(crate) struct ShapedRecipe {
    /// Recipe name inside the `projecte` namespace.
    pub name: &'static str,
    /// Three rows of three slots; `' '` is an empty slot.
    pub pattern: [&'static str; 3],
    /// Pattern letter to item id.
    pub key: &'static [(char, &'static str)],
    pub result: &'static str,
    pub count: u32,
}

/// A shapeless crafting recipe.
pub(crate) struct ShapelessRecipe {
    /// Recipe name inside the `projecte` namespace.
    pub name: &'static str,
    /// One entry per consumed item; a repeated id means that many items.
    pub ingredients: &'static [&'static str],
    pub result: &'static str,
    pub count: u32,
}

/// This mod's namespace, shared by every recipe id it emits.
pub(crate) const MOD_ID: &str = "projecte";

const STONE: &str = "projecte:philosophers_stone";
const REDSTONE: &str = "minecraft:redstone";
const GLOWSTONE_DUST: &str = "minecraft:glowstone_dust";
const GLOWSTONE: &str = "minecraft:glowstone";
const DIAMOND: &str = "minecraft:diamond";
const DIAMOND_BLOCK: &str = "minecraft:diamond_block";
const OBSIDIAN: &str = "minecraft:obsidian";
const STONE_BLOCK: &str = "minecraft:stone";
const FURNACE: &str = "minecraft:furnace";
const CHEST: &str = "minecraft:chest";
const GLASS: &str = "minecraft:glass";
const IRON: &str = "minecraft:iron_ingot";
const GOLD: &str = "minecraft:gold_ingot";
const EMERALD: &str = "minecraft:emerald";
const ENDER_PEARL: &str = "minecraft:ender_pearl";
const NETHER_STAR: &str = "minecraft:nether_star";
const COAL: &str = "minecraft:coal";
const DARK_MATTER: &str = "projecte:dark_matter";
const RED_MATTER: &str = "projecte:red_matter";

pub(crate) const ALCHEMICAL_COAL: &str = "projecte:alchemical_coal";
pub(crate) const MOBIUS_FUEL: &str = "projecte:mobius_fuel";
pub(crate) const AETERNALIS_FUEL: &str = "projecte:aeternalis_fuel";
pub(crate) const ALCHEMICAL_COAL_BLOCK: &str = "projecte:alchemical_coal_block";
pub(crate) const MOBIUS_FUEL_BLOCK: &str = "projecte:mobius_fuel_block";
pub(crate) const AETERNALIS_FUEL_BLOCK: &str = "projecte:aeternalis_fuel_block";

pub(crate) const COLLECTOR: &str = "projecte:energy_collector";
pub(crate) const COLLECTOR_MK2: &str = "projecte:energy_collector_mk2";
pub(crate) const COLLECTOR_MK3: &str = "projecte:energy_collector_mk3";
pub(crate) const RELAY: &str = "projecte:antimatter_relay";
pub(crate) const RELAY_MK2: &str = "projecte:antimatter_relay_mk2";
pub(crate) const RELAY_MK3: &str = "projecte:antimatter_relay_mk3";
pub(crate) const TABLE: &str = "projecte:transmutation_table";
pub(crate) const ALCHEMICAL_CHEST: &str = "projecte:alchemical_chest";
pub(crate) const ENERGY_CONDENSER: &str = "projecte:energy_condenser";
pub(crate) const KLEIN_STAR_EIN: &str = "projecte:klein_star_ein";
pub(crate) const KLEIN_STAR_ZWEI: &str = "projecte:klein_star_zwei";
pub(crate) const KLEIN_STAR_DREI: &str = "projecte:klein_star_drei";
pub(crate) const KLEIN_STAR_VIER: &str = "projecte:klein_star_vier";

/// Every recipe that needs the Philosopher's Stone without consuming it.
///
/// The stone is a crafting catalyst: the recipe data lists it as an ordinary
/// ingredient (the only form the pinned data-pack recipe format has), and the
/// crafting listener returns one stone for each of these recipes taken from a
/// crafting grid. A recipe missing from this list would eat the stone, and a
/// recipe listed here without the stone in its data would mint one.
pub(crate) const CATALYST_RECIPES: &[&str] = &[
    "transmutation_table",
    "alchemical_coal",
    "mobius_fuel",
    "aeternalis_fuel",
    "stone_iron_to_gold",
    "stone_gold_to_iron",
    "stone_gold_to_diamond",
    "stone_diamond_to_gold",
    "stone_diamond_to_emerald",
    "stone_emerald_to_diamond",
    "stone_iron_to_pearl",
    "stone_pearl_to_iron",
];

/// Layout-carrying recipes. The four obsidian corners, rings and 3x3 packs
/// matter: a grid that reads as the documented shape is what players build.
pub(crate) const SHAPED: &[ShapedRecipe] = &[
    // Public behavior: four redstone and four glowstone dust in a checkered
    // ring around one diamond.
    ShapedRecipe {
        name: "philosophers_stone",
        pattern: ["RGR", "GDG", "RGR"],
        key: &[('R', REDSTONE), ('G', GLOWSTONE_DUST), ('D', DIAMOND)],
        result: STONE,
        count: 1,
    },
    // Four obsidian corners, four stone sides, the stone in the middle.
    ShapedRecipe {
        name: "transmutation_table",
        pattern: ["OSO", "SPS", "OSO"],
        key: &[('O', OBSIDIAN), ('S', STONE_BLOCK), ('P', STONE)],
        result: TABLE,
        count: 1,
    },
    // Glowstone ring, diamond block core, furnace under it.
    ShapedRecipe {
        name: "energy_collector",
        pattern: ["GGG", "GDG", "GFG"],
        key: &[('G', GLOWSTONE), ('D', DIAMOND_BLOCK), ('F', FURNACE)],
        result: COLLECTOR,
        count: 1,
    },
    // The same ring in obsidian, with a glass core and a diamond block.
    ShapedRecipe {
        name: "antimatter_relay",
        pattern: ["OOO", "OXO", "ODO"],
        key: &[('O', OBSIDIAN), ('X', GLASS), ('D', DIAMOND_BLOCK)],
        result: RELAY,
        count: 1,
    },
    // Provisional Alchemical Chest recipe; verify this layout for the target version.
    ShapedRecipe {
        name: "alchemical_chest",
        pattern: ["OOO", "OCO", "OOO"],
        key: &[('O', OBSIDIAN), ('C', CHEST)],
        result: ALCHEMICAL_CHEST,
        count: 1,
    },
    // Energy Condenser MK1: obsidian corners, diamond sides, Alchemical Chest core.
    ShapedRecipe {
        name: "energy_condenser",
        pattern: ["ODO", "DCD", "ODO"],
        key: &[('O', OBSIDIAN), ('D', DIAMOND), ('C', ALCHEMICAL_CHEST)],
        result: ENERGY_CONDENSER,
        count: 1,
    },
    // Tier upgrades: the previous machine in the middle, glowstone around it,
    // the next matter tier on top.
    ShapedRecipe {
        name: "energy_collector_mk2",
        pattern: ["GGM", "GCG", "GGG"],
        key: &[('G', GLOWSTONE), ('M', DARK_MATTER), ('C', COLLECTOR)],
        result: COLLECTOR_MK2,
        count: 1,
    },
    ShapedRecipe {
        name: "energy_collector_mk3",
        pattern: ["GGR", "GCG", "GGG"],
        key: &[('G', GLOWSTONE), ('R', RED_MATTER), ('C', COLLECTOR_MK2)],
        result: COLLECTOR_MK3,
        count: 1,
    },
    ShapedRecipe {
        name: "antimatter_relay_mk2",
        pattern: ["GGM", "GRG", "GGG"],
        key: &[('G', GLOWSTONE), ('M', DARK_MATTER), ('R', RELAY)],
        result: RELAY_MK2,
        count: 1,
    },
    ShapedRecipe {
        name: "antimatter_relay_mk3",
        pattern: ["GGM", "GRG", "GGG"],
        key: &[('G', GLOWSTONE), ('M', RED_MATTER), ('R', RELAY_MK2)],
        result: RELAY_MK3,
        count: 1,
    },
    // Klein Star Ein: eight diamonds around a glowstone block.
    ShapedRecipe {
        name: "klein_star_ein",
        pattern: ["DDD", "DGD", "DDD"],
        key: &[('D', DIAMOND), ('G', GLOWSTONE)],
        result: KLEIN_STAR_EIN,
        count: 1,
    },
    // Fuel blocks: nine fuels pack into one block.
    ShapedRecipe {
        name: "alchemical_coal_block",
        pattern: ["AAA", "AAA", "AAA"],
        key: &[('A', ALCHEMICAL_COAL)],
        result: ALCHEMICAL_COAL_BLOCK,
        count: 1,
    },
    ShapedRecipe {
        name: "mobius_fuel_block",
        pattern: ["MMM", "MMM", "MMM"],
        key: &[('M', MOBIUS_FUEL)],
        result: MOBIUS_FUEL_BLOCK,
        count: 1,
    },
    ShapedRecipe {
        name: "aeternalis_fuel_block",
        pattern: ["AAA", "AAA", "AAA"],
        key: &[('A', AETERNALIS_FUEL)],
        result: AETERNALIS_FUEL_BLOCK,
        count: 1,
    },
    // Public behavior: eight iron ingots around the stone become one gold.
    ShapedRecipe {
        name: "stone_iron_to_gold",
        pattern: ["III", "IPI", "III"],
        key: &[('I', IRON), ('P', STONE)],
        result: GOLD,
        count: 1,
    },
];

/// Layout-free recipes. The stone still appears where the conversion needs it.
pub(crate) const SHAPELESS: &[ShapelessRecipe] = &[
    // Four of a fuel plus the stone make one of the next fuel.
    ShapelessRecipe {
        name: "alchemical_coal",
        ingredients: &[COAL, COAL, COAL, COAL, STONE],
        result: ALCHEMICAL_COAL,
        count: 1,
    },
    ShapelessRecipe {
        name: "mobius_fuel",
        ingredients: &[
            ALCHEMICAL_COAL,
            ALCHEMICAL_COAL,
            ALCHEMICAL_COAL,
            ALCHEMICAL_COAL,
            STONE,
        ],
        result: MOBIUS_FUEL,
        count: 1,
    },
    ShapelessRecipe {
        name: "aeternalis_fuel",
        ingredients: &[MOBIUS_FUEL, MOBIUS_FUEL, MOBIUS_FUEL, MOBIUS_FUEL, STONE],
        result: AETERNALIS_FUEL,
        count: 1,
    },
    // Unpacking keeps the fuel chain reversible.
    ShapelessRecipe {
        name: "alchemical_coal_from_block",
        ingredients: &[ALCHEMICAL_COAL_BLOCK],
        result: ALCHEMICAL_COAL,
        count: 9,
    },
    ShapelessRecipe {
        name: "mobius_fuel_from_block",
        ingredients: &[MOBIUS_FUEL_BLOCK],
        result: MOBIUS_FUEL,
        count: 9,
    },
    ShapelessRecipe {
        name: "aeternalis_fuel_from_block",
        ingredients: &[AETERNALIS_FUEL_BLOCK],
        result: AETERNALIS_FUEL,
        count: 9,
    },
    // Klein Star upgrades: four of a tier plus a nether star.
    ShapelessRecipe {
        name: "klein_star_zwei",
        ingredients: &[
            KLEIN_STAR_EIN,
            KLEIN_STAR_EIN,
            KLEIN_STAR_EIN,
            KLEIN_STAR_EIN,
            NETHER_STAR,
        ],
        result: KLEIN_STAR_ZWEI,
        count: 1,
    },
    ShapelessRecipe {
        name: "klein_star_drei",
        ingredients: &[
            KLEIN_STAR_ZWEI,
            KLEIN_STAR_ZWEI,
            KLEIN_STAR_ZWEI,
            KLEIN_STAR_ZWEI,
            NETHER_STAR,
        ],
        result: KLEIN_STAR_DREI,
        count: 1,
    },
    ShapelessRecipe {
        name: "klein_star_vier",
        ingredients: &[
            KLEIN_STAR_DREI,
            KLEIN_STAR_DREI,
            KLEIN_STAR_DREI,
            KLEIN_STAR_DREI,
            NETHER_STAR,
        ],
        result: KLEIN_STAR_VIER,
        count: 1,
    },
    // Stone conversions that carry no layout, with their reverse direction so
    // the exchanges stay worth the same EMC either way.
    ShapelessRecipe {
        name: "stone_gold_to_iron",
        ingredients: &[GOLD, STONE],
        result: IRON,
        count: 8,
    },
    ShapelessRecipe {
        name: "stone_gold_to_diamond",
        ingredients: &[GOLD, GOLD, GOLD, GOLD, STONE],
        result: DIAMOND,
        count: 1,
    },
    ShapelessRecipe {
        name: "stone_diamond_to_gold",
        ingredients: &[DIAMOND, STONE],
        result: GOLD,
        count: 4,
    },
    ShapelessRecipe {
        name: "stone_diamond_to_emerald",
        ingredients: &[DIAMOND, DIAMOND, STONE],
        result: EMERALD,
        count: 1,
    },
    ShapelessRecipe {
        name: "stone_emerald_to_diamond",
        ingredients: &[EMERALD, STONE],
        result: DIAMOND,
        count: 2,
    },
    ShapelessRecipe {
        name: "stone_iron_to_pearl",
        ingredients: &[IRON, IRON, IRON, IRON, STONE],
        result: ENDER_PEARL,
        count: 1,
    },
    ShapelessRecipe {
        name: "stone_pearl_to_iron",
        ingredients: &[ENDER_PEARL, STONE],
        result: IRON,
        count: 4,
    },
    // Dark and red matter: the current ingredients are audit placeholders; the
    // matter phase replaces them once the public recipe is confirmed.
    ShapelessRecipe {
        name: "dark_matter",
        ingredients: &[COAL, COAL, COAL, COAL, COAL, COAL, COAL, COAL, DIAMOND],
        result: DARK_MATTER,
        count: 1,
    },
    ShapelessRecipe {
        name: "red_matter",
        ingredients: &[
            DARK_MATTER,
            DARK_MATTER,
            DARK_MATTER,
            DARK_MATTER,
            DARK_MATTER,
            DARK_MATTER,
            DARK_MATTER,
            DARK_MATTER,
            DIAMOND_BLOCK,
        ],
        result: RED_MATTER,
        count: 1,
    },
];

/// Whether a matched recipe id (`projecte:alchemical_coal`) is a catalyst
/// recipe whose Philosopher's Stone must be returned to the crafter.
pub(crate) fn is_catalyst_recipe(recipe_id: &str) -> bool {
    let Some(name) = recipe_id
        .strip_prefix(MOD_ID)
        .and_then(|rest| rest.strip_prefix(':'))
    else {
        return false;
    };
    CATALYST_RECIPES.contains(&name)
}

/// The data-pack files for every recipe, keyed as `data_pack_files` expects.
pub(crate) fn files() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::with_capacity(SHAPED.len() + SHAPELESS.len());
    for recipe in SHAPED {
        out.push((
            format!("recipe/{MOD_ID}:{}", recipe.name),
            shaped_json(recipe).into_bytes(),
        ));
    }
    for recipe in SHAPELESS {
        out.push((
            format!("recipe/{MOD_ID}:{}", recipe.name),
            shapeless_json(recipe).into_bytes(),
        ));
    }
    out
}

fn shaped_json(recipe: &ShapedRecipe) -> String {
    let pattern = recipe
        .pattern
        .iter()
        .map(|row| format!("\"{row}\""))
        .collect::<Vec<_>>()
        .join(",");
    let key = recipe
        .key
        .iter()
        .map(|(letter, id)| format!("\"{letter}\":{{\"item\":\"{id}\"}}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"type\":\"minecraft:crafting_shaped\",\"category\":\"misc\",\"pattern\":[{pattern}],\"key\":{{{key}}},\"result\":{{\"id\":\"{}\",\"count\":{}}}}}",
        recipe.result, recipe.count
    )
}

fn shapeless_json(recipe: &ShapelessRecipe) -> String {
    let list = recipe
        .ingredients
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"type\":\"minecraft:crafting_shapeless\",\"category\":\"misc\",\"ingredients\":[{list}],\"result\":{{\"id\":\"{}\",\"count\":{}}}}}",
        recipe.result, recipe.count
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_result() -> Vec<&'static str> {
        let mut results: Vec<&'static str> = SHAPED.iter().map(|r| r.result).collect();
        results.extend(SHAPELESS.iter().map(|r| r.result));
        results
    }

    #[test]
    fn shaped_patterns_are_three_by_three_and_match_their_key() {
        for recipe in SHAPED {
            for row in recipe.pattern {
                assert_eq!(
                    row.chars().count(),
                    3,
                    "{} has a row of the wrong width",
                    recipe.name
                );
            }
            for row in recipe.pattern {
                for letter in row.chars() {
                    if letter == ' ' {
                        continue;
                    }
                    assert!(
                        recipe.key.iter().any(|(k, _)| *k == letter),
                        "{} uses {letter} without a key entry",
                        recipe.name
                    );
                }
            }
            for (letter, _) in recipe.key {
                assert!(
                    recipe
                        .pattern
                        .iter()
                        .any(|row| row.chars().any(|c| c == *letter)),
                    "{} declares {letter} but never places it",
                    recipe.name
                );
            }
        }
    }

    #[test]
    fn recipe_names_are_unique_and_namespaced() {
        let mut names: Vec<&str> = SHAPED.iter().map(|r| r.name).collect();
        names.extend(SHAPELESS.iter().map(|r| r.name));
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "duplicate recipe name");
        for name in names {
            assert!(!name.contains(':'), "{name} must not carry a namespace");
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn every_ingredient_and_result_has_emc_value() {
        for id in SHAPED.iter().flat_map(|r| r.key.iter().map(|(_, id)| *id)) {
            assert!(crate::emc::value(id) > 0, "{id} has no EMC value");
        }
        for id in SHAPELESS.iter().flat_map(|r| r.ingredients.iter().copied()) {
            assert!(crate::emc::value(id) > 0, "{id} has no EMC value");
        }
        for id in every_result() {
            assert!(crate::emc::value(id) > 0, "{id} has no EMC value");
        }
    }

    #[test]
    fn catalyst_recipes_name_the_stone_and_no_other_recipe_does() {
        for name in CATALYST_RECIPES {
            let shaped = SHAPED.iter().find(|r| r.name == *name);
            let shapeless = SHAPELESS.iter().find(|r| r.name == *name);
            let holds_stone = shaped
                .map(|r| r.key.iter().any(|(_, id)| *id == STONE))
                .or_else(|| shapeless.map(|r| r.ingredients.contains(&STONE)));
            assert_eq!(
                holds_stone,
                Some(true),
                "{name} is a catalyst recipe without the stone"
            );
        }
        for recipe in SHAPED {
            if recipe.name != "philosophers_stone" {
                let listed = CATALYST_RECIPES.contains(&recipe.name);
                let holds_stone = recipe.key.iter().any(|(_, id)| *id == STONE);
                assert_eq!(holds_stone, listed, "{} catalyst mismatch", recipe.name);
            }
        }
        for recipe in SHAPELESS {
            let listed = CATALYST_RECIPES.contains(&recipe.name);
            let holds_stone = recipe.ingredients.contains(&STONE);
            assert_eq!(holds_stone, listed, "{} catalyst mismatch", recipe.name);
        }
        assert!(!is_catalyst_recipe("projecte:philosophers_stone"));
        assert!(is_catalyst_recipe("projecte:alchemical_coal"));
        assert!(!is_catalyst_recipe("minecraft:oak_planks"));
        assert!(!is_catalyst_recipe("projecte:unlisted"));
    }

    #[test]
    fn fuel_chain_recipes_consume_four_of_the_previous_fuel() {
        for (recipe_name, fuel, previous) in [
            ("alchemical_coal", ALCHEMICAL_COAL, COAL),
            ("mobius_fuel", MOBIUS_FUEL, ALCHEMICAL_COAL),
            ("aeternalis_fuel", AETERNALIS_FUEL, MOBIUS_FUEL),
        ] {
            let recipe = SHAPELESS
                .iter()
                .find(|r| r.name == recipe_name)
                .expect("fuel recipe");
            assert_eq!(recipe.result, fuel);
            assert_eq!(recipe.count, 1);
            assert_eq!(
                recipe
                    .ingredients
                    .iter()
                    .filter(|id| **id == previous)
                    .count(),
                4
            );
            assert_eq!(
                recipe.ingredients.iter().filter(|id| **id == STONE).count(),
                1
            );
        }
    }

    #[test]
    fn fuel_block_recipes_pack_nine_and_unpack_nine() {
        for (fuel, block) in [
            (ALCHEMICAL_COAL, ALCHEMICAL_COAL_BLOCK),
            (MOBIUS_FUEL, MOBIUS_FUEL_BLOCK),
            (AETERNALIS_FUEL, AETERNALIS_FUEL_BLOCK),
        ] {
            let pack = SHAPED
                .iter()
                .find(|r| r.result == block)
                .expect("block recipe");
            assert_eq!(pack.count, 1);
            assert_eq!(
                pack.pattern
                    .iter()
                    .flat_map(|row| row.chars())
                    .filter(|c| *c != ' ')
                    .count(),
                9
            );
            let unpack = SHAPELESS
                .iter()
                .find(|r| r.ingredients.len() == 1 && r.ingredients[0] == block)
                .expect("unpack recipe");
            assert_eq!(unpack.result, fuel);
            assert_eq!(unpack.count, 9);
        }
    }

    #[test]
    fn emitted_files_are_well_formed_json() {
        let files = files();
        assert_eq!(files.len(), SHAPED.len() + SHAPELESS.len());
        let mut keys: Vec<&str> = files.iter().map(|(key, _)| key.as_str()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), files.len(), "duplicate data-pack key");
        for (key, body) in &files {
            let text = std::str::from_utf8(body).expect("recipe json is utf-8");
            let value: horizon_client::mod_api::serde_json::Value =
                horizon_client::mod_api::serde_json::from_str(text)
                    .unwrap_or_else(|error| panic!("{key}: {error} in {text}"));
            let kind = value["type"].as_str().expect("recipe type");
            assert!(
                kind == "minecraft:crafting_shaped" || kind == "minecraft:crafting_shapeless",
                "{key}: {kind}"
            );
            assert_eq!(value["result"]["count"].as_u64(), Some(1), "{key}");
            if kind == "minecraft:crafting_shaped" {
                assert_eq!(value["pattern"].as_array().map(|rows| rows.len()), Some(3));
            } else {
                assert!(value["ingredients"].as_array().is_some(), "{key}");
            }
        }
    }

    #[test]
    fn condenser_recipe_chain_has_matching_emc_values() {
        let chest = SHAPED
            .iter()
            .find(|recipe| recipe.name == "alchemical_chest")
            .expect("Alchemical Chest recipe");
        assert_eq!(chest.result, ALCHEMICAL_CHEST);
        assert_eq!(crate::emc::value(ALCHEMICAL_CHEST), 544);
        assert_eq!(crate::emc::value(CHEST), 32);
        assert_eq!(
            crate::emc::value(OBSIDIAN) * 8 + crate::emc::value(CHEST),
            544
        );

        let condenser = SHAPED
            .iter()
            .find(|recipe| recipe.name == "energy_condenser")
            .expect("Energy Condenser recipe");
        assert_eq!(condenser.result, ENERGY_CONDENSER);
        let recipe_cost = crate::emc::value(OBSIDIAN) * 4
            + crate::emc::value(DIAMOND) * 4
            + crate::emc::value(ALCHEMICAL_CHEST);
        assert_eq!(crate::emc::value(ENERGY_CONDENSER), recipe_cost);
        assert_eq!(recipe_cost, 33_568);
    }

    #[test]
    fn files_are_deterministic() {
        assert_eq!(files(), files());
    }
}
