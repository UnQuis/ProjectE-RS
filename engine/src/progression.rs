//! The complete ProjectE progression, as typed data.
//!
//! `PROGRESSION.md` documents the same roadmap in prose. This module keeps the
//! machine-readable part honest: every node names its prerequisites, its
//! implemented state and the Horizon capabilities it needs, and the tests
//! below check the roadmap is ordered, that implemented content exists, and
//! that no node silently references an unknown prerequisite.
//!
//! A node is [`Status::Implemented`] only when the item, its recipe and its
//! behavior exist at the pinned API. Anything that still needs a missing
//! Horizon capability stays [`Status::Blocked`] or [`Status::Partial`], even
//! when the item can be crafted.

/// How far a progression node has been built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// Item, recipe and behavior are all in place.
    Implemented,
    /// Something playable exists, but the node is incomplete.
    Partial,
    /// Nothing of the node is implemented yet.
    Planned,
    /// The item may exist, but its behavior needs a Horizon capability the
    /// pinned API does not expose.
    Blocked,
}

impl Status {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Partial => "partial",
            Self::Planned => "missing",
            Self::Blocked => "blocked",
        }
    }
}

/// One step of the progression.
pub(crate) struct Node {
    /// Stable identifier, used by `PROGRESSION.md` and by the tests.
    pub id: &'static str,
    pub name: &'static str,
    /// Coarse stage: 0 is the Philosopher's Stone, 4 is endgame.
    pub stage: u8,
    pub status: Status,
    /// Ids of earlier nodes this one needs.
    pub prerequisites: &'static [&'static str],
    /// The gameplay the node unlocks.
    pub unlocks: &'static str,
    /// The Horizon capabilities the node relies on, or needs and lacks.
    pub horizon: &'static str,
}

