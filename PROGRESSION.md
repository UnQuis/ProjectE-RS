# ProjectE-RS progression

This document is the end-to-end plan for the mod's Equivalent Exchange
progression, and the record of how much of it exists today. It is written
against the pinned Horizon revision in `horizon-mixin.toml`
(`fdc87aec…`, native hooks API 411) and Minecraft 26.3.

The machine-readable half of this roadmap lives in `engine/src/progression.rs`.
Every node below has an entry there with its status, prerequisites and Horizon
dependencies, and the tests in that file check the roadmap is ordered and that
"implemented" nodes really have items and recipes. `/projecte roadmap` prints
the same list in game.

Legend for status:

| Status | Meaning |
| --- | --- |
| **Implemented** | Item, recipe and behavior exist at the pinned API. |
| **Partial** | Something playable exists, but the node is incomplete. |
| **Missing** | Nothing of the node exists yet. |
| **Blocked** | The item may exist, but the behavior needs a Horizon capability the pinned API does not expose. |

Recipes marked ⚠ are provisional: the public behavior is either unclear or was
recorded from a secondary source and has not been confirmed in game. They are
deliberate placeholders, not claims.

## 1. Progression at a glance

| Stage | Nodes | State |
| --- | --- | --- |
| 0 — Foundation | Philosopher's Stone, Transmutation Table, EMC and knowledge | Stone implemented; table partial |
| 1 — Alchemy | Alchemical Coal → Mobius Fuel → Aeternalis Fuel, fuel blocks, burn time | Fuels and blocks implemented; burn time blocked |
| 2 — Machines | Collectors, Relays, Klein Stars, Condensers, Power Flower, Alchemical Chest, data-driven EMC | Collectors/Relays/Stars partial; the rest missing |
| 3 — Matter | Dark Matter, Red Matter, matter blocks, tools, armor, Tablet, Tome | Matter items partial; everything built on them missing |
| 4 — Endgame | Rings, amulets, bags, catalysts, advancements | Missing |

The dependency shape is: the stone gates everything; fuels gate the machines;
machines gate matter through EMC; matter gates tools and armor; the endgame
utility items sit on both matter tiers.

## 2. Stage 0 — the foundation

### 2.1 Philosopher's Stone — implemented

- **Ingredients:** 4 Redstone, 4 Glowstone Dust, 1 Diamond in a checkered ring.
- **Recipe:** shaped 3x3, `RGR / GDG / RGR`.
- **Result:** `projecte:philosophers_stone`, a single non-stacking item.
- **Prerequisites:** none. This is the entry point.
- **Place in progression:** the crafting catalyst for fuels, machine
  components and every conversion below; also the key that opens the
  Transmutation Table when used in the air.
- **Capabilities:**
  - *Crafting conversions* (stage 0.2) in a crafting grid.
  - *World transmutation*: right-clicking a block swaps it for a partner block;
    sneaking selects the second option where one exists.
  - *Table access*: using the stone opens the Transmutation Table UI.
- **Horizon dependencies:** `items`, `data_pack_files`, `item_crafted`,
  `use_item_on`, `use_item`, and the `edit_blocks` helper.
- **Workarounds:** the stone is never consumed, which data-pack recipes cannot
  express. The recipes list it as an ingredient and `item_crafted` returns one
  stone (API_WORKAROUNDS §5). World transmutation writes blocks through
  `edit_blocks` because no name-to-state-id helper is exported
  (API_WORKAROUNDS §6).

### 2.2 Stone crafting conversions — implemented

Both directions exist so an exchange never creates or destroys EMC.

| From | To | Recipe form | EMC |
| --- | --- | --- | --- |
| 8 Iron Ingot | 1 Gold Ingot | shaped, iron ring around the stone | 2048 → 2048 |
| 1 Gold Ingot | 8 Iron Ingot | shapeless + stone | 2048 → 2048 |
| 4 Gold Ingot | 1 Diamond | shapeless + stone | 8192 → 8192 |
| 1 Diamond | 4 Gold Ingot | shapeless + stone | 8192 → 8192 |
| 2 Diamond | 1 Emerald | shapeless + stone | 16384 → 16384 |
| 1 Emerald | 2 Diamond | shapeless + stone | 16384 → 16384 |
| 4 Iron Ingot | 1 Ender Pearl | shapeless + stone | 1024 → 1024 |
| 1 Ender Pearl | 4 Iron Ingot | shapeless + stone | 1024 → 1024 |

