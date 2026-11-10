//! The alchemical progression: fuels, fuel blocks and the Philosopher's
//! Stone's world transmutation.
//!
//! Everything here is pure data and pure functions so the rules can be tested
//! without a running game. The hooks in `lib.rs` only translate between these
//! rules and the Horizon `Host`; they never invent a rule of their own.
//!
//! Balance note: the fuel values in [`crate::emc`] are the canonical ProjectE
//! ratios scaled to this project's coal value (32 instead of 128), so each
//! tier is exactly four times the previous one and the four-to-one crafting
//! step is EMC-neutral. See `PROGRESSION.md`.

use crate::emc;

pub(crate) const PHILOSOPHERS_STONE: &str = "projecte:philosophers_stone";
pub(crate) const COAL: &str = "minecraft:coal";
pub(crate) const ALCHEMICAL_COAL: &str = "projecte:alchemical_coal";
pub(crate) const MOBIUS_FUEL: &str = "projecte:mobius_fuel";
pub(crate) const AETERNALIS_FUEL: &str = "projecte:aeternalis_fuel";

/// The fuel chain from plain coal to Aeternalis Fuel, weakest first.
pub(crate) const FUEL_TIERS: [&str; 4] = [COAL, ALCHEMICAL_COAL, MOBIUS_FUEL, AETERNALIS_FUEL];

/// One fuel block per ProjectE fuel tier, in the same order as [`FUEL_TIERS`].
pub(crate) const FUEL_BLOCKS: [(&str, &str); 3] = [
    (ALCHEMICAL_COAL, "projecte:alchemical_coal_block"),
    (MOBIUS_FUEL, "projecte:mobius_fuel_block"),
    (AETERNALIS_FUEL, "projecte:aeternalis_fuel_block"),
];

/// Furnace burn time in ticks for each [`FUEL_TIERS`] entry: vanilla coal is
/// 1600 and every alchemical tier lasts four times as long as the fuel it was
/// made from. The pinned API has no hook that sets an item's burn time (the
/// `furnace_burn` hook only hears the event), so this is metadata for the
/// later furnace integration, not live behavior yet.
pub(crate) const BURN_TICKS: [u32; 4] = [1_600, 6_400, 25_600, 102_400];

/// Position in [`FUEL_TIERS`], if `id` is a fuel.
pub(crate) fn tier(id: &str) -> Option<usize> {
    FUEL_TIERS.iter().position(|fuel| *fuel == id)
}

/// The fuel one step above `id`, e.g. coal to Alchemical Coal.
pub(crate) fn next_tier(id: &str) -> Option<&'static str> {
    let index = tier(id)?;
    FUEL_TIERS.get(index + 1).copied()
}

/// The fuel one step below `id`, e.g. Mobius Fuel to Alchemical Coal.
pub(crate) fn previous_tier(id: &str) -> Option<&'static str> {
    let index = tier(id)?;
    index
        .checked_sub(1)
        .and_then(|lower| FUEL_TIERS.get(lower).copied())
}

/// Furnace burn time in ticks, if `id` is a fuel.
pub(crate) fn burn_ticks(id: &str) -> Option<u32> {
    tier(id).map(|index| BURN_TICKS[index])
}

/// The fuel block a fuel packs into, if it has one.
pub(crate) fn fuel_block(id: &str) -> Option<&'static str> {
    FUEL_BLOCKS
        .iter()
        .find(|(fuel, _)| *fuel == id)
        .map(|(_, block)| *block)
}

/// The fuel a block unpacks into, if it is a fuel block.
#[cfg(test)]
pub(crate) fn fuel_of_block(block: &str) -> Option<&'static str> {
    FUEL_BLOCKS
        .iter()
        .find(|(_, packed)| *packed == block)
        .map(|(fuel, _)| *fuel)
}

/// One deterministic world transformation the Philosopher's Stone performs.
pub(crate) struct BlockSwap {
    pub from: &'static str,
    pub to: &'static str,
    /// The second result, applied while the player sneaks.
    pub sneaking: Option<&'static str>,
}

