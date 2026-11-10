//! EMC values and the matter/fuel economy.
//!
//! Values follow ProjectE's spirit (one cobblestone is 1 EMC, a diamond is
//! 8192) but are deliberately kept in one readable table so a pack author can
//! audit them. Unknown vanilla items fall back to cheap tag-like rules.

/// The value of one item, or 0 when the item has no EMC value.
pub fn value(id: &str) -> u64 {
    if let Some(v) = table_value(id) {
        return v;
    }
    // ProjectE-style convention: a log is 32 EMC and planks are 8.
    if id.ends_with("_log") || id.ends_with("_wood") || id.ends_with("_stem")
        || id.ends_with("_hyphae")
    {
        return 32;
    }
    if id.ends_with("_planks") {
        return 8;
    }
    if id.ends_with("_sapling") || id.ends_with("_leaves") {
        return 1;
    }
    if id.ends_with("_ore") {
        return 256;
    }
    if id.ends_with("_ingot") || id.ends_with("_nugget") {
        return if id.ends_with("_nugget") { 28 } else { 256 };
    }
    if id.ends_with("_wool") || id.ends_with("_carpet") {
        return 32;
    }
    0
}

fn table_value(id: &str) -> Option<u64> {
    Some(match id {
        // Raw resources and building blocks.
        "minecraft:cobblestone" | "minecraft:stone" | "minecraft:dirt"
        | "minecraft:netherrack" | "minecraft:end_stone" | "minecraft:tuff" => 1,
        "minecraft:sand" | "minecraft:gravel" => 4,
        "minecraft:flint" => 4,
        "minecraft:clay" => 64,
        "minecraft:clay_ball" => 16,
        "minecraft:brick" => 64,
        "minecraft:glass" => 16,
        "minecraft:obsidian" => 64,
        "minecraft:nether_brick" => 16,
        "minecraft:quartz" => 256,
        "minecraft:amethyst_shard" => 32,
        "minecraft:basalt" => 4,
        "minecraft:blackstone" => 4,
        "minecraft:soul_sand" => 48,
        "minecraft:soul_soil" => 48,
        "minecraft:glowstone" => 1536,
        "minecraft:glowstone_dust" => 384,
        "minecraft:redstone" => 64,
        "minecraft:redstone_block" => 576,
        "minecraft:lapis_lazuli" => 864,
        "minecraft:lapis_block" => 7776,
        "minecraft:coal" | "minecraft:charcoal" => 32,
        "minecraft:coal_block" => 288,
        "minecraft:diamond" => 8192,
        "minecraft:diamond_block" => 73728,
        "minecraft:emerald" => 16384,
        "minecraft:emerald_block" => 147456,
        "minecraft:iron_ingot" => 256,
        "minecraft:iron_block" => 2304,
        "minecraft:raw_iron" => 256,
        "minecraft:iron_ore" | "minecraft:deepslate_iron_ore" => 384,
        "minecraft:gold_ingot" => 2048,
        "minecraft:gold_block" => 18432,
        "minecraft:raw_gold" => 2048,
        "minecraft:gold_ore" | "minecraft:deepslate_gold_ore" => 3072,
        "minecraft:copper_ingot" => 128,
        "minecraft:raw_copper" => 128,
        "minecraft:netherite_scrap" => 32768,
        "minecraft:netherite_ingot" => 139264,
        "minecraft:netherite_block" => 1253376,
        "minecraft:ancient_debris" => 32768,
        "minecraft:ender_pearl" => 1024,
        "minecraft:ender_eye" => 1792,
        "minecraft:blaze_rod" => 1536,
        "minecraft:blaze_powder" => 768,
        "minecraft:ghast_tear" => 4096,
        "minecraft:nether_star" => 139264,
        "minecraft:experience_bottle" => 768,
        "minecraft:gunpowder" => 192,
        "minecraft:string" => 12,
        "minecraft:feather" => 8,
        "minecraft:leather" => 16,
        "minecraft:bone" => 16,
        "minecraft:slime_ball" => 32,
        "minecraft:sugar" => 16,
        "minecraft:paper" => 32,
        "minecraft:book" => 96,
        "minecraft:stick" => 4,
        "minecraft:apple" => 128,
        "minecraft:bread" => 72,
        "minecraft:golden_apple" => 1856,
        "minecraft:enchanted_golden_apple" => 147456,
        "minecraft:carrot" => 64,
        "minecraft:potato" => 64,
        "minecraft:baked_potato" => 128,
        "minecraft:wheat" => 24,
        "minecraft:wheat_seeds" => 16,
        "minecraft:oak_log" | "minecraft:birch_log" | "minecraft:spruce_log"
        | "minecraft:jungle_log" | "minecraft:acacia_log" | "minecraft:dark_oak_log"
        | "minecraft:mangrove_log" | "minecraft:cherry_log" => 32,
        "minecraft:oak_planks" | "minecraft:birch_planks" | "minecraft:spruce_planks"
        | "minecraft:jungle_planks" | "minecraft:acacia_planks"
        | "minecraft:dark_oak_planks" | "minecraft:mangrove_planks"
        | "minecraft:cherry_planks" => 8,
        "minecraft:torch" => 9,
        "minecraft:chest" => 32,
        "minecraft:furnace" => 8,
        "minecraft:crafting_table" => 32,
        "minecraft:enchanting_table" => 16448,
        "minecraft:anvil" => 7936,
        "minecraft:beacon" => 147840,
        "minecraft:shulker_shell" => 2048,
        "minecraft:chorus_fruit" => 192,
        "minecraft:popped_chorus_fruit" => 192,
        "minecraft:dragon_breath" => 512,
        "minecraft:phantom_membrane" => 512,
        "minecraft:nautilus_shell" => 1024,
        "minecraft:heart_of_the_sea" => 8192,
        "minecraft:echo_shard" => 1024,
        "minecraft:disc_fragment_5" => 512,
        "minecraft:netherite_upgrade_smithing_template" => 1024,

        // ProjectE's own materials.
        "projecte:philosophers_stone" => 16384,
        "projecte:alchemical_coal" => 128,
        "projecte:mobius_fuel" => 512,
        "projecte:aeternalis_fuel" => 2048,
        "projecte:dark_matter" => 16384,
        "projecte:red_matter" => 131072,
        "projecte:klein_star_ein" => 8192,
        "projecte:klein_star_zwei" => 32768,
        "projecte:klein_star_drei" => 131072,
        "projecte:klein_star_vier" => 524288,
        "projecte:transmutation_table" => 32768,
        "projecte:energy_collector" => 16384,
        "projecte:antimatter_relay" => 65536,
        _ => return None,
    })
}