The 8-iron ring is the documented layout; the other conversions are shapeless
because the public material does not describe a grid shape for them ⚠.

### 2.3 World transmutation — implemented

Deterministic and server-authoritative: the block that was clicked must still be
the block that resolves, or nothing changes (the edit is filtered on it).
Free, as public behavior describes; the stone is not consumed.

| Block | Result | Sneaking |
| --- | --- | --- |
| Stone | Cobblestone | Grass Block |
| Cobblestone | Stone | — |
| Dirt | Sand | Clay ⚠ |
| Grass Block | Sand | Dirt |
| Sand | Sandstone ⚠ | Glass ⚠ |
| Gravel | Cobblestone ⚠ | Sand ⚠ |
| Water | Ice | — |
| Ice | Water | — |
| Lava | Obsidian | — |
| Netherrack | Nether Bricks ⚠ | Soul Sand ⚠ |
| End Stone | Obsidian ⚠ | Purpur Block ⚠ |
| Oak/Spruce/Birch/Jungle/Acacia/Dark Oak/Mangrove/Cherry logs | next family in that order, wrapping | previous family |
| The same eight families, planks | next family, wrapping | previous family |
| All 16 wool colours and carpets | next colour, wrapping | previous colour |

Stone→cobblestone, dirt/grass→sand, water→ice and lava→obsidian, plus
"logs, planks and wool cycle through their types and colors", are documented
public behavior. The remaining pairs are our own deterministic choices, marked
⚠, and can be changed without touching the transform machinery.

**Not implemented:** the area charge (a bigger region per click) and the
cube/panel/line shape selection. Both need input hooks and a client-side
selection view; see §7.

### 2.4 Transmutation Table — partial

- **Ingredients:** 4 Obsidian (corners), 4 Stone (sides), 1 Philosopher's Stone
  (centre); the stone is returned.
- **Recipe:** shaped 3x3, `OSO / SPS / OSO`.
- **Result:** `projecte:transmutation_table`, a block with an item form.
- **Prerequisites:** Philosopher's Stone.
- **Place in progression:** the hub. Everything that spends or learns EMC goes
  through it.
- **Implemented today:** the 54-slot menu, bought items charged through
  `transaction::plan_purchase`/`settle_purchase`, EMC balance in a player
  attachment, learned-item list.
- **Missing:** *depositing* an item to learn it and gain its EMC (the documented
  first-EMC step), an unlearn slot, and a client screen with search, categories
  and paging. The current 54-slot chest menu is the bootstrap UI.
- **Horizon dependencies:** `blocks`, `open_menu`, `menu_clicked`,
  `player_stacks`, `give_stack`, attachments with a codec.
- **Workarounds:** none for the menu itself. EMC values are static, see §6.

### 2.5 EMC and knowledge — partial

- **Capability:** a per-player EMC balance and learned set, server-authoritative
  and persisted through an attachment codec (`copy_on_death = true`,
  `sync = false`).
- **Implemented today:** `/projecte emc`, `/projecte emc set` (operator),
  `/projecte learn` (held item, named item, or `all` carried items),
  `/projecte transmute`, `/projecte value`, `/projecte reset`, and learning by
  buying. Zero-EMC items are refused everywhere.
- **Missing:** learning by depositing items into the table; unlearning;
  knowledge sharing items (Tome of Knowledge); any client-side view of the
  learned set outside the table menu.
- **Horizon dependencies:** attachment `get`/`set` with a codec; command hooks.
- **Known gap:** `data::get`/`data::set` in `engine/src/data.rs` still collapse
  attachment errors into defaults. That predates this phase and is recorded as
  a cleanup item rather than copied into new code.

## 3. Stage 1 — alchemical fuels

### 3.1 Alchemical Coal, Mobius Fuel, Aeternalis Fuel — implemented

