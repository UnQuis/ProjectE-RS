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
const COBBLESTONE: &str = "minecraft:cobblestone";
const FURNACE: &str = "minecraft:furnace";
const CHEST: &str = "minecraft:chest";
const GLASS: &str = "minecraft:glass";
const IRON: &str = "minecraft:iron_ingot";
const GOLD: &str = "minecraft:gold_ingot";
const EMERALD: &str = "minecraft:emerald";
const ENDER_PEARL: &str = "minecraft:ender_pearl";
const COAL: &str = "minecraft:coal";
const CHARCOAL: &str = "minecraft:charcoal";
const STICK: &str = "minecraft:stick";
const STRING: &str = "minecraft:string";
const PAPER: &str = "minecraft:paper";
const BARREL: &str = "minecraft:barrel";
const TNT: &str = "minecraft:tnt";
const WATER_BUCKET: &str = "minecraft:water_bucket";
const LAVA_BUCKET: &str = "minecraft:lava_bucket";
const FLINT_AND_STEEL: &str = "minecraft:flint_and_steel";
const CLOCK: &str = "minecraft:clock";
const BRICKS: &str = "minecraft:bricks";
const SNOWBALL: &str = "minecraft:snowball";
const SNOW_BLOCK: &str = "minecraft:snow_block";
const OAK_SAPLING: &str = "minecraft:oak_sapling";
const DANDELION: &str = "minecraft:dandelion";
const TRIDENT: &str = "minecraft:trident";
const BOW: &str = "minecraft:bow";
const COOKED_BEEF: &str = "minecraft:cooked_beef";
const FEATHER: &str = "minecraft:feather";
const LAPIS: &str = "minecraft:lapis_lazuli";
const SUGAR: &str = "minecraft:sugar";
const BOOK: &str = "minecraft:book";
const DARK_MATTER: &str = "projecte:dark_matter";
const RED_MATTER: &str = "projecte:red_matter";

pub(crate) const ALCHEMICAL_COAL: &str = "projecte:alchemical_coal";
pub(crate) const MOBIUS_FUEL: &str = "projecte:mobius_fuel";
pub(crate) const AETERNALIS_FUEL: &str = "projecte:aeternalis_fuel";
pub(crate) const ALCHEMICAL_COAL_BLOCK: &str = "projecte:alchemical_coal_block";
pub(crate) const MOBIUS_FUEL_BLOCK: &str = "projecte:mobius_fuel_block";
pub(crate) const AETERNALIS_FUEL_BLOCK: &str = "projecte:aeternalis_fuel_block";

pub(crate) const COLLECTOR: &str = "projecte:collector_mk1";
pub(crate) const COLLECTOR_MK2: &str = "projecte:collector_mk2";
pub(crate) const COLLECTOR_MK3: &str = "projecte:collector_mk3";
pub(crate) const RELAY: &str = "projecte:relay_mk1";
pub(crate) const RELAY_MK2: &str = "projecte:relay_mk2";
pub(crate) const RELAY_MK3: &str = "projecte:relay_mk3";
pub(crate) const TABLE: &str = "projecte:transmutation_table";
pub(crate) const ALCHEMICAL_CHEST: &str = "projecte:alchemical_chest";
pub(crate) const ENERGY_CONDENSER: &str = "projecte:condenser_mk1";
pub(crate) const KLEIN_STAR_EIN: &str = "projecte:klein_star_ein";
pub(crate) const KLEIN_STAR_ZWEI: &str = "projecte:klein_star_zwei";
pub(crate) const KLEIN_STAR_DREI: &str = "projecte:klein_star_drei";
pub(crate) const KLEIN_STAR_VIER: &str = "projecte:klein_star_vier";
pub(crate) const KLEIN_STAR_SPHERE: &str = "projecte:klein_star_sphere";
pub(crate) const KLEIN_STAR_OMEGA: &str = "projecte:klein_star_omega";

pub(crate) const DARK_MATTER_BLOCK: &str = "projecte:dark_matter_block";
pub(crate) const RED_MATTER_BLOCK: &str = "projecte:red_matter_block";

pub(crate) const LOW_COVALENCE_DUST: &str = "projecte:low_covalence_dust";
pub(crate) const MEDIUM_COVALENCE_DUST: &str = "projecte:medium_covalence_dust";
pub(crate) const HIGH_COVALENCE_DUST: &str = "projecte:high_covalence_dust";

