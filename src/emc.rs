//! EMC values and the matter/fuel economy.
//!
//! Values follow ProjectE's spirit (one cobblestone is 1 EMC, a diamond is
//! 8192) but are deliberately kept in one readable table so a pack author can
//! audit them. Unknown vanilla items fall back to cheap tag-like rules, and
//! only inside the `minecraft` namespace.
//!
//! **Known limitation:** this is a simplified, hand-audited table, not the
//! graph ProjectE computes from its fixed seeds plus every recipe at world
//! load (its 1.21 reference gives, for example, coal 128, Philosopher's Stone
//! 9984 and Dark Matter 1 816 576). Those numbers are not hardcoded here: a
//! pack can supply them through the `projecte:emc` data map, which
//! [`value_mapped`] reads before falling back to the table.
//!
//! Fuel values are ProjectE's ratios on this table's coal value: coal is 32,
//! Alchemical Coal 128, Mobius Fuel 512 and Aeternalis Fuel 2048, so each
//! tier is four times the last and the four-to-one crafting step in
//! `recipes.rs` moves no EMC. Fuel blocks are nine fuels. Machine upgrade
//! tiers are four times the tier below them; those prices are provisional and
//! are listed for review in `PROGRESSION.md`.
//!
//! `furnace_burn` is listen-only at the pinned revision, so no burn
//! time is registered from here; `alchemical::burn_ticks` carries the intended
//! numbers as metadata.
//!
//! Runtime values may be overridden by the `projecte:emc` data map on
//! `minecraft:item` ([`value_mapped`]). The static table remains the documented
//! default; a pack that wants ProjectE's computed values supplies them there.

use std::collections::BTreeSet;

use mod_api::{DataMap, DataMapError, ItemKey, Tag, data_map_value};

/// Id of the data map that carries item EMC overrides.
pub const EMC_MAP_ID: &str = "projecte:emc";

/// The typed `projecte:emc` data map. The registration hook pushes
/// [`DataMap::ty`]; reads go through [`value_mapped`].
pub struct EmcMap;

impl DataMap for EmcMap {
    type Entry = ItemKey;
    type Value = u64;

    fn id(&self) -> &str {
        EMC_MAP_ID
    }

    fn decode(&self, value: &Tag) -> Option<Self::Value> {
        decode_emc_tag(value)
    }
}

/// Registerable description of the EMC data map.
pub fn emc_data_map_type() -> mod_api::DataMapType {
    EmcMap.ty()
}

fn decode_emc_tag(tag: &Tag) -> Option<u64> {
    match tag {
        Tag::Int(value) => u64::try_from(*value).ok(),
        Tag::Long(value) => u64::try_from(*value).ok(),
        // A fractional or structured value is not an EMC amount. Refusing it
        // here prevents a malformed pack entry from silently becoming zero.
        _ => None,
    }
}

/// Resolve an item's EMC through the server's merged `projecte:emc` data map.
///
/// An unmapped item falls back to the static table; a load error or a value
/// that is not a non-negative integer is an explicit error so the caller can
/// warn and keep the static default deliberately rather than by accident.
pub fn value_mapped(
    host: &dyn mod_api::Host,
    id: &str,
) -> Result<u64, DataMapError> {
    match data_map_value(host, "minecraft:item", id, EMC_MAP_ID)? {
        Some(tag) => decode_emc_tag(&tag).ok_or(DataMapError::Undecodable),
        None => Ok(value(id)),
    }
}

/// Whether an item id is excluded from learning.
///
/// ProjectE gates learning on `projecte:blacklist_learning` (empty by
/// default). Horizon's tag read is left to the caller, so this stays a pure
/// exact-id check; tag expansion must happen at the boundary where the server
/// can resolve tags. An id on the list is silently ignored by `learn`, never
/// reported as an error.
pub fn is_learn_blacklisted(id: &str, blacklist: &BTreeSet<String>) -> bool {
    blacklist.contains(id)
}

/// Durability of a damaged stack: remaining out of maximum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Durability {
    pub remaining: u32,
    pub maximum: u32,
}

/// The data components that can change an item's EMC.
///
/// Mirrors ProjectE's processor chain (MIT, `github.com/sinkillerj/ProjectE`):
/// `DamageProcessor`, `EnchantmentProcessor` and `StoredEMCProcessor`. An
/// exact component-bearing mapping, when present, wins over all processors.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemComponents {
    pub durability: Option<Durability>,
    /// (enchantment id, level, rarity weight). A zero weight is invalid.
    pub enchantments: Vec<(String, u32, u32)>,
    /// Charge held by an EMC-storage item. Negative is invalid.
    pub stored_emc: i64,
    /// Explicit value for this exact component patch.
    pub exact_value: Option<u64>,
}