/// The burn value ProjectE's fuels carry (not used by vanilla furnaces yet;
/// exposed for future furnace hooks).
pub fn fuel_value(id: &str) -> Option<u64> {
    Some(match id {
        "projecte:alchemical_coal" => 128,
        "projecte:mobius_fuel" => 512,
        "projecte:aeternalis_fuel" => 2048,
        _ => return None,
    })
}

/// A short human-readable name for chat output.
pub fn display_name(id: &str) -> String {
    let path = id.rsplit(':').next().unwrap_or(id);
    let mut out = String::with_capacity(path.len());
    let mut upper = true;
    for c in path.chars() {
        if c == '_' {
            out.push(' ');
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values_and_fallbacks() {
        assert_eq!(value("minecraft:cobblestone"), 1);
        assert_eq!(value("minecraft:diamond"), 8192);
        assert_eq!(value("minecraft:oak_log"), 32);
        assert_eq!(value("minecraft:oak_planks"), 8);
        assert_eq!(value("minecraft:cherry_planks"), 8);
        assert_eq!(value("minecraft:iron_ore"), 384);
        assert_eq!(value("minecraft:unknown"), 0);
    }

    #[test]
    fn display_names() {
        assert_eq!(display_name("projecte:dark_matter"), "Dark Matter");
        assert_eq!(display_name("minecraft:glowstone_dust"), "Glowstone Dust");
    }
}