/// Fixed block-for-block swaps. Public behavior documents stone to
/// cobblestone (grass while sneaking), dirt and grass to sand, water to ice
/// and lava to obsidian; the remaining pairs keep a natural counterpart and
/// are provisional — see `PROGRESSION.md`.
pub(crate) const BLOCK_SWAPS: &[BlockSwap] = &[
    BlockSwap {
        from: "minecraft:stone",
        to: "minecraft:cobblestone",
        sneaking: Some("minecraft:grass_block"),
    },
    BlockSwap {
        from: "minecraft:cobblestone",
        to: "minecraft:stone",
        sneaking: None,
    },
    BlockSwap {
        from: "minecraft:dirt",
        to: "minecraft:sand",
        sneaking: Some("minecraft:clay"),
    },
    BlockSwap {
        from: "minecraft:grass_block",
        to: "minecraft:sand",
        sneaking: Some("minecraft:dirt"),
    },
    BlockSwap {
        from: "minecraft:sand",
        to: "minecraft:sandstone",
        sneaking: Some("minecraft:glass"),
    },
    BlockSwap {
        from: "minecraft:gravel",
        to: "minecraft:cobblestone",
        sneaking: Some("minecraft:sand"),
    },
    BlockSwap {
        from: "minecraft:water",
        to: "minecraft:ice",
        sneaking: None,
    },
    BlockSwap {
        from: "minecraft:ice",
        to: "minecraft:water",
        sneaking: None,
    },
    BlockSwap {
        from: "minecraft:lava",
        to: "minecraft:obsidian",
        sneaking: None,
    },
    BlockSwap {
        from: "minecraft:netherrack",
        to: "minecraft:nether_bricks",
        sneaking: Some("minecraft:soul_sand"),
    },
    BlockSwap {
        from: "minecraft:end_stone",
        to: "minecraft:obsidian",
        sneaking: Some("minecraft:purpur_block"),
    },
];

/// The eight overworld wood families, in the cycle order the stone walks.
const LOG_FAMILIES: [&str; 8] = [
    "oak", "spruce", "birch", "jungle", "acacia", "dark_oak", "mangrove", "cherry",
];

/// The sixteen vanilla wool colors, in the cycle order the stone walks.
const WOOL_COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

/// Resolves a world transformation: the block `from` becomes `to`, or the
/// sneaking variant of it. `None` means the stone leaves that block alone, or
/// that the sneaking form of a single-option swap has no second result.
pub(crate) fn world_swap(from: &str, sneaking: bool) -> Option<&'static str> {
    if let Some(swap) = BLOCK_SWAPS.iter().find(|swap| swap.from == from) {
        return if sneaking {
            swap.sneaking
        } else {
            Some(swap.to)
        };
    }
    for (suffix, families) in [
        ("_log", &LOG_FAMILIES[..]),
        ("_planks", &LOG_FAMILIES[..]),
        ("_wool", &WOOL_COLORS[..]),
        ("_carpet", &WOOL_COLORS[..]),
    ] {
        if let Some(next) = cycle(from, suffix, families, sneaking) {
            return Some(next);
        }
    }
    None
}

/// Whether the stone transforms `block` at all, whatever the sneaking state.
pub(crate) fn is_transformable(block: &str) -> bool {
    world_swap(block, false).is_some() || world_swap(block, true).is_some()
}

/// Cycles `minecraft:<family><suffix>` to the next family, or with `sneaking`
/// to the previous one, wrapping around.
fn cycle(
    from: &str,
    suffix: &str,
    families: &[&'static str],
    sneaking: bool,
) -> Option<&'static str> {
    let path = from.strip_prefix("minecraft:")?;
    let family = path.strip_suffix(suffix)?;
    let index = families.iter().position(|known| *known == family)?;
    let step = if sneaking { families.len() - 1 } else { 1 };
    let target = families[(index + step) % families.len()];
    let rendered = render(target, suffix);
    if rendered.is_empty() {
        None
    } else {
        Some(rendered)
    }
}