/// Which component processors are enabled. ProjectE disables enchantments by
/// default with a bonus of 5000 per level before rarity weighting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentConfig {
    pub enchantments_enabled: bool,
    pub enchantment_bonus: u64,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            enchantments_enabled: false,
            enchantment_bonus: 5_000,
        }
    }
}

/// Resolve the EMC of a stack whose components differ from the base item.
///
/// Fail closed: an invalid component state (zero durability, negative stored
/// EMC, zero rarity weight) or any overflow returns 0, matching ProjectE's
/// `ArithmeticException` handling. No value is ever invented for a broken
/// stack.
pub fn value_with_components(
    base: u64,
    components: &ItemComponents,
    config: ComponentConfig,
) -> u64 {
    if let Some(exact) = components.exact_value {
        return exact;
    }
    if base == 0 {
        return 0;
    }
    let mut current = base;
    if let Some(durability) = components.durability {
        if durability.maximum == 0 || durability.remaining == 0 {
            return 0;
        }
        if durability.remaining < durability.maximum {
            let scaled = u128::from(current) * u128::from(durability.remaining)
                / u128::from(durability.maximum);
            match u64::try_from(scaled) {
                Ok(value) => current = value,
                Err(_) => return 0,
            }
        }
    }
    if config.enchantments_enabled {
        for (_, level, weight) in &components.enchantments {
            if *weight == 0 {
                return 0;
            }
            let per_level = config.enchantment_bonus / u64::from(*weight);
            let added = u128::from(per_level) * u128::from(*level);
            match u64::try_from(u128::from(current) + added) {
                Ok(value) => current = value,
                Err(_) => return 0,
            }
        }
    }
    if components.stored_emc != 0 {
        let Ok(stored) = u64::try_from(components.stored_emc) else {
            return 0;
        };
        match current.checked_add(stored) {
            Some(value) => current = value,
            None => return 0,
        }
    }
    current
}