| Recipe | Ingredients | Result | EMC in → out |
| --- | --- | --- | --- |
| Alchemical Coal | 4 Coal + Philosopher's Stone | 1 Alchemical Coal | 128 → 128 |
| Mobius Fuel | 4 Alchemical Coal + Stone | 1 Mobius Fuel | 512 → 512 |
| Aeternalis Fuel | 4 Mobius Fuel + Stone | 1 Aeternalis Fuel | 2048 → 2048 |

- **Recipe form:** shapeless; the documented behavior names ingredients, not a
  layout, and the stone is a catalyst (§5 of API_WORKAROUNDS).
- **Prerequisites:** Philosopher's Stone.
- **Place in progression:** the food of every machine, and the item matter is
  built from. Each tier is exactly four times the previous one, so the chain
  moves no EMC: it compresses value, it does not create it.
- **Burn times (metadata only):** 1,600 / 6,400 / 25,600 / 102,400 ticks for
  coal and the three tiers.

### 3.2 Fuel blocks — implemented

- **Ingredients:** nine of one fuel in a 3x3 pack; unpacking returns nine.
- **Result:** `projecte:alchemical_coal_block`,
  `projecte:mobius_fuel_block`, `projecte:aeternalis_fuel_block`, each a block
  with an item form. EMC is exactly nine fuels (1152 / 4608 / 18432).
- **Prerequisites:** the matching fuel.
- **Place in progression:** compact storage, and an ingredient for later
  machine tiers.
- **Horizon dependencies:** `blocks` plus `data_pack_files`.

### 3.3 Fuel burn time — blocked

Fuels should burn in a vanilla furnace for four times the fuel they came from.
The pinned API cannot express this: `furnace_burn` is a **listen-only** hook and
no helper sets an item's burn time, so the numbers live in
`alchemical::BURN_TICKS` as metadata for a later integration. Do not fake this
with a furnace tick loop that eats items: that duplicates vanilla smelting
state and desynchronises the furnace's own fuel slot.

## 4. Stage 2 — machines

### 4.1 Energy Collectors (MK1–MK3) — partial

- **Ingredients (MK1):** 7 Glowstone, 1 Diamond Block, 1 Furnace in the
  documented ring/diamond-core/furnace shape; **MK2** is MK1 + Glowstone ring +
  Dark Matter; **MK3** is MK2 + Glowstone ring + Red Matter ⚠.
- **Recipes:** shaped 3x3.
- **Prerequisites:** Transmutation Table, fuels.
- **Implemented today:** three tiers, 4/12/40 EMC per second in full light with
  fractional accumulation, capacities 10,000/30,000/60,000, machine slots,
  Klein Star charging, collection into the player's balance.
- **Missing:** fuel upgrading inside the collector's slots; light-level
  sensitivity (the current rate assumes full light); MK2/MK3 recipe confirmation.
- **Horizon dependencies:** `block_entity`, `block_entity_ticks`,
  `block_entity_container`, block-entity NBT.
- **Workarounds:** native block energy is not callable, so EMC lives in the
  block entity's NBT (API_WORKAROUNDS §1).

### 4.2 Anti-Matter Relays (MK1–MK3) — partial

- **Ingredients (MK1):** 7 Obsidian, 1 Glass, 1 Diamond Block; MK2/MK3 add Dark
  and Red Matter the way the collectors do ⚠.
- **Prerequisites:** Energy Collector.
- **Implemented today:** capacities 100,000/1,000,000/10,000,000, transfer
  limits 64/192/640 EMC per tick, transfer bonuses, relay-to-relay routing that
  only feeds a fuller-to-emptier target.
- **Missing:** Condenser and player-inventory targets; a documented answer for
  relay-to-relay ping-pong beyond the fill-ratio rule.
- **Horizon dependencies:** `change_mod_block_entity` for neighbour writes.
- **Workarounds:** same NBT backend as collectors (API_WORKAROUNDS §1, §3).

### 4.3 Klein Stars (six tiers) — partial