/// A `'static` id for a family and suffix pair; empty when the pair is not a
/// known block.
fn render(family: &str, suffix: &str) -> &'static str {
    match (family, suffix) {
        ("oak", "_log") => "minecraft:oak_log",
        ("spruce", "_log") => "minecraft:spruce_log",
        ("birch", "_log") => "minecraft:birch_log",
        ("jungle", "_log") => "minecraft:jungle_log",
        ("acacia", "_log") => "minecraft:acacia_log",
        ("dark_oak", "_log") => "minecraft:dark_oak_log",
        ("mangrove", "_log") => "minecraft:mangrove_log",
        ("cherry", "_log") => "minecraft:cherry_log",
        ("oak", "_planks") => "minecraft:oak_planks",
        ("spruce", "_planks") => "minecraft:spruce_planks",
        ("birch", "_planks") => "minecraft:birch_planks",
        ("jungle", "_planks") => "minecraft:jungle_planks",
        ("acacia", "_planks") => "minecraft:acacia_planks",
        ("dark_oak", "_planks") => "minecraft:dark_oak_planks",
        ("mangrove", "_planks") => "minecraft:mangrove_planks",
        ("cherry", "_planks") => "minecraft:cherry_planks",
        ("white", "_wool") => "minecraft:white_wool",
        ("orange", "_wool") => "minecraft:orange_wool",
        ("magenta", "_wool") => "minecraft:magenta_wool",
        ("light_blue", "_wool") => "minecraft:light_blue_wool",
        ("yellow", "_wool") => "minecraft:yellow_wool",
        ("lime", "_wool") => "minecraft:lime_wool",
        ("pink", "_wool") => "minecraft:pink_wool",
        ("gray", "_wool") => "minecraft:gray_wool",
        ("light_gray", "_wool") => "minecraft:light_gray_wool",
        ("cyan", "_wool") => "minecraft:cyan_wool",
        ("purple", "_wool") => "minecraft:purple_wool",
        ("blue", "_wool") => "minecraft:blue_wool",
        ("brown", "_wool") => "minecraft:brown_wool",
        ("green", "_wool") => "minecraft:green_wool",
        ("red", "_wool") => "minecraft:red_wool",
        ("black", "_wool") => "minecraft:black_wool",
        ("white", "_carpet") => "minecraft:white_carpet",
        ("orange", "_carpet") => "minecraft:orange_carpet",
        ("magenta", "_carpet") => "minecraft:magenta_carpet",
        ("light_blue", "_carpet") => "minecraft:light_blue_carpet",
        ("yellow", "_carpet") => "minecraft:yellow_carpet",
        ("lime", "_carpet") => "minecraft:lime_carpet",
        ("pink", "_carpet") => "minecraft:pink_carpet",
        ("gray", "_carpet") => "minecraft:gray_carpet",
        ("light_gray", "_carpet") => "minecraft:light_gray_carpet",
        ("cyan", "_carpet") => "minecraft:cyan_carpet",
        ("purple", "_carpet") => "minecraft:purple_carpet",
        ("blue", "_carpet") => "minecraft:blue_carpet",
        ("brown", "_carpet") => "minecraft:brown_carpet",
        ("green", "_carpet") => "minecraft:green_carpet",
        ("red", "_carpet") => "minecraft:red_carpet",
        ("black", "_carpet") => "minecraft:black_carpet",
        _ => "",
    }
}

/// A conversion the Philosopher's Stone performs in a crafting grid, as the
/// data-pack recipes in [`crate::recipes`] express it. The amounts are chosen
/// so both directions carry the same EMC.
#[cfg(test)]
pub(crate) struct StoneConversion {
    pub from: &'static str,
    pub from_count: u32,
    pub to: &'static str,
    pub to_count: u32,
}