/// The value of one item, or 0 when the item has no EMC value.
pub fn value(id: &str) -> u64 {
    if let Some(v) = table_value(id) {
        return v;
    }
    // The suffix rules below model vanilla item families. Explicit table
    // entries (including ProjectE's own materials) were handled above, so an
    // unknown item from any other namespace has no EMC value: a modded
    // `_ingot` is not automatically worth an iron ingot.
    if !id.starts_with("minecraft:") {
        return 0;
    }
    // ProjectE-style convention: a log is 32 EMC and planks are 8.
    if id.ends_with("_log")
        || id.ends_with("_wood")
        || id.ends_with("_stem")
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

/// Calculates the EMC cost of `count` items at `value` EMC each.
///
/// Returns `None` when the cost cannot be represented as an `i64` (the type
/// used for a player's EMC balance): either the multiplication overflows
/// `u64` or the product exceeds `i64::MAX`. Callers must reject the
/// transaction in that case instead of wrapping the result.
///
/// A count of zero is defined to cost `Some(0)` for any value, including
/// `u64::MAX`: zero items cost nothing and cannot overflow. Callers do not
/// rely on this to grant items for free — purchases always request at least
/// one item, and a zero `given` count is rejected before this is called for
/// the amount actually spent.
pub fn checked_cost(value: u64, count: u32) -> Option<i64> {
    let total = value.checked_mul(u64::from(count))?;
    i64::try_from(total).ok()
}

fn table_value(id: &str) -> Option<u64> {
    Some(match id {
        // Raw resources and building blocks.
        "minecraft:cobblestone"
        | "minecraft:stone"
        | "minecraft:dirt"
        | "minecraft:netherrack"
        | "minecraft:end_stone"
        | "minecraft:tuff" => 1,
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
        "minecraft:oak_log"
        | "minecraft:birch_log"
        | "minecraft:spruce_log"
        | "minecraft:jungle_log"
        | "minecraft:acacia_log"
        | "minecraft:dark_oak_log"
        | "minecraft:mangrove_log"
        | "minecraft:cherry_log" => 32,
        "minecraft:oak_planks"
        | "minecraft:birch_planks"
        | "minecraft:spruce_planks"
        | "minecraft:jungle_planks"
        | "minecraft:acacia_planks"
        | "minecraft:dark_oak_planks"
        | "minecraft:mangrove_planks"
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
        // The two highest tiers were registered but had no EMC value at all.
        // Keep the fourfold upgrade chain of the lower tiers as the default;
        // a data pack can supply ProjectE's own computed numbers instead.
        "projecte:klein_star_sphere" => 2_097_152,
        "projecte:klein_star_omega" => 8_388_608,
        // Matter blocks are four matter per block in ProjectE; nine would be
        // the wrong ratio, so use four times the matter value here.
        "projecte:dark_matter_block" => 65_536,
        "projecte:red_matter_block" => 524_288,
        "projecte:transmutation_table" => 32768,
        // Covalence dusts are priced as 1/40 of their ingredients, so the
        // real Alchemical Chest recipe is EMC-neutral at this table's values.
        "projecte:low_covalence_dust" => 1,
        "projecte:medium_covalence_dust" => 8,
        "projecte:high_covalence_dust" => 205,
        "projecte:alchemical_chest" => 8_952,
        "projecte:condenser_mk1" => 41_976,
        // Values derived from the recipes in recipes.rs, using this table's
        // base values (the data map below carries ProjectE's own numbers).
        "minecraft:barrel" => 64,
        "minecraft:tnt" => 976,
        "projecte:condenser_mk2" => 2_401_272,
        "projecte:magenta_matter" => 405_504,
        "projecte:magenta_matter_block" => 1_622_016,
        "projecte:condenser_mk3" => 18_494_424,
        "minecraft:water_bucket" => 768,
        "minecraft:lava_bucket" => 768,
        "minecraft:flint_and_steel" => 260,
        "minecraft:clock" => 8256,
        "minecraft:bricks" => 256,
        "minecraft:snowball" => 4,
        "minecraft:snow_block" => 16,
        "minecraft:oak_sapling" => 16,
        "minecraft:dandelion" => 1,
        "projecte:iron_band" => 2_816,
        "projecte:volcanite_amulet" => 53_760,
        "projecte:evertide_amulet" => 53_760,
        "projecte:black_hole_band" => 35_656,
        "projecte:zero_ring" => 35_656,
        "projecte:harvest_goddess_band" => 35_650,
        "projecte:ignition_ring" => 37_648,
        "projecte:swiftwolf_rending_gale" => 68_384,
        "projecte:arcana_ring" => 832_698,
        "projecte:gem_of_eternal_density" => 73_856,
        "projecte:body_stone" => 263_104,
        "projecte:soul_stone" => 265_312,
        "projecte:mind_stone" => 263_584,
        "projecte:life_stone" => 528_416,
        "projecte:destruction_catalyst" => 5_284,
        "projecte:hyperkinetic_lens" => 82_664,
        "projecte:catalytic_lens" => 202_636,
        "projecte:mercurial_eye" => 140_672,
        "projecte:watch_of_flowing_time" => 76_992,
        "projecte:dark_matter_shield" => 65_536,
        "projecte:red_matter_shield" => 278_528,
        "projecte:void_ring" => 371_656,
        "minecraft:trident" => 16_398,
        "minecraft:bow" => 44,
        "minecraft:cooked_beef" => 64,
        "projecte:dark_matter_trident" => 65_550,
        "projecte:rm_leggings" => 1_032_192,
        "projecte:red_matter_trident" => 360_462,
        "projecte:archangel_smite" => 35_776,
        "projecte:gem_boots" => 9_115_200,
        "projecte:gem_chestplate" => 9_885_120,
        "projecte:gem_helmet" => 9_444_960,
        "projecte:gem_leggings" => 9_533_448,
        "projecte:basic_alchemical_book" => 280_468,
        "projecte:infinite_fuel" => 295_172,
        "projecte:infinite_steak" => 295_428,
        "projecte:dm_axe" => 65_536,
        "projecte:dm_boots" => 65_536,
        "projecte:dm_chestplate" => 131_072,
        "projecte:dm_hammer" => 57_344,
        "projecte:dm_helmet" => 81_920,
        "projecte:dm_hoe" => 49_152,
        "projecte:dm_leggings" => 114_688,
        "projecte:dm_pick" => 65_536,
        "projecte:dm_shears" => 24_576,
        "projecte:dm_shovel" => 32_768,
        "projecte:dm_sword" => 40_960,
        "projecte:rm_axe" => 475_136,
        "projecte:rm_boots" => 589_824,
        "projecte:rm_chestplate" => 1_179_648,
        "projecte:rm_hammer" => 352_256,
        "projecte:rm_helmet" => 737_280,
        "projecte:rm_pick" => 475_136,
        "projecte:rm_shears" => 155_648,
        "projecte:rm_shovel" => 180_224,
        "projecte:rm_sword" => 303_104,
        "projecte:transmutation_tablet" => 294_916,
        "projecte:dm_pedestal" => 851_968,
        "projecte:dm_furnace" => 524_296,
        "projecte:rm_furnace" => 2_097_160,
        "projecte:alchemical_barrel" => 8_984,
        "projecte:repair_talisman" => 484,
        "projecte:nova_catalyst" => 744,
        "projecte:nova_cataclysm" => 1_396,
        "projecte:divining_rod_1" => 12,
        "projecte:divining_rod_2" => 76,
        "projecte:divining_rod_3" => 1_716,
        "projecte:alchemical_coal_block" => 1152,
        "projecte:mobius_fuel_block" => 4608,
        "projecte:aeternalis_fuel_block" => 18432,
        "projecte:collector_mk1" => 16384,
        "projecte:collector_mk2" => 65536,
        "projecte:collector_mk3" => 262144,
        "projecte:relay_mk1" => 65536,
        "projecte:relay_mk2" => 262144,
        "projecte:relay_mk3" => 1048576,
        _ => return None,
    })
}

/// Canonical ProjectE values shipped as the default `projecte:emc` data map.
///
/// Adapted from the MIT-licensed ProjectE source (upstream
/// `github.com/sinkillerj/ProjectE`): fixed seeds from
/// `pe_custom_conversions/defaults.json` plus values its recipe graph derives.
/// Only ids this crate can register are listed. A pack may override any entry
/// with a later data-map file; [`value_mapped`] reads the merged result.
///
/// These numbers are deliberately consistent with the recipes in
/// `recipes.rs`, so crafting them does not mint or destroy EMC.
const CANONICAL_VALUES: &[(&str, u64)] = &[
    // Fixed vanilla seeds and their direct peers.
    ("minecraft:cobblestone", 1),
    ("minecraft:dirt", 1),
    ("minecraft:sand", 1),
    ("minecraft:end_stone", 1),
    ("minecraft:netherrack", 1),
    ("minecraft:gravel", 4),
    ("minecraft:obsidian", 64),
    ("minecraft:crying_obsidian", 768),
    ("minecraft:apple", 128),
    ("minecraft:coal", 128),
    // Charcoal is reached from a 32-EMC log, so the graph keeps it at 32.
    ("minecraft:charcoal", 32),
    ("minecraft:redstone", 64),
    ("minecraft:glowstone_dust", 384),
    ("minecraft:diamond", 8192),
    ("minecraft:emerald", 16384),
    ("minecraft:feather", 48),
    ("minecraft:string", 12),
    ("minecraft:flint", 4),
    ("minecraft:wheat", 24),
    ("minecraft:wheat_seeds", 16),
    ("minecraft:gunpowder", 192),
    ("minecraft:lapis_lazuli", 864),
    ("minecraft:blaze_rod", 1536),
    ("minecraft:ender_pearl", 1024),
    ("minecraft:ghast_tear", 4096),
    ("minecraft:nether_star", 139264),
    ("minecraft:netherite_scrap", 12288),
    ("minecraft:ancient_debris", 12288),
    ("minecraft:netherite_ingot", 57344),
    ("minecraft:iron_ingot", 256),
    ("minecraft:gold_ingot", 2048),
    ("minecraft:heart_of_the_sea", 32768),
    // Nine-to-one storage blocks and half-value dusts follow the same graph.
    ("minecraft:coal_block", 1_152),
    ("minecraft:redstone_block", 576),
    ("minecraft:glowstone", 3_456),
    ("minecraft:lapis_block", 7_776),
    ("minecraft:diamond_block", 73_728),
    ("minecraft:emerald_block", 147_456),
    ("minecraft:iron_block", 2_304),
    ("minecraft:gold_block", 18_432),
    ("minecraft:netherite_block", 516_096),
    ("minecraft:blaze_powder", 768),
    // Other fixed seeds from ProjectE's custom-conversion defaults.
    ("minecraft:sugar_cane", 32),
    ("minecraft:bamboo", 32),
    ("minecraft:pumpkin", 144),
    ("minecraft:soul_sand", 49),
    ("minecraft:magma_block", 128),
    ("minecraft:sponge", 128),
    ("minecraft:trident", 16_398),
    ("minecraft:dragon_egg", 262_144),
    // ProjectE materials, derived from the same recipes this crate ships.
    ("projecte:philosophers_stone", 9_984),
    ("projecte:low_covalence_dust", 1),
    ("projecte:medium_covalence_dust", 8),
    ("projecte:high_covalence_dust", 208),
    ("projecte:alchemical_chest", 8_955),
    ("projecte:condenser_mk1", 41_979),
    // Expansion/core machines and tools derived with ProjectE's graph.
    ("minecraft:barrel", 64),
    ("minecraft:tnt", 964),
    ("projecte:condenser_mk2", 137_217_019),
    ("projecte:magenta_matter", 21_577_728),
    ("projecte:magenta_matter_block", 86_310_912),
    ("projecte:condenser_mk3", 1_031_328_743),
    ("minecraft:water_bucket", 768),
    ("minecraft:lava_bucket", 768),
    ("minecraft:flint_and_steel", 260),
    ("minecraft:clock", 8_256),
    ("minecraft:bricks", 256),
    ("minecraft:snowball", 4),
    ("minecraft:snow_block", 16),
    ("minecraft:oak_sapling", 16),
    ("minecraft:dandelion", 1),
    ("projecte:iron_band", 2_816),
    ("projecte:volcanite_amulet", 5_454_336),
    ("projecte:evertide_amulet", 5_454_336),
    ("projecte:black_hole_band", 3_636_040),
    ("projecte:zero_ring", 3_636_040),
    ("projecte:harvest_goddess_band", 3_636_034),
    ("projecte:ignition_ring", 3_740_944),
    ("projecte:swiftwolf_rending_gale", 7_269_312),
    ("projecte:arcana_ring", 52_066_650),
    ("projecte:gem_of_eternal_density", 3_674_240),
    ("projecte:body_stone", 13_514_688),
    ("projecte:soul_stone", 13_516_896),
    ("projecte:mind_stone", 13_515_168),
    ("projecte:life_stone", 27_031_584),
    ("projecte:destruction_catalyst", 313_996),
    ("projecte:hyperkinetic_lens", 3_708_770),
    ("projecte:catalytic_lens", 16_738_798),
    ("projecte:mercurial_eye", 6_766_464),
    ("projecte:watch_of_flowing_time", 7_281_600),
    ("projecte:dark_matter_shield", 1_865_728),
    ("projecte:red_matter_shield", 17_705_472),
    ("projecte:void_ring", 20_823_008),
    ("projecte:dark_matter_trident", 3_665_934),
    ("projecte:rm_leggings", 60_014_080),
    ("projecte:red_matter_trident", 20_812_814),
    ("projecte:archangel_smite", 3_636_240),
    ("projecte:gem_boots", 482_942_848),
    ("projecte:gem_chestplate", 521_667_008),
    ("projecte:gem_helmet", 495_948_896),
    ("projecte:gem_leggings", 505_042_184),
    ("projecte:basic_alchemical_book", 13_532_064),
    ("projecte:infinite_fuel", 29_099_012),
    ("projecte:infinite_steak", 29_098_500),
    ("projecte:dm_axe", 5_466_112),
    ("projecte:dm_boots", 7_266_304),
    ("projecte:dm_chestplate", 14_532_608),
    ("projecte:dm_hammer", 3_657_728),
    ("projecte:dm_helmet", 9_082_880),
    ("projecte:dm_hoe", 3_649_536),
    ("projecte:dm_leggings", 12_716_032),
    ("projecte:dm_pick", 5_466_112),
    ("projecte:dm_shears", 1_824_768),
    ("projecte:dm_shovel", 1_832_960),
    ("projecte:dm_sword", 3_641_344),
    ("projecte:rm_axe", 27_553_280),
    ("projecte:rm_boots", 34_293_760),
    ("projecte:rm_chestplate", 68_587_520),
    ("projecte:rm_hammer", 20_804_608),
    ("projecte:rm_helmet", 42_867_200),
    ("projecte:rm_pick", 27_553_280),
    ("projecte:rm_shears", 8_581_632),
    ("projecte:rm_shovel", 10_406_400),
    ("projecte:rm_sword", 17_155_072),
    ("projecte:transmutation_tablet", 29_097_988),
    ("projecte:dm_pedestal", 63_358_976),
    ("projecte:dm_furnace", 58_130_440),
    ("projecte:rm_furnace", 139_212_808),
    ("projecte:alchemical_barrel", 8_987),
    ("projecte:repair_talisman", 490),
    ("projecte:nova_catalyst", 26_466),
    ("projecte:nova_cataclysm", 122_161),
    ("projecte:divining_rod_1", 12),
    ("projecte:divining_rod_2", 76),
    ("projecte:divining_rod_3", 1_740),
    ("projecte:alchemical_coal", 10_496),
    ("projecte:mobius_fuel", 51_968),
    ("projecte:aeternalis_fuel", 217_856),
    ("projecte:dark_matter", 1_816_576),
    ("projecte:red_matter", 6_756_864),
    ("projecte:alchemical_coal_block", 94_464),
    ("projecte:mobius_fuel_block", 467_712),
    ("projecte:aeternalis_fuel_block", 1_960_704),
    ("projecte:klein_star_ein", 423_936),
    ("projecte:klein_star_zwei", 1_695_744),
    ("projecte:klein_star_drei", 6_782_976),
    ("projecte:klein_star_vier", 27_131_904),
    ("projecte:klein_star_sphere", 108_527_616),
    ("projecte:klein_star_omega", 434_110_464),
    ("projecte:dark_matter_block", 7_266_304),
    ("projecte:red_matter_block", 27_027_456),
];

/// The default `projecte:emc` data-map file, keyed as `data_pack_files` expects.
///
/// The map is registered by the hook layer; shipping this file is what makes
/// [`value_mapped`] return ProjectE's numbers instead of the static fallback.
pub fn data_pack_files() -> Vec<(String, Vec<u8>)> {
    let values = CANONICAL_VALUES
        .iter()
        .map(|(id, amount)| format!("\"{id}\":{amount}"))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!("{{\"replace\":false,\"values\":{{{values}}}}}");
    vec![("data_maps/projecte:item/emc".to_owned(), json.into_bytes())]
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
    fn non_minecraft_suffix_fallback_is_zero() {
        assert_eq!(value("example:iron_ingot"), 0);
        assert_eq!(value("example:deepslate_ore"), 0);
        assert_eq!(value("example:oak_log"), 0);
        assert_eq!(value("example:white_wool"), 0);
        assert_eq!(value("example:crimson_planks"), 0);
        assert_eq!(value("example:iron_nugget"), 0);
        assert_eq!(value("projecte:oak_log"), 0);
    }

    #[test]
    fn vanilla_suffix_fallback_still_works() {
        // Deliberately absent from the table: only the suffix rules apply.
        assert_eq!(value("minecraft:crimson_log"), 32);
        assert_eq!(value("minecraft:crimson_stem"), 32);
        assert_eq!(value("minecraft:crimson_planks"), 8);
        assert_eq!(value("minecraft:oak_leaves"), 1);
        assert_eq!(value("minecraft:coal_ore"), 256);
        assert_eq!(value("minecraft:iron_nugget"), 28);
        assert_eq!(value("minecraft:white_carpet"), 32);
    }

    #[test]
    fn explicit_entries_take_precedence_over_fallback() {
        assert_eq!(value("minecraft:iron_ore"), 384);
        assert_eq!(value("minecraft:copper_ingot"), 128);
        assert_eq!(value("minecraft:netherite_ingot"), 139264);
        assert_eq!(value("projecte:dark_matter"), 16384);
        assert_eq!(value("projecte:red_matter"), 131072);
        assert_eq!(value("projecte:alchemical_chest"), 8_952);
        assert_eq!(value("projecte:condenser_mk1"), 41_976);
        assert_eq!(value("projecte:low_covalence_dust"), 1);
        assert_eq!(value("projecte:medium_covalence_dust"), 8);
        assert_eq!(value("projecte:high_covalence_dust"), 205);
    }

    #[test]
    fn display_names() {
        assert_eq!(display_name("projecte:dark_matter"), "Dark Matter");
        assert_eq!(display_name("minecraft:glowstone_dust"), "Glowstone Dust");
    }

    #[test]
    fn checked_cost_ordinary() {
        assert_eq!(checked_cost(1, 1), Some(1));
        assert_eq!(checked_cost(8192, 1), Some(8192));
        assert_eq!(checked_cost(8192, 64), Some(524_288));
        assert_eq!(checked_cost(32, 16), Some(512));
        assert_eq!(checked_cost(value("minecraft:diamond"), 16), Some(131_072));
        assert_eq!(checked_cost(value("minecraft:cobblestone"), 64), Some(64));
        assert_eq!(checked_cost(0, 64), Some(0));
    }

    #[test]
    fn checked_cost_largest_representable() {
        // The largest cost that still fits in an i64.
        assert_eq!(checked_cost(i64::MAX as u64, 1), Some(i64::MAX));
        // Largest even product below i64::MAX: (MAX / 2) * 2 == MAX - 1.
        assert_eq!(checked_cost((i64::MAX as u64) / 2, 2), Some(i64::MAX - 1));
        assert_eq!(checked_cost(1, u32::MAX), Some(i64::from(u32::MAX)));
    }

    #[test]
    fn checked_cost_multiplication_overflow() {
        // Products that overflow u64 itself.
        assert_eq!(checked_cost(u64::MAX, 2), None);
        assert_eq!(checked_cost(u64::MAX, u32::MAX), None);
        assert_eq!(checked_cost(1u64 << 63, 2), None);
        assert_eq!(checked_cost(u64::MAX / 2 + 1, 2), None);
    }

    #[test]
    fn checked_cost_conversion_overflow_beyond_i64_max() {
        // Products that fit in u64 but exceed i64::MAX.
        assert_eq!(checked_cost(i64::MAX as u64 + 1, 1), None);
        assert_eq!(checked_cost(u64::MAX, 1), None);
        // (i64::MAX) * 2 == 2^64 - 2, which fits in u64 but not in i64.
        assert_eq!(checked_cost(i64::MAX as u64, 2), None);
        // ((i64::MAX / 2) + 1) * 2 == i64::MAX + 1, which fits in u64.
        assert_eq!(checked_cost((i64::MAX as u64) / 2 + 1, 2), None);
    }

    #[test]
    fn checked_cost_zero_count() {
        // Zero count is defined as Some(0) for any value: zero items cost
        // nothing and cannot overflow, even at u64::MAX.
        assert_eq!(checked_cost(8192, 0), Some(0));
        assert_eq!(checked_cost(0, 0), Some(0));
        assert_eq!(checked_cost(u64::MAX, 0), Some(0));
        assert_eq!(checked_cost(i64::MAX as u64, 0), Some(0));
    }

    #[test]
    fn data_map_type_targets_the_item_registry() {
        let ty = emc_data_map_type();
        assert_eq!(ty.id, EMC_MAP_ID);
        assert_eq!(ty.registry, "minecraft:item");
        assert_eq!(EmcMap.id(), EMC_MAP_ID);
    }

    #[test]
    fn data_map_values_accept_only_non_negative_integers() {
        assert_eq!(decode_emc_tag(&Tag::Int(8192)), Some(8192));
        assert_eq!(decode_emc_tag(&Tag::Long(9_984)), Some(9_984));
        assert_eq!(decode_emc_tag(&Tag::Int(0)), Some(0));
        assert_eq!(decode_emc_tag(&Tag::Int(-1)), None);
        assert_eq!(decode_emc_tag(&Tag::Long(-1)), None);
        assert_eq!(decode_emc_tag(&Tag::String("8192".into())), None);
        assert_eq!(decode_emc_tag(&Tag::Double(8192)), None);
    }

    #[test]
    fn learning_blacklist_is_an_exact_id_check() {
        let blacklist: BTreeSet<String> = ["minecraft:diamond".to_string()].into_iter().collect();
        assert!(is_learn_blacklisted("minecraft:diamond", &blacklist));
        assert!(!is_learn_blacklisted("minecraft:iron_ingot", &blacklist));
        assert!(!is_learn_blacklisted("diamond", &blacklist));
        assert!(blacklist.is_empty() == false);
    }

    #[test]
    fn components_without_processors_leave_the_base_value() {
        let config = ComponentConfig::default();
        assert_eq!(value_with_components(8192, &ItemComponents::default(), config), 8192);
        assert_eq!(value_with_components(0, &ItemComponents::default(), config), 0);
    }

    #[test]
    fn damage_scales_value_and_fails_closed_at_zero_durability() {
        let config = ComponentConfig::default();
        let components = ItemComponents {
            durability: Some(Durability {
                remaining: 1,
                maximum: 2,
            }),
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(8192, &components, config), 4096);

        let broken = ItemComponents {
            durability: Some(Durability {
                remaining: 0,
                maximum: 100,
            }),
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(8192, &broken, config), 0);

        let malformed = ItemComponents {
            durability: Some(Durability {
                remaining: 5,
                maximum: 0,
            }),
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(8192, &malformed, config), 0);

        let undamaged = ItemComponents {
            durability: Some(Durability {
                remaining: 100,
                maximum: 100,
            }),
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(8192, &undamaged, config), 8192);
    }

    #[test]
    fn stored_emc_adds_to_value_and_rejects_negatives_and_overflow() {
        let config = ComponentConfig::default();
        let charged = ItemComponents {
            stored_emc: 1000,
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(8192, &charged, config), 9192);

        let negative = ItemComponents {
            stored_emc: -1,
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(8192, &negative, config), 0);

        let overflow = ItemComponents {
            stored_emc: 1,
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(u64::MAX, &overflow, config), 0);
    }

    #[test]
    fn enchantments_are_disabled_by_default_and_use_rarity_weights_when_on() {
        let components = ItemComponents {
            enchantments: vec![("minecraft:sharpness".to_string(), 2, 2)],
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(100, &components, ComponentConfig::default()), 100);

        let enabled = ComponentConfig {
            enchantments_enabled: true,
            enchantment_bonus: 5000,
        };
        // (5000 / 2) * 2 levels == 5000.
        assert_eq!(value_with_components(100, &components, enabled), 5100);

        let invalid_weight = ItemComponents {
            enchantments: vec![("test:bad".to_string(), 1, 0)],
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(100, &invalid_weight, enabled), 0);
    }

    #[test]
    fn exact_component_mapping_overrides_processors() {
        let config = ComponentConfig {
            enchantments_enabled: true,
            enchantment_bonus: 5000,
        };
        let components = ItemComponents {
            durability: Some(Durability {
                remaining: 0,
                maximum: 100,
            }),
            stored_emc: -5,
            exact_value: Some(777),
            ..ItemComponents::default()
        };
        assert_eq!(value_with_components(8192, &components, config), 777);
    }

    #[test]
    fn damage_scaling_never_overflows() {
        let components = ItemComponents {
            durability: Some(Durability {
                remaining: u32::MAX,
                maximum: u32::MAX - 1,
            }),
            ..ItemComponents::default()
        };
        // remaining > maximum leaves the value untouched (not "damaged").
        assert_eq!(value_with_components(u64::MAX, &components, ComponentConfig::default()), u64::MAX);
    }

    #[test]
    fn canonical_storage_blocks_are_nine_times_their_items() {
        let canonical = |id: &str| {
            CANONICAL_VALUES
                .iter()
                .find(|(item, _)| *item == id)
                .map(|(_, amount)| *amount)
                .unwrap_or_else(|| panic!("{id} has no canonical value"))
        };
        for (item, block) in [
            ("minecraft:coal", "minecraft:coal_block"),
            ("minecraft:redstone", "minecraft:redstone_block"),
            ("minecraft:diamond", "minecraft:diamond_block"),
            ("minecraft:emerald", "minecraft:emerald_block"),
            ("minecraft:iron_ingot", "minecraft:iron_block"),
            ("minecraft:gold_ingot", "minecraft:gold_block"),
            ("minecraft:netherite_ingot", "minecraft:netherite_block"),
        ] {
            assert_eq!(canonical(block), 9 * canonical(item), "{item} -> {block}");
        }
        assert_eq!(canonical("minecraft:glowstone"), 9 * canonical("minecraft:glowstone_dust"));
    }

    #[test]
    fn canonical_values_match_the_reference_examples() {
        let canonical = |id: &str| {
            CANONICAL_VALUES
                .iter()
                .find(|(item, _)| *item == id)
                .map(|(_, amount)| *amount)
        };
        assert_eq!(canonical("minecraft:coal"), Some(128));
        assert_eq!(canonical("projecte:philosophers_stone"), Some(9_984));
        assert_eq!(canonical("projecte:dark_matter"), Some(1_816_576));
        assert_eq!(canonical("projecte:red_matter"), Some(6_756_864));
    }

    #[test]
    fn canonical_values_are_unique_valid_ids_and_fit_the_balance() {
        let mut seen = std::collections::BTreeSet::new();
        for (id, amount) in CANONICAL_VALUES {
            assert!(seen.insert(*id), "duplicate data-map id {id}");
            assert!(id.contains(':'), "{id} must be a fully qualified id");
            assert!(i64::try_from(*amount).is_ok(), "{id} exceeds i64");
        }
    }

    #[test]
    fn canonical_values_are_emc_neutral_with_the_shipped_recipes() {
        let canonical = |id: &str| {
            CANONICAL_VALUES
                .iter()
                .find(|(item, _)| *item == id)
                .map(|(_, amount)| *amount)
                .unwrap_or_else(|| panic!("{id} has no canonical value"))
        };
        // 4 coal + the Philosopher's Stone catalyst is one Alchemical Coal.
        assert_eq!(
            canonical("projecte:alchemical_coal"),
            4 * canonical("minecraft:coal") + canonical("projecte:philosophers_stone")
        );
        // 8 Aeternalis Fuel + one diamond block is one Dark Matter.
        assert_eq!(
            canonical("projecte:dark_matter"),
            8 * canonical("projecte:aeternalis_fuel") + 9 * canonical("minecraft:diamond")
        );
        // 6 Aeternalis Fuel + 3 Dark Matter is one Red Matter.
        assert_eq!(
            canonical("projecte:red_matter"),
            6 * canonical("projecte:aeternalis_fuel") + 3 * canonical("projecte:dark_matter")
        );
        // The Klein Star and matter chains are fourfold.
        for (lower, upper) in [
            ("projecte:klein_star_ein", "projecte:klein_star_zwei"),
            ("projecte:klein_star_zwei", "projecte:klein_star_drei"),
            ("projecte:klein_star_drei", "projecte:klein_star_vier"),
            ("projecte:klein_star_vier", "projecte:klein_star_sphere"),
            ("projecte:klein_star_sphere", "projecte:klein_star_omega"),
            ("projecte:dark_matter", "projecte:dark_matter_block"),
            ("projecte:red_matter", "projecte:red_matter_block"),
        ] {
            assert_eq!(canonical(upper), 4 * canonical(lower), "{lower} -> {upper}");
        }
    }

    #[test]
    fn data_map_file_is_well_formed_and_complete() {
        let files = data_pack_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "data_maps/projecte:item/emc");
        let text = std::str::from_utf8(&files[0].1).unwrap();
        let json: mod_api::serde_json::Value = mod_api::serde_json::from_str(text).unwrap();
        assert_eq!(json["replace"], false);
        let values = json["values"].as_object().expect("a values object");
        assert_eq!(values.len(), CANONICAL_VALUES.len());
        for (id, amount) in CANONICAL_VALUES {
            assert_eq!(values[*id].as_u64(), Some(*amount), "{id}");
        }
        assert_eq!(data_pack_files(), files);
    }

    #[test]
    fn previously_unvalued_items_have_defaults() {
        assert_eq!(value("projecte:klein_star_sphere"), 2_097_152);
        assert_eq!(value("projecte:klein_star_omega"), 8_388_608);
        assert_eq!(value("projecte:dark_matter_block"), 65_536);
        assert_eq!(value("projecte:red_matter_block"), 524_288);
        for (lower, upper) in [
            ("projecte:klein_star_ein", "projecte:klein_star_zwei"),
            ("projecte:klein_star_zwei", "projecte:klein_star_drei"),
            ("projecte:klein_star_drei", "projecte:klein_star_vier"),
            ("projecte:klein_star_vier", "projecte:klein_star_sphere"),
            ("projecte:klein_star_sphere", "projecte:klein_star_omega"),
        ] {
            assert_eq!(value(upper), value(lower) * 4, "{lower} -> {upper}");
        }
    }
}