- **Ingredients:** Ein is 8 Diamonds around Glowstone ⚠; Zwei/Drei/Vier are four
  of the previous tier plus a Nether Star. Sphere and Omega exist as items but
  have no recipe verified yet.
- **Capacities:** 50,000 → 51,200,000, four times per tier.
- **Implemented today:** item registration for six tiers, persistent charge,
  charging and draining through collector and relay slots, tooltip.
- **Missing:** direct player-inventory charge/discharge, Sphere/Omega recipes,
  and native item energy.
- **Horizon dependencies:** item components (`component_types`, `component`,
  `set_component`, `remove_component`).
- **Workarounds:** `item_energy_buffer` is not callable from a trait mod, so
  charge uses a ProjectE component (API_WORKAROUNDS §2).

### 4.4 Energy Condenser MK1/MK2 — missing

- **Ingredients (MK1):** 4 Obsidian (corners), 4 Diamonds (sides), 1 Alchemical
  Chest (centre); MK2 is a ring of Red and Dark Matter blocks around MK1 ⚠.
- **Prerequisites:** Relay, Transmutation Table, Alchemical Chest.
- **Planned behavior:** a target item in the top slot is duplicated whenever the
  condenser has its EMC value; everything else inside is burned for EMC; EMC can
  also be piped in from adjacent collectors and relays.
- **Horizon dependencies:** `block_entity_container`, block-entity NBT, and the
  existing transaction planner for the purchase step.
- **Note:** this is the node that turns a machine grid into a real Power Flower,
  so it comes before the build pattern in the roadmap even though players
  usually describe the flower first.

### 4.5 Alchemical Chest — missing

- **Ingredients:** not yet verified ⚠.
- **Planned behavior:** a large chest that the Condenser and later machines can
  read and write.
- **Horizon dependencies:** `block_entity_container`.

### 4.6 Power Flower — missing

Not a block: a documented build pattern (collectors feeding relays feeding a
condenser). It becomes reachable once §4.4 exists; no new API is required, only
a player-facing explanation in the tooltip or guide item.

### 4.7 Data-map-driven EMC — blocked

`data_map_types` can register `projecte:emc` for `minecraft:item` and a pack can
ship `data/projecte/data_maps/item/emc.json`, but the pinned `Host` has no
read helper for the merged map (`native_data_map` is internal). Until Horizon
exports one, EMC stays in the static table plus vanilla-only family rules, and
we ship no data-map file rather than implying pack overrides work. See
API_WORKAROUNDS §7.

## 5. Stage 3 — matter

### 5.1 Dark Matter — partial

- **Ingredients:** currently 8 Coal + 1 Diamond, inherited from the prototype ⚠.
  Public behavior ties Dark Matter to Aeternalis Fuel and a Diamond Block; the
  exact shape is unconfirmed, so the recipe is **not** changed in this phase.
- **Place in progression:** unlocks the matter machines and the first tools.
- **EMC:** 16,384 today, below the value of its expected fuel inputs. This is
  the largest known value/recipe contradiction in the table and is the first
  item the balance pass in §8 must fix.

### 5.2 Red Matter — partial

- **Ingredients:** currently 8 Dark Matter + 1 Diamond Block ⚠, same caveat.
- **EMC:** 131,072 (8 × 16,384, consistent with the current recipe).

### 5.3 Matter blocks — missing

Nine matter of one tier in a 3x3 pack, with the reverse unpack, exactly like the
fuel blocks. Needed by the Transmutation Tablet and the Condenser MK2.

### 5.4 Dark/Red Matter tools — missing

The full tool set per tier (sword, pickaxe, axe, shovel, hoe, shears, hammer,
Katar, Morningstar), with area mining and the tier's special attack. Horizon
dependencies: item components for damage/tool data, `destroy_block`,
`item_damaged`. Balance is deliberately untouched in this phase.

### 5.5 Dark/Red Matter armor and Gem armor — missing

Armor that spends EMC instead of taking durability damage, with the gem set
adding flight, water walking, lava protection and fall immunity. Horizon
dependencies: `on_equip`, `living_incoming_damage`, `living_equipment`; the
damage-prevention path needs the cancelling form of the damage hook.