pub(crate) const CONDENSER_MK2: &str = "projecte:condenser_mk2";
pub(crate) const CONDENSER_MK3: &str = "projecte:condenser_mk3";
pub(crate) const MAGENTA_MATTER: &str = "projecte:magenta_matter";
pub(crate) const MAGENTA_MATTER_BLOCK: &str = "projecte:magenta_matter_block";
pub(crate) const TRANSMUTATION_TABLET: &str = "projecte:transmutation_tablet";
pub(crate) const DM_PEDESTAL: &str = "projecte:dm_pedestal";
pub(crate) const DM_FURNACE: &str = "projecte:dm_furnace";
pub(crate) const RM_FURNACE: &str = "projecte:rm_furnace";
pub(crate) const ALCHEMICAL_BARREL: &str = "projecte:alchemical_barrel";
pub(crate) const REPAIR_TALISMAN: &str = "projecte:repair_talisman";
pub(crate) const NOVA_CATALYST: &str = "projecte:nova_catalyst";
pub(crate) const NOVA_CATACLYSM: &str = "projecte:nova_cataclysm";
pub(crate) const DIVINING_ROD_1: &str = "projecte:divining_rod_1";
pub(crate) const DIVINING_ROD_2: &str = "projecte:divining_rod_2";
pub(crate) const DIVINING_ROD_3: &str = "projecte:divining_rod_3";
pub(crate) const DM_AXE: &str = "projecte:dm_axe";
pub(crate) const DM_BOOTS: &str = "projecte:dm_boots";
pub(crate) const DM_CHESTPLATE: &str = "projecte:dm_chestplate";
pub(crate) const DM_HAMMER: &str = "projecte:dm_hammer";
pub(crate) const DM_HELMET: &str = "projecte:dm_helmet";
pub(crate) const DM_HOE: &str = "projecte:dm_hoe";
pub(crate) const DM_LEGGINGS: &str = "projecte:dm_leggings";
pub(crate) const DM_PICK: &str = "projecte:dm_pick";
pub(crate) const DM_SHEARS: &str = "projecte:dm_shears";
pub(crate) const DM_SHOVEL: &str = "projecte:dm_shovel";
pub(crate) const DM_SWORD: &str = "projecte:dm_sword";
pub(crate) const IRON_BAND: &str = "projecte:iron_band";
pub(crate) const VOLCANITE_AMULET: &str = "projecte:volcanite_amulet";
pub(crate) const EVERTIDE_AMULET: &str = "projecte:evertide_amulet";
pub(crate) const BLACK_HOLE_BAND: &str = "projecte:black_hole_band";
pub(crate) const ZERO_RING: &str = "projecte:zero_ring";
pub(crate) const HARVEST_GODDESS_BAND: &str = "projecte:harvest_goddess_band";
pub(crate) const IGNITION_RING: &str = "projecte:ignition_ring";
pub(crate) const SWIFTWOLF_RENDING_GALE: &str = "projecte:swiftwolf_rending_gale";
pub(crate) const ARCANA_RING: &str = "projecte:arcana_ring";
pub(crate) const VOID_RING: &str = "projecte:void_ring";
pub(crate) const GEM_OF_ETERNAL_DENSITY: &str = "projecte:gem_of_eternal_density";
pub(crate) const BODY_STONE: &str = "projecte:body_stone";
pub(crate) const SOUL_STONE: &str = "projecte:soul_stone";
pub(crate) const MIND_STONE: &str = "projecte:mind_stone";
pub(crate) const LIFE_STONE: &str = "projecte:life_stone";
pub(crate) const DESTRUCTION_CATALYST: &str = "projecte:destruction_catalyst";
pub(crate) const HYPERKINETIC_LENS: &str = "projecte:hyperkinetic_lens";
pub(crate) const CATALYTIC_LENS: &str = "projecte:catalytic_lens";
pub(crate) const MERCURIAL_EYE: &str = "projecte:mercurial_eye";
pub(crate) const WATCH_OF_FLOWING_TIME: &str = "projecte:watch_of_flowing_time";
pub(crate) const DARK_MATTER_SHIELD: &str = "projecte:dark_matter_shield";
pub(crate) const RED_MATTER_SHIELD: &str = "projecte:red_matter_shield";
pub(crate) const DARK_MATTER_TRIDENT: &str = "projecte:dark_matter_trident";
pub(crate) const RED_MATTER_TRIDENT: &str = "projecte:red_matter_trident";
pub(crate) const RM_LEGGINGS: &str = "projecte:rm_leggings";
pub(crate) const ARCHANGEL_SMITE: &str = "projecte:archangel_smite";
pub(crate) const GEM_BOOTS: &str = "projecte:gem_boots";
pub(crate) const GEM_CHESTPLATE: &str = "projecte:gem_chestplate";
pub(crate) const GEM_HELMET: &str = "projecte:gem_helmet";
pub(crate) const GEM_LEGGINGS: &str = "projecte:gem_leggings";
pub(crate) const BASIC_ALCHEMICAL_BOOK: &str = "projecte:basic_alchemical_book";
pub(crate) const INFINITE_FUEL: &str = "projecte:infinite_fuel";
pub(crate) const INFINITE_STEAK: &str = "projecte:infinite_steak";
pub(crate) const RM_AXE: &str = "projecte:rm_axe";
pub(crate) const RM_BOOTS: &str = "projecte:rm_boots";
pub(crate) const RM_CHESTPLATE: &str = "projecte:rm_chestplate";
pub(crate) const RM_HAMMER: &str = "projecte:rm_hammer";
pub(crate) const RM_HELMET: &str = "projecte:rm_helmet";
pub(crate) const RM_PICK: &str = "projecte:rm_pick";
pub(crate) const RM_SHEARS: &str = "projecte:rm_shears";
pub(crate) const RM_SHOVEL: &str = "projecte:rm_shovel";
pub(crate) const RM_SWORD: &str = "projecte:rm_sword";

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
    "stone_charcoal_to_coal",
    "stone_coal_to_charcoal",
    "alchemical_coal_to_coal",
    "mobius_fuel_to_alchemical_coal",
    "aeternalis_fuel_to_mobius_fuel",
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
    // ProjectE's documented alternate layout uses the same nine ingredients.
    ShapedRecipe {
        name: "philosophers_stone_alt",
        pattern: ["GRG", "RDR", "GRG"],
        key: &[('G', GLOWSTONE_DUST), ('R', REDSTONE), ('D', DIAMOND)],
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
    // ProjectE `collector_mk1`: glowstone ring, glass above a diamond-block
    // core, furnace below it. The glass is part of the shape, not decoration.
    ShapedRecipe {
        name: "collector_mk1",
        pattern: ["GTG", "GDG", "GFG"],
        key: &[
            ('G', GLOWSTONE),
            ('T', GLASS),
            ('D', DIAMOND_BLOCK),
            ('F', FURNACE),
        ],
        result: COLLECTOR,
        count: 1,
    },
    // ProjectE `relay_mk1`: seven obsidian, glass above the diamond-block core.
    ShapedRecipe {
        name: "relay_mk1",
        pattern: ["OSO", "ODO", "OOO"],
        key: &[('O', OBSIDIAN), ('S', GLASS), ('D', DIAMOND_BLOCK)],
        result: RELAY,
        count: 1,
    },
    // ProjectE `alchemical_chest`: one of each covalence dust, two stone,
    // one diamond, two iron and a wooden chest. The placeholder obsidian ring
    // is gone — its 544 EMC was never a reference value.
    ShapedRecipe {
        name: "alchemical_chest",
        pattern: ["LMH", "SDS", "ICI"],
        key: &[
            ('L', LOW_COVALENCE_DUST),
            ('M', MEDIUM_COVALENCE_DUST),
            ('H', HIGH_COVALENCE_DUST),
            ('S', STONE_BLOCK),
            ('D', DIAMOND),
            ('I', IRON),
            ('C', CHEST),
        ],
        result: ALCHEMICAL_CHEST,
        count: 1,
    },
    // Energy Condenser MK1: obsidian corners, diamond sides, Alchemical Chest core.
    ShapedRecipe {
        name: "condenser_mk1",
        pattern: ["ODO", "DCD", "ODO"],
        key: &[('O', OBSIDIAN), ('D', DIAMOND), ('C', ALCHEMICAL_CHEST)],
        result: ENERGY_CONDENSER,
        count: 1,
    },
    // ProjectE `collector_mk2` / `mk3`: seven glowstone, the matter tier in
    // the TOP-CENTRE slot, the previous collector in the middle.
    ShapedRecipe {
        name: "collector_mk2",
        pattern: ["GUG", "GPG", "GGG"],
        key: &[('G', GLOWSTONE), ('U', DARK_MATTER), ('P', COLLECTOR)],
        result: COLLECTOR_MK2,
        count: 1,
    },
    ShapedRecipe {
        name: "collector_mk3",
        pattern: ["GUG", "GPG", "GGG"],
        key: &[('G', GLOWSTONE), ('U', RED_MATTER), ('P', COLLECTOR_MK2)],
        result: COLLECTOR_MK3,
        count: 1,
    },
    // ProjectE `relay_mk2` / `mk3`: an obsidian ring, no glowstone at all.
    ShapedRecipe {
        name: "relay_mk2",
        pattern: ["OUO", "OPO", "OOO"],
        key: &[('O', OBSIDIAN), ('U', DARK_MATTER), ('P', RELAY)],
        result: RELAY_MK2,
        count: 1,
    },
    ShapedRecipe {
        name: "relay_mk3",
        pattern: ["OUO", "OPO", "OOO"],
        key: &[('O', OBSIDIAN), ('U', RED_MATTER), ('P', RELAY_MK2)],
        result: RELAY_MK3,
        count: 1,
    },
    // ProjectE `klein_star_ein`: eight Mobius Fuel around one diamond.
    ShapedRecipe {
        name: "klein_star_ein",
        pattern: ["MMM", "MDM", "MMM"],
        key: &[('M', MOBIUS_FUEL), ('D', DIAMOND)],
        result: KLEIN_STAR_EIN,
        count: 1,
    },
    // ProjectE `dark_matter`: eight Aeternalis Fuel around a diamond BLOCK.
    ShapedRecipe {
        name: "dark_matter",
        pattern: ["AAA", "ADA", "AAA"],
        key: &[('A', AETERNALIS_FUEL), ('D', DIAMOND_BLOCK)],
        result: DARK_MATTER,
        count: 1,
    },
    // ProjectE `red_matter`: six Aeternalis Fuel and three Dark Matter.
    // (`red_matter_alt` uses the same counts in three identical columns.)
    ShapedRecipe {
        name: "red_matter",
        pattern: ["AAA", "DDD", "AAA"],
        key: &[('A', AETERNALIS_FUEL), ('D', DARK_MATTER)],
        result: RED_MATTER,
        count: 1,
    },
    // ProjectE matter blocks are four matter in a 2x2 square. This crate's
    // recipe type is fixed at 3 rows, so the square sits in the top-left with
    // trailing blanks — the same four items, the same shape.
    ShapedRecipe {
        name: "dark_matter_block",
        pattern: ["MM ", "MM ", "   "],
        key: &[('M', DARK_MATTER)],
        result: DARK_MATTER_BLOCK,
        count: 1,
    },
    ShapedRecipe {
        name: "red_matter_block",
        pattern: ["MM ", "MM ", "   "],
        key: &[('M', RED_MATTER)],
        result: RED_MATTER_BLOCK,
        count: 1,
    },
    // ProjectE `condenser_mk2`: four of each matter block around MK1.
    ShapedRecipe {
        name: "condenser_mk2",
        pattern: ["RDR", "DCD", "RDR"],
        key: &[
            ('R', RED_MATTER_BLOCK),
            ('D', DARK_MATTER_BLOCK),
            ('C', ENERGY_CONDENSER),
        ],
        result: CONDENSER_MK2,
        count: 1,
    },
    // Expansion matter chain: six Aeternalis Fuel + three Red Matter make
    // Magenta Matter; four of those pack into a block. These come from
    // ProjectE's expansion_matter.json custom conversions.
    ShapedRecipe {
        name: "magenta_matter",
        pattern: ["AAA", "RRR", "AAA"],
        key: &[('A', AETERNALIS_FUEL), ('R', RED_MATTER)],
        result: MAGENTA_MATTER,
        count: 1,
    },
    ShapedRecipe {
        name: "magenta_matter_block",
        pattern: ["MM ", "MM ", "   "],
        key: &[('M', MAGENTA_MATTER)],
        result: MAGENTA_MATTER_BLOCK,
        count: 1,
    },
    // ProjectE `condenser_mk3`: five MK2 condensers and four magenta blocks.
    ShapedRecipe {
        name: "condenser_mk3",
        pattern: ["MCM", "CCC", "MCM"],
        key: &[('M', MAGENTA_MATTER_BLOCK), ('C', CONDENSER_MK2)],
        result: CONDENSER_MK3,
        count: 1,
    },
    ShapedRecipe {
        name: "transmutation_tablet",
        pattern: ["DSD", "STS", "DSD"],
        key: &[
            ('D', DARK_MATTER_BLOCK),
            ('S', STONE_BLOCK),
            ('T', TABLE),
        ],
        result: TRANSMUTATION_TABLET,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_pedestal",
        pattern: ["RDR", "RDR", "DDD"],
        key: &[('R', RED_MATTER), ('D', DARK_MATTER_BLOCK)],
        result: DM_PEDESTAL,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_furnace",
        pattern: ["DDD", "DFD", "DDD"],
        key: &[('D', DARK_MATTER_BLOCK), ('F', FURNACE)],
        result: DM_FURNACE,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_furnace",
        pattern: [" R ", "RFR", "   "],
        key: &[('R', RED_MATTER_BLOCK), ('F', DM_FURNACE)],
        result: RM_FURNACE,
        count: 1,
    },
    ShapedRecipe {
        name: "alchemical_barrel",
        pattern: ["LMH", "SDS", "IBI"],
        key: &[
            ('L', LOW_COVALENCE_DUST),
            ('M', MEDIUM_COVALENCE_DUST),
            ('H', HIGH_COVALENCE_DUST),
            ('S', STONE_BLOCK),
            ('D', DIAMOND),
            ('I', IRON),
            ('B', BARREL),
        ],
        result: ALCHEMICAL_BARREL,
        count: 1,
    },
    ShapedRecipe {
        name: "repair_talisman",
        pattern: ["LMH", "SPS", "HML"],
        key: &[
            ('L', LOW_COVALENCE_DUST),
            ('M', MEDIUM_COVALENCE_DUST),
            ('H', HIGH_COVALENCE_DUST),
            ('S', STRING),
            ('P', PAPER),
        ],
        result: REPAIR_TALISMAN,
        count: 1,
    },
    // Three divining rods: eight dust and the previous rod around the centre.
    ShapedRecipe {
        name: "divining_rod_1",
        pattern: ["DDD", "DSD", "DDD"],
        key: &[('D', LOW_COVALENCE_DUST), ('S', STICK)],
        result: DIVINING_ROD_1,
        count: 1,
    },
    ShapedRecipe {
        name: "divining_rod_2",
        pattern: ["DDD", "DSD", "DDD"],
        key: &[('D', MEDIUM_COVALENCE_DUST), ('S', DIVINING_ROD_1)],
        result: DIVINING_ROD_2,
        count: 1,
    },
    ShapedRecipe {
        name: "divining_rod_3",
        pattern: ["DDD", "DSD", "DDD"],
        key: &[('D', HIGH_COVALENCE_DUST), ('S', DIVINING_ROD_2)],
        result: DIVINING_ROD_3,
        count: 1,
    },
    // Phase 1 of the content audit: Dark/Red Matter tools and armour.
    // Recipes and ids are from ProjectE (MIT, github.com/sinkillerj/ProjectE).
    ShapedRecipe {
        name: "dm_axe",
        pattern: ["MM ", "MD ", " D "],
        key: &[('M', DARK_MATTER), ('D', DIAMOND)],
        result: DM_AXE,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_boots",
        pattern: ["M M", "M M", "   "],
        key: &[('M', DARK_MATTER)],
        result: DM_BOOTS,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_chestplate",
        pattern: ["M M", "MMM", "MMM"],
        key: &[('M', DARK_MATTER)],
        result: DM_CHESTPLATE,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_hammer",
        pattern: ["MDM", " D ", " D "],
        key: &[('M', DARK_MATTER), ('D', DIAMOND)],
        result: DM_HAMMER,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_helmet",
        pattern: ["MMM", "M M", "   "],
        key: &[('M', DARK_MATTER)],
        result: DM_HELMET,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_hoe",
        pattern: ["MM ", " D ", " D "],
        key: &[('M', DARK_MATTER), ('D', DIAMOND)],
        result: DM_HOE,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_leggings",
        pattern: ["MMM", "M M", "M M"],
        key: &[('M', DARK_MATTER)],
        result: DM_LEGGINGS,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_pick",
        pattern: ["MMM", " D ", " D "],
        key: &[('M', DARK_MATTER), ('D', DIAMOND)],
        result: DM_PICK,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_shears",
        pattern: [" M ", "D  ", "   "],
        key: &[('M', DARK_MATTER), ('D', DIAMOND)],
        result: DM_SHEARS,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_shovel",
        pattern: ["M  ", "D  ", "D  "],
        key: &[('M', DARK_MATTER), ('D', DIAMOND)],
        result: DM_SHOVEL,
        count: 1,
    },
    ShapedRecipe {
        name: "dm_sword",
        pattern: ["M  ", "M  ", "D  "],
        key: &[('M', DARK_MATTER), ('D', DIAMOND)],
        result: DM_SWORD,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_axe",
        pattern: ["RR ", "RA ", " M "],
        key: &[('R', RED_MATTER), ('A', DM_AXE), ('M', DARK_MATTER)],
        result: RM_AXE,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_boots",
        pattern: ["MDM", "M M", "   "],
        key: &[('M', RED_MATTER), ('D', DM_BOOTS)],
        result: RM_BOOTS,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_chestplate",
        pattern: ["MDM", "MMM", "MMM"],
        key: &[('M', RED_MATTER), ('D', DM_CHESTPLATE)],
        result: RM_CHESTPLATE,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_hammer",
        pattern: ["RMR", " H ", " M "],
        key: &[('R', RED_MATTER), ('H', DM_HAMMER), ('M', DARK_MATTER)],
        result: RM_HAMMER,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_helmet",
        pattern: ["MMM", "MDM", "   "],
        key: &[('M', RED_MATTER), ('D', DM_HELMET)],
        result: RM_HELMET,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_pick",
        pattern: ["RRR", " P ", " M "],
        key: &[('R', RED_MATTER), ('P', DM_PICK), ('M', DARK_MATTER)],
        result: RM_PICK,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_shears",
        pattern: [" R ", "S  ", "   "],
        key: &[('R', RED_MATTER), ('S', DM_SHEARS)],
        result: RM_SHEARS,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_shovel",
        pattern: ["R  ", "S  ", "M  "],
        key: &[('R', RED_MATTER), ('S', DM_SHOVEL), ('M', DARK_MATTER)],
        result: RM_SHOVEL,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_sword",
        pattern: ["R  ", "R  ", "S  "],
        key: &[('R', RED_MATTER), ('S', DM_SWORD)],
        result: RM_SWORD,
        count: 1,
    },
    // Phase 2 of the content audit: rings, relics and shields.
    // Recipes from ProjectE (MIT, github.com/sinkillerj/ProjectE).
    ShapedRecipe {
        name: "iron_band",
        pattern: ["III", "ILI", "III"],
        key: &[('I', IRON), ('L', LAVA_BUCKET)],
        result: IRON_BAND,
        count: 1,
    },
    ShapedRecipe {
        name: "volcanite_amulet",
        pattern: ["LLL", "DDD", "LLL"],
        key: &[('L', LAVA_BUCKET), ('D', DARK_MATTER)],
        result: VOLCANITE_AMULET,
        count: 1,
    },
    ShapedRecipe {
        name: "evertide_amulet",
        pattern: ["WWW", "DDD", "WWW"],
        key: &[('W', WATER_BUCKET), ('D', DARK_MATTER)],
        result: EVERTIDE_AMULET,
        count: 1,
    },
    ShapedRecipe {
        name: "black_hole_band",
        pattern: ["SSS", "DID", "SSS"],
        key: &[('S', STRING), ('D', DARK_MATTER), ('I', IRON_BAND)],
        result: BLACK_HOLE_BAND,
        count: 1,
    },
    ShapedRecipe {
        name: "zero_ring",
        pattern: ["SBS", "MIM", "SBS"],
        key: &[('S', SNOW_BLOCK), ('B', SNOWBALL), ('M', DARK_MATTER), ('I', IRON_BAND)],
        result: ZERO_RING,
        count: 1,
    },
    ShapedRecipe {
        name: "harvest_goddess_band",
        pattern: ["SFS", "DID", "SFS"],
        key: &[('S', OAK_SAPLING), ('F', DANDELION), ('D', DARK_MATTER), ('I', IRON_BAND)],
        result: HARVEST_GODDESS_BAND,
        count: 1,
    },
    ShapedRecipe {
        name: "ignition_ring",
        pattern: ["FMF", "DID", "FMF"],
        key: &[('F', FLINT_AND_STEEL), ('D', DARK_MATTER), ('I', IRON_BAND), ('M', MOBIUS_FUEL)],
        result: IGNITION_RING,
        count: 1,
    },
    ShapedRecipe {
        name: "swiftwolf_rending_gale",
        pattern: ["DFD", "FIF", "DFD"],
        key: &[('D', DARK_MATTER), ('F', FEATHER), ('I', IRON_BAND)],
        result: SWIFTWOLF_RENDING_GALE,
        count: 1,
    },
    ShapedRecipe {
        name: "arcana_ring",
        pattern: ["ZIH", "SMM", "MMM"],
        key: &[('Z', ZERO_RING), ('I', IGNITION_RING), ('H', HARVEST_GODDESS_BAND), ('S', SWIFTWOLF_RENDING_GALE), ('M', RED_MATTER)],
        result: ARCANA_RING,
        count: 1,
    },
    ShapedRecipe {
        name: "gem_of_eternal_density",
        pattern: ["DOD", "MDM", "DOD"],
        key: &[('D', DIAMOND), ('O', OBSIDIAN), ('M', DARK_MATTER)],
        result: GEM_OF_ETERNAL_DENSITY,
        count: 1,
    },
    ShapedRecipe {
        name: "body_stone",
        pattern: ["SSS", "RLR", "SSS"],
        key: &[('S', SUGAR), ('R', RED_MATTER), ('L', LAPIS)],
        result: BODY_STONE,
        count: 1,
    },
    ShapedRecipe {
        name: "soul_stone",
        pattern: ["GGG", "RLR", "GGG"],
        key: &[('G', GLOWSTONE_DUST), ('R', RED_MATTER), ('L', LAPIS)],
        result: SOUL_STONE,
        count: 1,
    },
    ShapedRecipe {
        name: "mind_stone",
        pattern: ["BBB", "RLR", "BBB"],
        key: &[('B', BOOK), ('R', RED_MATTER), ('L', LAPIS)],
        result: MIND_STONE,
        count: 1,
    },
    ShapedRecipe {
        name: "destruction_catalyst",
        pattern: ["NMN", "MFM", "NMN"],
        key: &[('N', NOVA_CATALYST), ('M', MOBIUS_FUEL), ('F', FLINT_AND_STEEL)],
        result: DESTRUCTION_CATALYST,
        count: 1,
    },
    ShapedRecipe {
        name: "hyperkinetic_lens",
        pattern: ["DDD", "MNM", "DDD"],
        key: &[('D', DIAMOND), ('M', DARK_MATTER), ('N', NOVA_CATALYST)],
        result: HYPERKINETIC_LENS,
        count: 1,
    },
    ShapedRecipe {
        name: "catalytic_lens",
        pattern: ["MMM", "DMH", "MMM"],
        key: &[('M', DARK_MATTER), ('D', DESTRUCTION_CATALYST), ('H', HYPERKINETIC_LENS)],
        result: CATALYTIC_LENS,
        count: 1,
    },
    ShapedRecipe {
        name: "mercurial_eye",
        pattern: ["OBO", "BRB", "BDB"],
        key: &[('O', OBSIDIAN), ('B', BRICKS), ('R', RED_MATTER), ('D', DIAMOND)],
        result: MERCURIAL_EYE,
        count: 1,
    },
    ShapedRecipe {
        name: "watch_of_flowing_time",
        pattern: ["DGD", "OCO", "DGD"],
        key: &[('D', DARK_MATTER), ('G', GLOWSTONE), ('O', OBSIDIAN), ('C', CLOCK)],
        result: WATCH_OF_FLOWING_TIME,
        count: 1,
    },
    ShapedRecipe {
        name: "dark_matter_shield",
        pattern: ["PMP", "PPP", " P "],
        key: &[('P', DIAMOND), ('M', DARK_MATTER)],
        result: DARK_MATTER_SHIELD,
        count: 1,
    },
    ShapedRecipe {
        name: "red_matter_shield",
        pattern: ["PMP", "PSP", " P "],
        key: &[('P', DARK_MATTER), ('M', RED_MATTER), ('S', DARK_MATTER_SHIELD)],
        result: RED_MATTER_SHIELD,
        count: 1,
    },

    // Phase 3: tridents, archangel bow, books and infinite items.
    ShapedRecipe {
        name: "dark_matter_trident",
        pattern: ["MTM", " P ", " P "],
        key: &[('M', DARK_MATTER), ('T', TRIDENT), ('P', DIAMOND)],
        result: DARK_MATTER_TRIDENT,
        count: 1,
    },
    ShapedRecipe {
        name: "rm_leggings",
        pattern: ["MMM", "MDM", "M M"],
        key: &[('M', RED_MATTER), ('D', DM_LEGGINGS)],
        result: RM_LEGGINGS,
        count: 1,
    },
    ShapedRecipe {
        name: "red_matter_trident",
        pattern: ["MTM", " P ", " P "],
        key: &[('M', RED_MATTER), ('P', DARK_MATTER), ('T', DARK_MATTER_TRIDENT)],
        result: RED_MATTER_TRIDENT,
        count: 1,
    },
    ShapedRecipe {
        name: "archangel_smite",
        pattern: ["BFB", "MIM", "BFB"],
        key: &[('B', BOW), ('F', FEATHER), ('I', IRON_BAND), ('M', DARK_MATTER)],
        result: ARCHANGEL_SMITE,
        count: 1,
    },
    ShapedRecipe {
        name: "basic_alchemical_book",
        pattern: ["HRH", "EBP", "HRH"],
        key: &[('H', HIGH_COVALENCE_DUST), ('R', RED_MATTER), ('E', ENDER_PEARL), ('B', BOOK), ('P', STONE)],
        result: BASIC_ALCHEMICAL_BOOK,
        count: 1,
    },
    ShapedRecipe {
        name: "infinite_fuel",
        pattern: ["CCC", "CTC", "CCC"],
        key: &[('C', COAL), ('T', TRANSMUTATION_TABLET)],
        result: INFINITE_FUEL,
        count: 1,
    },
    ShapedRecipe {
        name: "infinite_steak",
        pattern: ["SSS", "STS", "SSS"],
        key: &[('S', COOKED_BEEF), ('T', TRANSMUTATION_TABLET)],
        result: INFINITE_STEAK,
        count: 1,
    },
];