/// The roadmap, in progression order. Prerequisites always point at an
/// earlier entry, which the `prerequisites_come_first` test enforces.
pub(crate) const NODES: &[Node] = &[
    Node {
        id: "philosophers_stone",
        name: "Philosopher's Stone",
        stage: 0,
        status: Status::Implemented,
        prerequisites: &[],
        unlocks: "EMC conversions in a crafting grid, block transmutation in the world, and access to the Transmutation Table",
        horizon: "items and data_pack_files for recipes; item_crafted returns the catalyst stone; use_item_on plus edit_blocks for blocks",
    },
    Node {
        id: "transmutation_table",
        name: "Transmutation Table",
        stage: 0,
        status: Status::Partial,
        prerequisites: &["philosophers_stone"],
        unlocks: "Learning items and buying learned items with EMC",
        horizon: "blocks, open_menu, menu_clicked; data maps are unreadable, so EMC values are static (API_WORKAROUNDS.md)",
    },
    Node {
        id: "emc_acquisition",
        name: "EMC acquisition and knowledge",
        stage: 0,
        status: Status::Partial,
        prerequisites: &["transmutation_table"],
        unlocks: "An EMC balance and a learned-item list that belong to the player",
        horizon: "player attachment with a codec, server-authoritative; depositing items for EMC is not implemented yet",
    },
    Node {
        id: "alchemical_fuels",
        name: "Alchemical fuels",
        stage: 1,
        status: Status::Implemented,
        prerequisites: &["philosophers_stone"],
        unlocks: "Alchemical Coal, Mobius Fuel and Aeternalis Fuel crafting, four of a fuel at a time",
        horizon: "data_pack_files for the recipes; item_crafted returns the stone",
    },
    Node {
        id: "fuel_blocks",
        name: "Fuel blocks",
        stage: 1,
        status: Status::Implemented,
        prerequisites: &["alchemical_fuels"],
        unlocks: "Compact fuel storage and a nine-fuel packing step",
        horizon: "blocks plus data_pack_files",
    },
    Node {
        id: "fuel_burn_time",
        name: "Fuel burn time",
        stage: 1,
        status: Status::Blocked,
        prerequisites: &["alchemical_fuels"],
        unlocks: "Fuels would burn for 4x, 16x and 64x vanilla coal",
        horizon: "furnace_burn only hears the event, so a mod cannot set an item's burn time at this revision",
    },
    Node {
        id: "alchemical_chest",
        name: "Alchemical Chest",
        stage: 2,
        status: Status::Partial,
        prerequisites: &["transmutation_table"],
        unlocks: "A persistent 27-slot storage block",
        horizon: "block_entity_container with a 27-slot generic menu; recipe is provisional",
    },
    Node {
        id: "energy_collector",
        name: "Energy Collector tiers",
        stage: 2,
        status: Status::Partial,
        prerequisites: &["transmutation_table", "alchemical_fuels"],
        unlocks: "Passive EMC generation from light, and fuel upgrading in its slots",
        horizon: "block_entity_energy is not callable from a trait mod, so machine EMC lives in block-entity NBT (API_WORKAROUNDS.md)",
    },
    Node {
        id: "antimatter_relay",
        name: "Anti-Matter Relay tiers",
        stage: 2,
        status: Status::Partial,
        prerequisites: &["energy_collector"],
        unlocks: "Storing and routing EMC, with a bonus per transfer",
        horizon: "the same block-energy limitation as collectors; neighbour writes go through change_mod_block_entity",
    },
    Node {
        id: "klein_stars",
        name: "Klein Star tiers",
        stage: 2,
        status: Status::Partial,
        prerequisites: &["energy_collector"],
        unlocks: "Portable EMC storage that machines charge and drain",
        horizon: "item_energy_buffer is not callable from a trait mod, so charge uses a ProjectE data component (API_WORKAROUNDS.md)",
    },
    Node {
        id: "energy_condenser",
        name: "Energy Condenser MK1 and MK2",
        stage: 2,
        status: Status::Partial,
        prerequisites: &[
            "antimatter_relay",
            "transmutation_table",
            "alchemical_chest",
        ],
        unlocks: "Automatic item duplication paid for with EMC",
        horizon: "MK1 uses block_entity_container and NBT; MK2 batching and inventory layout remain missing",
    },
    Node {
        id: "power_flower",
        name: "Power Flower",
        stage: 2,
        status: Status::Partial,
        prerequisites: &["energy_collector", "antimatter_relay", "energy_condenser"],
        unlocks: "Collector and Relay networks feeding a terminal Condenser sink",
        horizon: "adjacent live block-entity mutations; Condensers accept EMC but never forward it",
    },
    Node {
        id: "emc_data_map",
        name: "Data-Map-driven EMC values",
        stage: 2,
        status: Status::Blocked,
        prerequisites: &["emc_acquisition"],
        unlocks: "Pack-defined EMC overrides through projecte:emc",
        horizon: "the map registers through data_map_types, but there is no Host read helper at the pinned revision (API_WORKAROUNDS.md)",
    },
    Node {
        id: "dark_matter",
        name: "Dark Matter",
        stage: 3,
        status: Status::Partial,
        prerequisites: &["alchemical_fuels"],
        unlocks: "The first matter tier: tools, armor and machine upgrades",
        horizon: "items and data_pack_files; the recipe's exact ingredients are still unverified",
    },
    Node {
        id: "red_matter",
        name: "Red Matter",
        stage: 3,
        status: Status::Partial,
        prerequisites: &["dark_matter"],
        unlocks: "The endgame matter tier",
        horizon: "items and data_pack_files; the recipe's exact ingredients are still unverified",
    },
    Node {
        id: "matter_blocks",
        name: "Matter blocks",
        stage: 3,
        status: Status::Planned,
        prerequisites: &["dark_matter"],
        unlocks: "Storage blocks for the Tablet, the Condenser MK2 and armor",
        horizon: "blocks plus 3x3 pack and unpack recipes",
    },
    Node {
        id: "matter_tools",
        name: "Dark and Red Matter tools",
        stage: 3,
        status: Status::Planned,
        prerequisites: &["dark_matter", "red_matter"],
        unlocks: "Tools that do not break, with area mining and special attacks",
        horizon: "tool items need item components plus destroy_block and item_damaged at this revision",
    },
    Node {
        id: "matter_armor",
        name: "Dark and Red Matter armor, Gem armor",
        stage: 3,
        status: Status::Planned,
        prerequisites: &["matter_tools", "klein_stars"],
        unlocks: "Armor that spends EMC instead of taking damage",
        horizon: "on_equip and living_incoming_damage; preventing damage needs the cancelling form of that hook",
    },
    Node {
        id: "transmutation_tablet",
        name: "Transmutation Tablet",
        stage: 3,
        status: Status::Planned,
        prerequisites: &["transmutation_table", "matter_blocks"],
        unlocks: "The portable Transmutation Table",
        horizon: "use_item plus the existing menu path; no new capability needed",
    },
    Node {
        id: "tome_of_knowledge",
        name: "Tome of Knowledge",
        stage: 3,
        status: Status::Planned,
        prerequisites: &["klein_stars", "transmutation_table"],
        unlocks: "Learning every item at once",
        horizon: "items plus a use_item path that fills the learned set",
    },
    Node {
        id: "utility_devices",
        name: "Utility devices: rings, amulets, bags, catalysts",
        stage: 4,
        status: Status::Planned,
        prerequisites: &["dark_matter", "red_matter"],
        unlocks: "Flight, water and lava control, item pickup, portable storage, block destruction",
        horizon: "player movement, fluids and EMC spending hooks; some abilities have no equivalent at this revision",
    },
    Node {
        id: "advancements",
        name: "Advancement progression",
        stage: 4,
        status: Status::Planned,
        prerequisites: &[
            "philosophers_stone",
            "alchemical_fuels",
            "dark_matter",
            "red_matter",
        ],
        unlocks: "Visible goals and unlock relationships across the whole mod",
        horizon: "advancement files through data_pack_files; the catalog also has advancement hooks",
    },
];