### 5.6 Transmutation Tablet — missing

- **Ingredients:** 4 Dark Matter Blocks, 4 Stone, 1 Transmutation Table ⚠.
- **Planned behavior:** the portable table; opens the same EMC balance and
  learned list from anywhere.
- **Horizon dependencies:** `use_item` plus the existing menu path. No new
  capability needed, which makes this a good first matter-tier item.

### 5.7 Tome of Knowledge — missing

- **Ingredients:** 2 Klein Star Omega, 1 Book, the three Covalence Dusts ⚠.
- **Planned behavior:** learns every item at once. Disabled by default in the
  original mod's config; we should decide the default explicitly rather than
  inherit it silently.

## 6. Stage 4 — endgame

Rings and amulets (Zero Ring, Ring of Ignition, Swiftwolf's Rending Gale,
Harvest Goddess Band, Black Hole Band, Evertide and Volcanite amulets), the
Destruction Catalyst, the Gem of Eternal Density, Alchemical Bags, and the
advancement tree. All are **missing**.

- **Horizon dependencies:** player movement and flight, fluid placement,
  item pickup, portable containers, and EMC spending from item use. Some
  abilities (creative-style flight, fluid conversion in the world) depend on
  hooks that need to be checked one by one against the pinned catalog before
  committing to a design.
- **Advancements:** data-pack advancement files through `data_pack_files`, with
  the catalog's advancement hooks for criteria we cannot express in data.

## 7. Recipes: shaped versus shapeless, and what changed

The prototype emitted every recipe as `crafting_shapeless`. Layout now matters
where players read it, so recipes are split in `engine/src/recipes.rs`:

| Recipe | Before | Now | Why |
| --- | --- | --- | --- |
| Philosopher's Stone | shapeless, 4 glowstone dust + diamond + redstone | shaped checkered ring `RGR/GDG/RGR` | documented layout |
| Transmutation Table | shapeless, 8 obsidian + stone | shaped `OSO/SPS/OSO` | documented layout |
| Energy Collector MK1 | shapeless, 8 glowstone + diamond | shaped ring + diamond block + furnace | documented layout |
| Anti-Matter Relay MK1 | shapeless, 8 obsidian + collector | shaped ring + glass + diamond block | documented layout |
| Collector/Relay MK2–MK3 | no recipe | shaped upgrade around the previous tier ⚠ | progression needs an upgrade path |
| Klein Star Ein | shapeless, 8 diamond + glowstone | shaped ring + glowstone centre ⚠ | a 9-ingredient recipe reads as a ring |
| Fuel chain | shapeless, 4 fuels + redstone/glowstone dust → **4** fuels | shapeless, 4 fuels + stone → **1** fuel | documented 4:1 chain, EMC-neutral |
| Fuel blocks | none | shaped 3x3 pack + shapeless unpack | new content |
| Stone conversions | none | shaped 8-iron ring, shapeless for the rest | new content |
| Klein upgrades, matter | shapeless | unchanged, reviewed | layout not documented yet ⚠ |

Two consequences are deliberate and documented rather than tuned away:

1. The fuel recipes no longer multiply output fourfold, so they cannot create
   EMC (the old 4→4 form did, by 320 EMC per craft).
2. Machine recipes now cost what the documented layout costs. Collector MK1
   costs 84,488 EMC in ingredients but is worth 16,384 in the table; that
   mismatch is flagged in §8 rather than fixed here, because machine balance is
   the next phase's job.

## 8. EMC consistency review

Invariants the tests enforce today:

- Each fuel tier is exactly 4× the previous one, and the 4:1 craft is neutral.
- A fuel block is exactly 9× its fuel.
- Both directions of every stone conversion are worth the same.
- Every ingredient and result of every recipe has a non-zero EMC value, so no
  recipe consumes a zero-value item and no result is unlearnable.
- Cost arithmetic stays in `emc::checked_cost`, which rejects multiplication
  overflow and balances above `i64::MAX`.

Known discrepancies, left for the balance phase with a reason for each:

| Item | Table value | Contradiction | Intent |
| --- | --- | --- | --- |
| Coal | 32 | Public default is 128 | Whole fuel chain is scaled to 1/4 of the public values; ratios are preserved. Changing it rescales every fuel, block and machine. |
| Sand / Gravel | 4 | Public default is 1 | Prototype value; harmless but inconsistent with cobblestone. |
| Netherite Scrap | 32,768 | Public default is 12,288 | Prototype value; does not affect progression because nothing in the alchemy chain uses it. |
| Philosopher's Stone | 16,384 | Its ingredients are worth 9,984 | Intentional in the original economy; the stone is a gate, not a resource sink. No item-to-EMC path exists yet, so it cannot be farmed for EMC. |
| Energy Collector MK1 | 16,384 | Recipe costs 84,488 | Machine values were set before the documented layouts were known. Must be re-based in the machine phase. |
| Collector/Relay MK2–MK3 | 4× the tier below | Recipes not yet confirmed | Provisional tier pricing, chosen to match the 4× machine progression. |
| Dark Matter | 16,384 | Recipe is a placeholder | Value will be re-derived from the confirmed Aeternalis-based recipe. |
| Klein Star Ein–Vier | 8,192 / 32,768 / 131,072 / 524,288 | Ingredients cost far more than the star's own value (Ein alone is 67,072) | Star values predate the recipe audit; the balance pass must re-derive them together with the upgrade recipes, which are themselves ⚠. |

The table also keeps **no free transmutation**: an item with `value == 0` is
refused by `/projecte transmute`, by learning, and by the table menu, so nothing
can be bought for nothing.

## 9. API dependencies and workarounds, by node

| Node | Needs | Available | Workaround |
| --- | --- | --- | --- |
| Stone world transmutation | name-based block write | `edit_blocks` | §6 |
| Stone crafting catalyst | kept ingredient | — | `item_crafted` refund, §5 |
| Fuel burn time | set burn time | listen-only hook | none, blocked |
| Table / Tablet UI | custom screen | chest menu only | none yet |
| Collectors / Relays | native block energy | not callable | NBT buffer, §1 |
| Klein Stars | native item energy | not callable | data component, §2 |
| EMC values from packs | data-map read | not exported | static table, §7 |
| Condensers | container slots | `block_entity_container` | none needed |

## 10. Known limitations of this phase

- Nothing below is claimed as tested at runtime: no `cargo test` and no
  `horizon-launcher native-build` was executed in the environment this phase was
  written in (the toolchain and the pinned Horizon source were absent). Only
  static checks and a Rust grammar parse were done. See the PR description.
- The stone's world transmutation covers one block per click. Charging and the
  cube/panel/line modes are not implemented.
- Crafting the stone's recipes with a completely full inventory can fail to
  return the stone; the player is told, and the loss is not silent
  (API_WORKAROUNDS §5).
- Machines keep EMC in block-entity NBT; Klein Stars keep charge in a component.
  Both are replaced wholesale when the native energy helpers become callable.
- No assets: every block and item borrows a vanilla template.

## 11. Next phases, in order

1. **Table deposit and unlearn** — the missing half of stage 0: burn an item to
   learn it and gain EMC, using the existing transaction planner for bounds.
   Small, self-contained, and it makes the current table a complete loop.
2. **Energy Condenser MK1** — the last machine needed for a Power Flower, plus
   the Alchemical Chest it depends on. EMC plumbing already exists in
   `machines.rs`; this unlocks the "make diamonds while away" experience.
3. **Collector fuel upgrading** and light-level rates — completes the machine
   tiers that already exist.
4. **Matter phase** — confirm the Dark/Red Matter recipes from public behavior,
   then add matter blocks, the Transmutation Tablet, and the first tools.
5. **Balance pass** — settle coal's value and re-base machine, matter and
   Klein Star values from their recipes (see §8).
6. **Data-map EMC** — only when Horizon exports a read helper; do not build a
   parallel resource parser.

Each of these can be implemented without redesigning this foundation: recipes
live in `recipes.rs`, the fuel and transmutation rules in `alchemical.rs`, the
roadmap in `progression.rs`, the economy in `emc.rs`, and `lib.rs` stays a hook
adapter.