#[cfg(test)]
pub(crate) const STONE_CONVERSIONS: &[StoneConversion] = &[
    StoneConversion {
        from: "minecraft:iron_ingot",
        from_count: 8,
        to: "minecraft:gold_ingot",
        to_count: 1,
    },
    StoneConversion {
        from: "minecraft:gold_ingot",
        from_count: 1,
        to: "minecraft:iron_ingot",
        to_count: 8,
    },
    StoneConversion {
        from: "minecraft:gold_ingot",
        from_count: 4,
        to: "minecraft:diamond",
        to_count: 1,
    },
    StoneConversion {
        from: "minecraft:diamond",
        from_count: 1,
        to: "minecraft:gold_ingot",
        to_count: 4,
    },
    StoneConversion {
        from: "minecraft:diamond",
        from_count: 2,
        to: "minecraft:emerald",
        to_count: 1,
    },
    StoneConversion {
        from: "minecraft:emerald",
        from_count: 1,
        to: "minecraft:diamond",
        to_count: 2,
    },
    StoneConversion {
        from: "minecraft:iron_ingot",
        from_count: 4,
        to: "minecraft:ender_pearl",
        to_count: 1,
    },
    StoneConversion {
        from: "minecraft:ender_pearl",
        from_count: 1,
        to: "minecraft:iron_ingot",
        to_count: 4,
    },
];