/// Look a node up by id. Only the roadmap tests need it today.
#[cfg(test)]
pub(crate) fn node(id: &str) -> Option<&'static Node> {
    NODES.iter().find(|node| node.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn node_ids_are_unique() {
        let mut seen = BTreeSet::new();
        for node in NODES {
            assert!(seen.insert(node.id), "duplicate node id {}", node.id);
            assert!(!node.id.contains(':'), "{} must be a bare id", node.id);
            assert!(!node.name.is_empty(), "{} has no name", node.id);
            assert!(!node.unlocks.is_empty(), "{} has no unlock text", node.id);
            assert!(!node.horizon.is_empty(), "{} has no API notes", node.id);
        }
    }

    #[test]
    fn prerequisites_come_first() {
        let mut earlier = BTreeSet::new();
        for node in NODES {
            for prerequisite in node.prerequisites {
                assert!(
                    earlier.contains(prerequisite),
                    "{} needs {prerequisite}, which is not earlier in the roadmap",
                    node.id
                );
            }
            earlier.insert(node.id);
        }
    }

    #[test]
    fn every_implemented_node_exists_in_the_game() {
        // Implemented nodes must name real content in this crate's registries.
        for node in NODES.iter().filter(|n| n.status == Status::Implemented) {
            match node.id {
                "philosophers_stone" => {
                    assert!(crate::emc::value(crate::alchemical::PHILOSOPHERS_STONE) > 0)
                }
                "alchemical_fuels" => {
                    for fuel in crate::alchemical::FUEL_TIERS.iter().skip(1).copied() {
                        assert!(crate::emc::value(fuel) > 0, "{fuel}");
                        assert!(
                            crate::recipes::SHAPELESS
                                .iter()
                                .any(|recipe| recipe.result == fuel),
                            "{fuel} has no recipe"
                        );
                    }
                }
                "fuel_blocks" => {
                    for (_, block) in crate::alchemical::FUEL_BLOCKS {
                        assert!(crate::emc::value(block) > 0, "{block}");
                        assert!(
                            crate::recipes::SHAPED
                                .iter()
                                .any(|recipe| recipe.result == block),
                            "{block} has no recipe"
                        );
                    }
                }
                other => panic!("{other} claims implemented status without a registered item"),
            }
        }
    }

    #[test]
    fn the_foundation_is_complete_and_the_endgame_is_not() {
        assert_eq!(
            node("philosophers_stone").map(|n| n.status),
            Some(Status::Implemented)
        );
        assert_eq!(
            node("alchemical_fuels").map(|n| n.status),
            Some(Status::Implemented)
        );
        // Nothing in the matter tiers is finished yet.
        for id in [
            "matter_blocks",
            "matter_tools",
            "matter_armor",
            "utility_devices",
        ] {
            let node = node(id).expect("a roadmap node");
            assert_ne!(node.status, Status::Implemented, "{id}");
        }
    }

    #[test]
    fn blocked_nodes_name_a_missing_capability() {
        for node in NODES.iter().filter(|n| n.status == Status::Blocked) {
            assert!(
                node.horizon.contains("not callable")
                    || node.horizon.contains("no Host")
                    || node.horizon.contains("cannot set")
                    || node.horizon.contains("no Host read helper"),
                "{} is blocked without saying what is missing",
                node.id
            );
        }
    }

    #[test]
    fn stages_are_ordered_by_first_appearance() {
        let mut last = 0;
        for node in NODES {
            assert!(
                node.stage >= last,
                "{} goes back to stage {}",
                node.id,
                node.stage
            );
            last = node.stage;
        }
        assert!(
            NODES.iter().any(|node| node.stage == 0),
            "the foundation stage is empty"
        );
    }
}