/// Layout-free recipes. The stone still appears where the conversion needs it.
pub(crate) const SHAPELESS: &[ShapelessRecipe] = &[
    // Gem armour fuses Red Matter gear with relics and a Klein Star Omega.
    ShapelessRecipe {
        name: "gem_boots",
        ingredients: &[RM_BOOTS, SWIFTWOLF_RENDING_GALE, SWIFTWOLF_RENDING_GALE, KLEIN_STAR_OMEGA],
        result: GEM_BOOTS,
        count: 1,
    },
    ShapelessRecipe {
        name: "gem_chestplate",
        ingredients: &[RM_CHESTPLATE, VOLCANITE_AMULET, BODY_STONE, KLEIN_STAR_OMEGA],
        result: GEM_CHESTPLATE,
        count: 1,
    },
    ShapelessRecipe {
        name: "gem_helmet",
        ingredients: &[RM_HELMET, EVERTIDE_AMULET, SOUL_STONE, KLEIN_STAR_OMEGA],
        result: GEM_HELMET,
        count: 1,
    },
    ShapelessRecipe {
        name: "gem_leggings",
        ingredients: &[RM_LEGGINGS, BLACK_HOLE_BAND, WATCH_OF_FLOWING_TIME, KLEIN_STAR_OMEGA],
        result: GEM_LEGGINGS,
        count: 1,
    },
    // Void Ring: Black Hole Band + Gem of Eternal Density + two Red Matter.
    ShapelessRecipe {
        name: "void_ring",
        ingredients: &[BLACK_HOLE_BAND, GEM_OF_ETERNAL_DENSITY, RED_MATTER, RED_MATTER],
        result: VOID_RING,
        count: 1,
    },
    // Life Stone combines the Body and Soul stones.
    ShapelessRecipe {
        name: "life_stone",
        ingredients: &[BODY_STONE, SOUL_STONE],
        result: LIFE_STONE,
        count: 1,
    },
    // Covalence dusts: 40 per craft, priced so each dust is worth exactly the
    // fraction of its ingredients (1 / 8 / 205 with this crate's base table).
    ShapelessRecipe {
        name: "low_covalence_dust",
        ingredients: &[
            CHARCOAL, COBBLESTONE, COBBLESTONE, COBBLESTONE, COBBLESTONE, COBBLESTONE,
            COBBLESTONE, COBBLESTONE, COBBLESTONE,
        ],
        result: LOW_COVALENCE_DUST,
        count: 40,
    },
    ShapelessRecipe {
        name: "medium_covalence_dust",
        ingredients: &[IRON, REDSTONE],
        result: MEDIUM_COVALENCE_DUST,
        count: 40,
    },
    ShapelessRecipe {
        name: "high_covalence_dust",
        ingredients: &[DIAMOND, COAL],
        result: HIGH_COVALENCE_DUST,
        count: 40,
    },
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
    // ProjectE's reverse fuel conversions: the stone is a catalyst here too.
    ShapelessRecipe {
        name: "alchemical_coal_to_coal",
        ingredients: &[STONE, ALCHEMICAL_COAL],
        result: COAL,
        count: 4,
    },
    ShapelessRecipe {
        name: "mobius_fuel_to_alchemical_coal",
        ingredients: &[STONE, MOBIUS_FUEL],
        result: ALCHEMICAL_COAL,
        count: 4,
    },
    ShapelessRecipe {
        name: "aeternalis_fuel_to_mobius_fuel",
        ingredients: &[STONE, AETERNALIS_FUEL],
        result: MOBIUS_FUEL,
        count: 4,
    },
    // Charcoal and coal convert in both directions through the stone.
    ShapelessRecipe {
        name: "stone_charcoal_to_coal",
        ingredients: &[CHARCOAL, CHARCOAL, CHARCOAL, CHARCOAL, STONE],
        result: COAL,
        count: 1,
    },
    ShapelessRecipe {
        name: "stone_coal_to_charcoal",
        ingredients: &[COAL, COAL, COAL, COAL, STONE],
        result: CHARCOAL,
        count: 1,
    },
    // ProjectE packs nine fuels into a block shapelessly; the 3x3 layout was
    // a local convenience with no gameplay meaning.
    ShapelessRecipe {
        name: "alchemical_coal_block",
        ingredients: &[ALCHEMICAL_COAL; 9],
        result: ALCHEMICAL_COAL_BLOCK,
        count: 1,
    },
    ShapelessRecipe {
        name: "mobius_fuel_block",
        ingredients: &[MOBIUS_FUEL; 9],
        result: MOBIUS_FUEL_BLOCK,
        count: 1,
    },
    ShapelessRecipe {
        name: "aeternalis_fuel_block",
        ingredients: &[AETERNALIS_FUEL; 9],
        result: AETERNALIS_FUEL_BLOCK,
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
    // ProjectE upgrades a Klein Star from exactly four of the previous tier.
    // Stored EMC is conserved by the item component, so no nether star is used.
    ShapelessRecipe {
        name: "klein_star_zwei",
        ingredients: &[KLEIN_STAR_EIN; 4],
        result: KLEIN_STAR_ZWEI,
        count: 1,
    },
    ShapelessRecipe {
        name: "klein_star_drei",
        ingredients: &[KLEIN_STAR_ZWEI; 4],
        result: KLEIN_STAR_DREI,
        count: 1,
    },
    ShapelessRecipe {
        name: "klein_star_vier",
        ingredients: &[KLEIN_STAR_DREI; 4],
        result: KLEIN_STAR_VIER,
        count: 1,
    },
    ShapelessRecipe {
        name: "klein_star_sphere",
        ingredients: &[KLEIN_STAR_VIER; 4],
        result: KLEIN_STAR_SPHERE,
        count: 1,
    },
    ShapelessRecipe {
        name: "klein_star_omega",
        ingredients: &[KLEIN_STAR_SPHERE; 4],
        result: KLEIN_STAR_OMEGA,
        count: 1,
    },
    // ProjectE's stone conversions are all shapeless: the stone is a catalyst,
    // the ingredients carry no layout. (Ids here are this crate's own; the
    // upstream files live under `projecte:recipe/conversions/`.)
    ShapelessRecipe {
        name: "stone_iron_to_gold",
        ingredients: &[IRON, IRON, IRON, IRON, IRON, IRON, IRON, IRON, STONE],
        result: GOLD,
        count: 1,
    },
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
    // ProjectE's alternate Red Matter arrangement, same six fuels and three
    // Dark Matter in three identical columns.
    ShapelessRecipe {
        name: "red_matter_alt",
        ingredients: &[
            AETERNALIS_FUEL,
            AETERNALIS_FUEL,
            DARK_MATTER,
            AETERNALIS_FUEL,
            AETERNALIS_FUEL,
            DARK_MATTER,
            AETERNALIS_FUEL,
            AETERNALIS_FUEL,
            DARK_MATTER,
        ],
        result: RED_MATTER,
        count: 1,
    },
    // Nova catalysts: TNT + Mobius Fuel, then a catalyst + Aeternalis Fuel.
    ShapelessRecipe {
        name: "nova_catalyst",
        ingredients: &[TNT, MOBIUS_FUEL],
        result: NOVA_CATALYST,
        count: 2,
    },
    ShapelessRecipe {
        name: "nova_cataclysm",
        ingredients: &[NOVA_CATALYST, AETERNALIS_FUEL],
        result: NOVA_CATACLYSM,
        count: 2,
    },
    // Matter block deconstruction: the reverse of the two shaped pack recipes.
    ShapelessRecipe {
        name: "dark_matter_block_deconstruct",
        ingredients: &[DARK_MATTER_BLOCK],
        result: DARK_MATTER,
        count: 4,
    },
    ShapelessRecipe {
        name: "red_matter_block_deconstruct",
        ingredients: &[RED_MATTER_BLOCK],
        result: RED_MATTER,
        count: 4,
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
        .map(|(letter, id)| format!("\"{letter}\":\"{id}\""))
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
    fn shaped_patterns_are_rectangular_and_match_their_key() {
        for recipe in SHAPED {
            // Most recipes are 3x3; ProjectE's matter blocks are 2x2.
            let width = recipe.pattern[0].chars().count();
            assert!(width == 2 || width == 3, "{} has an odd width", recipe.name);
            for row in recipe.pattern {
                assert_eq!(
                    row.chars().count(),
                    width,
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
            // ProjectE consumes the stone in both its own crafting and the
            // basic Alchemical Book; every other stone recipe is a catalyst.
            if recipe.name != "philosophers_stone" && recipe.name != "basic_alchemical_book" {
                let listed = CATALYST_RECIPES.contains(&recipe.name);
                let holds_stone = recipe.key.iter().any(|(_, id)| *id == STONE);
                assert_eq!(holds_stone, listed, "{} catalyst mismatch", recipe.name);
            }
        }
        for recipe in SHAPELESS {
            // The basic Alchemical Book is the one recipe where ProjectE
            // genuinely consumes the Philosopher's Stone as an ingredient.
            if recipe.name == "basic_alchemical_book" {
                assert!(recipe.ingredients.contains(&STONE));
                continue;
            }
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
            let pack = SHAPELESS
                .iter()
                .find(|r| r.result == block)
                .expect("block recipe");
            assert_eq!(pack.count, 1);
            assert_eq!(pack.ingredients, &[fuel; 9]);
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
            let value: mod_api::serde_json::Value = mod_api::serde_json::from_str(text)
                .unwrap_or_else(|error| panic!("{key}: {error} in {text}"));
            let kind = value["type"].as_str().expect("recipe type");
            assert!(
                kind == "minecraft:crafting_shaped" || kind == "minecraft:crafting_shapeless",
                "{key}: {kind}"
            );
            let expected_count = match key.as_str() {
                "recipe/projecte:stone_gold_to_iron" => 8,
                "recipe/projecte:stone_diamond_to_gold" => 4,
                "recipe/projecte:stone_emerald_to_diamond" => 2,
                "recipe/projecte:stone_pearl_to_iron" => 4,
                "recipe/projecte:nova_catalyst" | "recipe/projecte:nova_cataclysm" => 2,
                "recipe/projecte:low_covalence_dust"
                | "recipe/projecte:medium_covalence_dust"
                | "recipe/projecte:high_covalence_dust" => 40,
                key if key.ends_with("_from_block") => 9,
                // Four charcoal make one coal; this one ends in `_to_coal`
                // but pays out a single item, unlike the fuel tiers.
                "recipe/projecte:stone_charcoal_to_coal" => 1,
                key
                    if key.ends_with("_deconstruct")
                        || key.ends_with("_to_coal")
                        || key.ends_with("_to_alchemical_coal")
                        || key.ends_with("_to_mobius_fuel") =>
                {
                    4
                }
                _ => 1,
            };
            assert_eq!(
                value["result"]["count"].as_u64(),
                Some(expected_count),
                "{key}"
            );
            if kind == "minecraft:crafting_shaped" {
                let rows = value["pattern"].as_array().map(|rows| rows.len());
                assert!(rows == Some(2) || rows == Some(3), "{key}: {rows:?}");
            } else {
                assert!(value["ingredients"].as_array().is_some(), "{key}");
            }
        }
    }

    fn const_for(name: &str) -> &'static str {
        match name {
            "dm_axe" => DM_AXE,
            "dm_boots" => DM_BOOTS,
            "dm_chestplate" => DM_CHESTPLATE,
            "dm_hammer" => DM_HAMMER,
            "dm_helmet" => DM_HELMET,
            "dm_hoe" => DM_HOE,
            "dm_leggings" => DM_LEGGINGS,
            "dm_pick" => DM_PICK,
            "dm_shears" => DM_SHEARS,
            "dm_shovel" => DM_SHOVEL,
            "dm_sword" => DM_SWORD,
            "rm_axe" => RM_AXE,
            "rm_boots" => RM_BOOTS,
            "rm_chestplate" => RM_CHESTPLATE,
            "rm_hammer" => RM_HAMMER,
            "rm_helmet" => RM_HELMET,
            "rm_pick" => RM_PICK,
            "rm_shears" => RM_SHEARS,
            "rm_shovel" => RM_SHOVEL,
            "rm_sword" => RM_SWORD,
            other => panic!("unknown tool {other}"),
        }
    }

    fn shaped_cost(recipe: &ShapedRecipe) -> u64 {
        recipe
            .pattern
            .iter()
            .flat_map(|row| row.chars())
            .filter_map(|letter| recipe.key.iter().find(|(key, _)| *key == letter))
            .map(|(_, id)| crate::emc::value(id))
            .sum()
    }

    #[test]
    fn condenser_recipe_chain_has_matching_emc_values() {
        let chest = SHAPED
            .iter()
            .find(|recipe| recipe.name == "alchemical_chest")
            .expect("Alchemical Chest recipe");
        assert_eq!(chest.result, ALCHEMICAL_CHEST);
        assert_eq!(shaped_cost(chest), crate::emc::value(ALCHEMICAL_CHEST));
        assert_eq!(shaped_cost(chest), 8_952);

        let condenser = SHAPED
            .iter()
            .find(|recipe| recipe.name == "condenser_mk1")
            .expect("Energy Condenser recipe");
        assert_eq!(condenser.result, ENERGY_CONDENSER);
        assert_eq!(
            shaped_cost(condenser),
            crate::emc::value(ENERGY_CONDENSER)
        );
        assert_eq!(shaped_cost(condenser), 41_976);
    }

    #[test]
    fn covalence_dusts_are_priced_from_their_ingredients() {
        for name in [
            "low_covalence_dust",
            "medium_covalence_dust",
            "high_covalence_dust",
        ] {
            let recipe = SHAPELESS.iter().find(|recipe| recipe.name == name).unwrap();
            let cost: u64 = recipe
                .ingredients
                .iter()
                .map(|id| crate::emc::value(id))
                .sum();
            // ProjectE truncates a fractional per-dust value rather than
            // rounding it up into a profitable recipe.
            assert_eq!(crate::emc::value(recipe.result), cost / recipe.count as u64, "{name}");
        }
    }

    #[test]
    fn files_are_deterministic() {
        assert_eq!(files(), files());
    }

    #[test]
    fn klein_star_upgrades_use_exactly_four_of_the_previous_tier() {
        for (name, previous, result) in [
            ("klein_star_zwei", KLEIN_STAR_EIN, KLEIN_STAR_ZWEI),
            ("klein_star_drei", KLEIN_STAR_ZWEI, KLEIN_STAR_DREI),
            ("klein_star_vier", KLEIN_STAR_DREI, KLEIN_STAR_VIER),
            ("klein_star_sphere", KLEIN_STAR_VIER, KLEIN_STAR_SPHERE),
            ("klein_star_omega", KLEIN_STAR_SPHERE, KLEIN_STAR_OMEGA),
        ] {
            let recipe = SHAPELESS
                .iter()
                .find(|recipe| recipe.name == name)
                .unwrap_or_else(|| panic!("{name} is missing"));
            assert_eq!(recipe.result, result);
            assert_eq!(recipe.ingredients, &[previous; 4], "{name}");
        }
    }

    #[test]
    fn matter_recipes_follow_the_reference() {
        let dark = SHAPED
            .iter()
            .find(|recipe| recipe.name == "dark_matter")
            .expect("dark matter recipe");
        assert_eq!(dark.pattern, ["AAA", "ADA", "AAA"]);
        assert_eq!(dark.key, &[('A', AETERNALIS_FUEL), ('D', DIAMOND_BLOCK)]);

        let red = SHAPED
            .iter()
            .find(|recipe| recipe.name == "red_matter")
            .expect("red matter recipe");
        assert_eq!(red.pattern, ["AAA", "DDD", "AAA"]);
        assert_eq!(red.key, &[('A', AETERNALIS_FUEL), ('D', DARK_MATTER)]);

        for (name, unpack_name, matter, block) in [
            (
                "dark_matter_block",
                "dark_matter_block_deconstruct",
                DARK_MATTER,
                DARK_MATTER_BLOCK,
            ),
            (
                "red_matter_block",
                "red_matter_block_deconstruct",
                RED_MATTER,
                RED_MATTER_BLOCK,
            ),
        ] {
            let pack = SHAPED.iter().find(|recipe| recipe.name == name).unwrap();
            assert_eq!(pack.pattern, ["MM ", "MM ", "   "]);
            assert_eq!(pack.key, &[('M', matter)]);
            assert_eq!(pack.result, block);

            let unpack = SHAPELESS
                .iter()
                .find(|recipe| recipe.name == unpack_name)
                .unwrap();
            assert_eq!(unpack.ingredients, &[block]);
            assert_eq!(unpack.result, matter);
            assert_eq!(unpack.count, 4);
        }
    }

    #[test]
    fn machine_patterns_match_the_reference_layout() {
        let collector = SHAPED
            .iter()
            .find(|recipe| recipe.name == "collector_mk1")
            .unwrap();
        assert_eq!(collector.pattern, ["GTG", "GDG", "GFG"]);
        let relay = SHAPED
            .iter()
            .find(|recipe| recipe.name == "relay_mk1")
            .unwrap();
        assert_eq!(relay.pattern, ["OSO", "ODO", "OOO"]);
        for name in ["collector_mk2", "collector_mk3"] {
            let upgrade = SHAPED.iter().find(|recipe| recipe.name == name).unwrap();
            assert_eq!(upgrade.pattern, ["GUG", "GPG", "GGG"], "{name}");
        }
        for name in ["relay_mk2", "relay_mk3"] {
            let upgrade = SHAPED.iter().find(|recipe| recipe.name == name).unwrap();
            assert_eq!(upgrade.pattern, ["OUO", "OPO", "OOO"], "{name}");
        }
    }

    #[test]
    fn every_stone_conversion_is_shapeless() {
        for recipe in SHAPED {
            assert!(
                !recipe.name.starts_with("stone_"),
                "{} must be shapeless",
                recipe.name
            );
        }
        assert!(
            SHAPELESS
                .iter()
                .any(|recipe| recipe.name == "stone_iron_to_gold"),
            "the iron-to-gold conversion must be present and layout-free"
        );
    }

    #[test]
    fn reverse_fuel_and_charcoal_conversions_match_the_reference() {
        for (name, input, output) in [
            ("alchemical_coal_to_coal", ALCHEMICAL_COAL, COAL),
            (
                "mobius_fuel_to_alchemical_coal",
                MOBIUS_FUEL,
                ALCHEMICAL_COAL,
            ),
            (
                "aeternalis_fuel_to_mobius_fuel",
                AETERNALIS_FUEL,
                MOBIUS_FUEL,
            ),
        ] {
            let recipe = SHAPELESS
                .iter()
                .find(|recipe| recipe.name == name)
                .unwrap_or_else(|| panic!("{name} is missing"));
            assert_eq!(recipe.ingredients, &[STONE, input], "{name}");
            assert_eq!(recipe.result, output);
            assert_eq!(recipe.count, 4);
        }
        for (name, input, output) in [
            ("stone_charcoal_to_coal", CHARCOAL, COAL),
            ("stone_coal_to_charcoal", COAL, CHARCOAL),
        ] {
            let recipe = SHAPELESS
                .iter()
                .find(|recipe| recipe.name == name)
                .unwrap_or_else(|| panic!("{name} is missing"));
            assert_eq!(recipe.ingredients, &[input, input, input, input, STONE]);
            assert_eq!(recipe.result, output);
            assert_eq!(recipe.count, 1);
        }
    }

    #[test]
    fn alternate_stone_and_matter_recipes_use_the_reference_ingredients() {
        let stone_alt = SHAPED
            .iter()
            .find(|recipe| recipe.name == "philosophers_stone_alt")
            .expect("alternate Philosopher's Stone recipe");
        assert_eq!(stone_alt.pattern, ["GRG", "RDR", "GRG"]);
        assert_eq!(stone_alt.result, STONE);

        let red_alt = SHAPELESS
            .iter()
            .find(|recipe| recipe.name == "red_matter_alt")
            .expect("alternate Red Matter recipe");
        assert_eq!(
            red_alt
                .ingredients
                .iter()
                .filter(|id| **id == AETERNALIS_FUEL)
                .count(),
            6
        );
        assert_eq!(
            red_alt
                .ingredients
                .iter()
                .filter(|id| **id == DARK_MATTER)
                .count(),
            3
        );
        assert_eq!(red_alt.result, RED_MATTER);
    }

    #[test]
    fn newly_ported_machines_and_tools_have_reference_recipes() {
        let spec = |name: &'static str| {
            SHAPED
                .iter()
                .find(|recipe| recipe.name == name)
                .unwrap_or_else(|| panic!("{name} is missing"))
        };
        let condenser = spec("condenser_mk2");
        assert_eq!(condenser.pattern, ["RDR", "DCD", "RDR"]);
        assert_eq!(condenser.result, CONDENSER_MK2);
        assert_eq!(spec("transmutation_tablet").pattern, ["DSD", "STS", "DSD"]);
        assert_eq!(spec("condenser_mk3").pattern, ["MCM", "CCC", "MCM"]);
        assert_eq!(spec("magenta_matter").pattern, ["AAA", "RRR", "AAA"]);
        assert_eq!(spec("magenta_matter_block").pattern, ["MM ", "MM ", "   "]);
        assert_eq!(spec("dm_pedestal").pattern, ["RDR", "RDR", "DDD"]);
        assert_eq!(spec("dm_furnace").pattern, ["DDD", "DFD", "DDD"]);
        assert_eq!(spec("rm_furnace").pattern, [" R ", "RFR", "   "]);
        assert_eq!(spec("alchemical_barrel").pattern, ["LMH", "SDS", "IBI"]);
        assert_eq!(spec("repair_talisman").pattern, ["LMH", "SPS", "HML"]);
        for (name, result) in [
            ("divining_rod_1", DIVINING_ROD_1),
            ("divining_rod_2", DIVINING_ROD_2),
            ("divining_rod_3", DIVINING_ROD_3),
        ] {
            assert_eq!(spec(name).pattern, ["DDD", "DSD", "DDD"]);
            assert_eq!(spec(name).result, result);
        }
        for (name, first, second) in [
            ("nova_catalyst", TNT, MOBIUS_FUEL),
            ("nova_cataclysm", NOVA_CATALYST, AETERNALIS_FUEL),
        ] {
            let recipe = SHAPELESS
                .iter()
                .find(|recipe| recipe.name == name)
                .unwrap_or_else(|| panic!("{name} is missing"));
            assert_eq!(recipe.ingredients, &[first, second]);
            assert_eq!(recipe.count, 2);
        }
    }

    #[test]
    fn newly_ported_values_match_their_recipe_costs() {
        for name in [
            "condenser_mk2",
            "condenser_mk3",
            "magenta_matter",
            "magenta_matter_block",
            "transmutation_tablet",
            "dm_pedestal",
            "dm_furnace",
            "rm_furnace",
            "alchemical_barrel",
            "repair_talisman",
            "divining_rod_1",
            "divining_rod_2",
            "divining_rod_3",
        ] {
            let recipe = SHAPED.iter().find(|recipe| recipe.name == name).unwrap();
            assert_eq!(
                shaped_cost(recipe),
                crate::emc::value(recipe.result),
                "{name}"
            );
        }
        for name in ["nova_catalyst", "nova_cataclysm"] {
            let recipe = SHAPELESS.iter().find(|recipe| recipe.name == name).unwrap();
            let cost: u64 = recipe
                .ingredients
                .iter()
                .map(|id| crate::emc::value(id))
                .sum();
            assert_eq!(
                cost,
                crate::emc::value(recipe.result) * recipe.count as u64,
                "{name}"
            );
        }
    }
    #[test]
    fn matter_tool_and_armour_recipes_are_emc_neutral() {
        for name in [
            "dm_axe",
            "dm_boots",
            "dm_chestplate",
            "dm_hammer",
            "dm_helmet",
            "dm_hoe",
            "dm_leggings",
            "dm_pick",
            "dm_shears",
            "dm_shovel",
            "dm_sword",
            "rm_axe",
            "rm_boots",
            "rm_chestplate",
            "rm_hammer",
            "rm_helmet",
            "rm_pick",
            "rm_shears",
            "rm_shovel",
            "rm_sword",
        ] {
            let recipe = SHAPED
                .iter()
                .find(|recipe| recipe.name == name)
                .unwrap_or_else(|| panic!("{name} is missing"));
            assert_eq!(recipe.result, const_for(name), "{name}");
            assert_eq!(shaped_cost(recipe), crate::emc::value(recipe.result), "{name}");
        }
    }

    #[test]
    fn rings_relics_and_shields_are_emc_neutral() {
        for (name, result) in [
            ("iron_band", IRON_BAND),
            ("volcanite_amulet", VOLCANITE_AMULET),
            ("evertide_amulet", EVERTIDE_AMULET),
            ("black_hole_band", BLACK_HOLE_BAND),
            ("zero_ring", ZERO_RING),
            ("harvest_goddess_band", HARVEST_GODDESS_BAND),
            ("ignition_ring", IGNITION_RING),
            ("swiftwolf_rending_gale", SWIFTWOLF_RENDING_GALE),
            ("arcana_ring", ARCANA_RING),
            ("gem_of_eternal_density", GEM_OF_ETERNAL_DENSITY),
            ("body_stone", BODY_STONE),
            ("soul_stone", SOUL_STONE),
            ("mind_stone", MIND_STONE),
            ("destruction_catalyst", DESTRUCTION_CATALYST),
            ("hyperkinetic_lens", HYPERKINETIC_LENS),
            ("catalytic_lens", CATALYTIC_LENS),
            ("mercurial_eye", MERCURIAL_EYE),
            ("watch_of_flowing_time", WATCH_OF_FLOWING_TIME),
            ("dark_matter_shield", DARK_MATTER_SHIELD),
            ("red_matter_shield", RED_MATTER_SHIELD),
        ] {
            let recipe = SHAPED.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(recipe.result, result, "{name}");
            assert_eq!(shaped_cost(recipe), crate::emc::value(result), "{name}");
        }
        for (name, result) in [("void_ring", VOID_RING), ("life_stone", LIFE_STONE)] {
            let recipe = SHAPELESS.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{name}"));
            let cost: u64 = recipe.ingredients.iter().map(|id| crate::emc::value(id)).sum();
            assert_eq!(recipe.result, result, "{name}");
            assert_eq!(cost, crate::emc::value(result), "{name}");
        }
    }

    #[test]
    fn phase_three_recipes_are_emc_neutral() {
        let r = SHAPED.iter().find(|r| r.name == "dark_matter_trident").unwrap();
        assert_eq!(r.result, DARK_MATTER_TRIDENT, "dark_matter_trident");
        assert_eq!(shaped_cost(r), crate::emc::value(r.result), "dark_matter_trident");
        let r = SHAPED.iter().find(|r| r.name == "rm_leggings").unwrap();
        assert_eq!(r.result, RM_LEGGINGS, "rm_leggings");
        assert_eq!(shaped_cost(r), crate::emc::value(r.result), "rm_leggings");
        let r = SHAPED.iter().find(|r| r.name == "red_matter_trident").unwrap();
        assert_eq!(r.result, RED_MATTER_TRIDENT, "red_matter_trident");
        assert_eq!(shaped_cost(r), crate::emc::value(r.result), "red_matter_trident");
        let r = SHAPED.iter().find(|r| r.name == "archangel_smite").unwrap();
        assert_eq!(r.result, ARCHANGEL_SMITE, "archangel_smite");
        assert_eq!(shaped_cost(r), crate::emc::value(r.result), "archangel_smite");
        let r = SHAPED.iter().find(|r| r.name == "basic_alchemical_book").unwrap();
        assert_eq!(r.result, BASIC_ALCHEMICAL_BOOK, "basic_alchemical_book");
        assert_eq!(shaped_cost(r), crate::emc::value(r.result), "basic_alchemical_book");
        let r = SHAPED.iter().find(|r| r.name == "infinite_fuel").unwrap();
        assert_eq!(r.result, INFINITE_FUEL, "infinite_fuel");
        assert_eq!(shaped_cost(r), crate::emc::value(r.result), "infinite_fuel");
        let r = SHAPED.iter().find(|r| r.name == "infinite_steak").unwrap();
        assert_eq!(r.result, INFINITE_STEAK, "infinite_steak");
        assert_eq!(shaped_cost(r), crate::emc::value(r.result), "infinite_steak");
        let r = SHAPELESS.iter().find(|r| r.name == "gem_boots").unwrap();
        assert_eq!(r.result, GEM_BOOTS, "gem_boots");
        let cost: u64 = r.ingredients.iter().map(|id| crate::emc::value(id)).sum();
        assert_eq!(cost, crate::emc::value(r.result), "gem_boots");
        let r = SHAPELESS.iter().find(|r| r.name == "gem_chestplate").unwrap();
        assert_eq!(r.result, GEM_CHESTPLATE, "gem_chestplate");
        let cost: u64 = r.ingredients.iter().map(|id| crate::emc::value(id)).sum();
        assert_eq!(cost, crate::emc::value(r.result), "gem_chestplate");
        let r = SHAPELESS.iter().find(|r| r.name == "gem_helmet").unwrap();
        assert_eq!(r.result, GEM_HELMET, "gem_helmet");
        let cost: u64 = r.ingredients.iter().map(|id| crate::emc::value(id)).sum();
        assert_eq!(cost, crate::emc::value(r.result), "gem_helmet");
        let r = SHAPELESS.iter().find(|r| r.name == "gem_leggings").unwrap();
        assert_eq!(r.result, GEM_LEGGINGS, "gem_leggings");
        let cost: u64 = r.ingredients.iter().map(|id| crate::emc::value(id)).sum();
        assert_eq!(cost, crate::emc::value(r.result), "gem_leggings");
    }

}