/// Total EMC of `count` items of `id`, or `None` when the total cannot be
/// represented.
#[cfg(test)]
pub(crate) fn total_value(id: &str, count: u32) -> Option<u64> {
    let each = emc::value(id);
    if each == 0 {
        return None;
    }
    each.checked_mul(u64::from(count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fuel_chain_steps_up_and_down() {
        assert_eq!(tier(COAL), Some(0));
        assert_eq!(next_tier(COAL), Some(ALCHEMICAL_COAL));
        assert_eq!(next_tier(ALCHEMICAL_COAL), Some(MOBIUS_FUEL));
        assert_eq!(next_tier(MOBIUS_FUEL), Some(AETERNALIS_FUEL));
        assert_eq!(next_tier(AETERNALIS_FUEL), None);
        assert_eq!(previous_tier(COAL), None);
        assert_eq!(previous_tier(AETERNALIS_FUEL), Some(MOBIUS_FUEL));
        assert_eq!(next_tier("minecraft:iron_ingot"), None);
        assert_eq!(previous_tier("projecte:red_matter"), None);
    }

    #[test]
    fn every_fuel_tier_is_worth_four_times_the_previous_one() {
        for pair in FUEL_TIERS.windows(2) {
            let lower = emc::value(pair[0]);
            let upper = emc::value(pair[1]);
            assert!(lower > 0 && upper > 0, "{pair:?} must have EMC values");
            assert_eq!(upper, lower * 4, "{pair:?} is not a fourfold step");
        }
    }

    #[test]
    fn the_four_to_one_crafting_step_is_emc_neutral() {
        for pair in FUEL_TIERS.windows(2) {
            let cost = total_value(pair[0], 4).expect("four of a fuel");
            let made = total_value(pair[1], 1).expect("one fuel");
            assert_eq!(cost, made, "{pair:?} would create or destroy EMC");
        }
    }

    #[test]
    fn burn_times_are_fourfold_and_match_the_tiers() {
        assert_eq!(burn_ticks(COAL), Some(1_600));
        for pair in FUEL_TIERS.windows(2) {
            let lower = burn_ticks(pair[0]).expect("a fuel");
            let upper = burn_ticks(pair[1]).expect("a fuel");
            assert_eq!(upper, lower * 4, "{pair:?} burn time");
        }
        assert_eq!(burn_ticks("minecraft:iron_ingot"), None);
    }

    #[test]
    fn fuel_blocks_pack_nine_fuels() {
        for (fuel, block) in FUEL_BLOCKS {
            assert_eq!(fuel_block(fuel), Some(block));
            assert_eq!(fuel_of_block(block), Some(fuel));
            assert_eq!(emc::value(block), emc::value(fuel) * 9, "{block}");
        }
        assert_eq!(fuel_block(COAL), None);
        assert_eq!(fuel_of_block("minecraft:coal_block"), None);
    }

    #[test]
    fn documented_world_swaps_resolve_both_ways() {
        assert_eq!(
            world_swap("minecraft:stone", false),
            Some("minecraft:cobblestone")
        );
        assert_eq!(
            world_swap("minecraft:stone", true),
            Some("minecraft:grass_block")
        );
        assert_eq!(
            world_swap("minecraft:grass_block", false),
            Some("minecraft:sand")
        );
        assert_eq!(world_swap("minecraft:water", false), Some("minecraft:ice"));
        assert_eq!(
            world_swap("minecraft:lava", false),
            Some("minecraft:obsidian")
        );
        // A swap without a sneaking variant stays put while sneaking.
        assert_eq!(world_swap("minecraft:lava", true), None);
    }

    #[test]
    fn wood_and_wool_families_cycle() {
        assert_eq!(
            world_swap("minecraft:oak_log", false),
            Some("minecraft:spruce_log")
        );
        assert_eq!(
            world_swap("minecraft:spruce_log", true),
            Some("minecraft:oak_log")
        );
        assert_eq!(
            world_swap("minecraft:cherry_log", false),
            Some("minecraft:oak_log")
        );
        assert_eq!(
            world_swap("minecraft:oak_planks", false),
            Some("minecraft:spruce_planks")
        );
        assert_eq!(
            world_swap("minecraft:white_wool", false),
            Some("minecraft:orange_wool")
        );
        assert_eq!(
            world_swap("minecraft:black_wool", false),
            Some("minecraft:white_wool")
        );
        assert_eq!(
            world_swap("minecraft:white_carpet", true),
            Some("minecraft:black_carpet")
        );
    }

    #[test]
    fn unsupported_blocks_stay_untouched() {
        assert_eq!(world_swap("minecraft:diamond_block", false), None);
        assert_eq!(world_swap("minecraft:bedrock", false), None);
        assert_eq!(world_swap("example:mystery_block", false), None);
        assert_eq!(world_swap("minecraft:stripped_oak_log", false), None);
        assert_eq!(world_swap("", false), None);
    }

    #[test]
    fn transformable_blocks_are_reported_for_both_forms() {
        assert!(is_transformable("minecraft:stone"));
        assert!(is_transformable("minecraft:lava"));
        assert!(is_transformable("minecraft:red_wool"));
        assert!(!is_transformable("minecraft:diamond_block"));
        assert!(!is_transformable("minecraft:stripped_oak_log"));
        // Sneaking on a single-option swap changes nothing but is not an
        // unsupported block.
        assert_eq!(world_swap("minecraft:lava", true), None);
        assert!(is_transformable("minecraft:lava"));
    }

    #[test]
    fn stone_conversions_are_emc_neutral() {
        for conversion in STONE_CONVERSIONS {
            let spent =
                total_value(conversion.from, conversion.from_count).expect("a valued ingredient");
            let made = total_value(conversion.to, conversion.to_count).expect("a valued result");
            assert_eq!(
                spent, made,
                "{} x{} -> {} x{} is not neutral",
                conversion.from, conversion.from_count, conversion.to, conversion.to_count
            );
        }
    }

    #[test]
    fn every_documented_stone_pair_appears_in_both_directions() {
        for conversion in STONE_CONVERSIONS {
            assert!(
                STONE_CONVERSIONS.iter().any(|reverse| {
                    reverse.from == conversion.to
                        && reverse.to == conversion.from
                        && reverse.from_count == conversion.to_count
                        && reverse.to_count == conversion.from_count
                }),
                "{} -> {} has no reverse",
                conversion.from,
                conversion.to
            );
        }
    }
}
