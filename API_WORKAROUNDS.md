# Horizon API workarounds

This file records deliberate compatibility workarounds. It is not a list of preferred APIs.
When the pinned Horizon API grows the missing trait-mod accessors, replace these implementations
in one migration and update this document.

Pinned Horizon revision: `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`.

## 1. Machine EMC: block-entity NBT instead of native energy access

**Current implementation**

- Collector and Relay store EMC in their own block entity's saved NBT compound, under `emc`.
- The Collector's current buffer is updated directly in `BlockEntityTickEvent.entity`.
- A one-time compatibility migration reads the legacy `projecte:buffer` attachment from PR #3 and moves it into the `emc` NBT field on the machine's first tick.
- Adjacent Relay block entities are modified through `Host::change_mod_block_entity`.
- Machine collection clears the NBT value, credits the player's ProjectE attachment through a fallible write, and attempts to restore the machine buffer if that write fails.
- The storage is server-authoritative and persists with the block entity.

**Why this is a workaround**

Horizon has the `block_entity_energy` capability hook (API 142), and its engine documents native energy operations. However, those operations are not exposed as callable methods on the pinned trait-mod `Host` surface / special Host bridge. The bridge does expose neighbouring mod block entities, which is enough for ProjectE-owned EMC transfer.

We therefore cannot safely use Horizon's energy handler as the source of truth from this trait mod. A ProjectE-owned NBT field keeps the gameplay functional without pretending that native energy access is available.

**Migration when Host access is added**

1. Preserve the one-time PR #3 attachment migration before replacing the NBT backend.
2. Register `ModEnergyBuffer` for each machine through `block_entity_energy`.
3. Replace reads/writes of `emc` with the official energy accessors.
4. Route collector-to-relay transfer through the official transfer helper.
5. Migrate old `emc` values to the native buffer exactly once; never initialize both stores independently.
6. Remove the NBT workaround and its migration code only after old worlds are covered.

## 2. Klein Star charge: ProjectE custom component instead of native item energy

**Current implementation**

- All six Klein Star tiers declare a ProjectE-owned `projecte:stored_emc` component.
- The component is read and updated through `Host::component`, `Host::set_component` and `Host::remove_component`; values are clamped to the tier's capacity.
- Collector slots 0 and 1 charge Klein Stars from its buffer. Relay slot 0 empties a star into its buffer; slot 1 charges a star. A full star moves to output slot 2.
- The tooltip displays stored EMC and capacity.

**Why this is a workaround**

Horizon has the `item_energy_buffer` capability hook (API 307), but the pinned trait-mod Host surface does not expose native item-energy read/insert/extract methods. The component Host bridge is available, so ProjectE stores charge in its own registered component.

**Migration when Host access is added**

1. Register `ItemEnergyBuffer` for each Klein Star tier.
2. Replace component reads/writes with native item-energy operations.
3. Preserve single-item stack semantics: Horizon's item-energy operation returns a modified one-item stack; split and replace stacks correctly.
4. Migrate existing `projecte:stored_emc` values into `horizon_energy` once.
5. Remove the custom component registration only after migration support is in place.

## 3. Neighbour transfer through block-entity mutation

`Host::mod_block_entity` returns a copy for reading. `Host::change_mod_block_entity` applies a mutation to the live block entity. ProjectE uses the latter for writes and only calls it from server-authoritative machine ticks or player interactions.

Do not replace this with a read-copy-write sequence: two machines could otherwise act on stale snapshots and duplicate or lose EMC.

## 4. Scope and known limitations

- Collector and Relay now have three tiers each. Energy Condenser MK1 is integrated as a terminal sink; MK2 remains unimplemented. This remains a partial machine port, not the complete ProjectE device lineup.
- Collector tiers generate 4/12/40 EMC per second in full sunlight, with capacities 10,000/30,000/60,000 and fractional EMC accumulation. Relay tiers have capacities 100,000/1,000,000/10,000,000, transfer limits 64/192/640 EMC per tick, and transfer bonuses of 0.05/0.15/0.5 EMC per successful transfer tick.
- Collectors feed adjacent Relays and Condensers. Relays feed adjacent Relays only when the target has a lower fill ratio, and feed adjacent Condensers as terminal sinks. Condensers never forward EMC. Machine slots are implemented; Collector fuel upgrades and Condenser MK2 remain outstanding.
- Klein Stars have persistent component storage and can be charged/discharged through Collector and Relay machine slots. Direct player-inventory transfer is not implemented yet.
- Attachment read/write errors in the existing player-data layer are a separate cleanup item; do not silently copy those patterns into the energy backend.
- Machine upgrade recipes (Collector MK2/MK3, Relay MK2/MK3) now exist in `engine/src/recipes.rs`, but a Collector still cannot upgrade fuel inside its own slots; that machine behavior is a later phase.

## 4.1 Condenser inventory and Power Flower routing

**Current implementation**

- Energy Condenser MK1 uses the existing `projecte:machine` block entity and a nine-slot generic menu: slot 1 is the player-set, non-consumed target template, slot 2 is output, and slots 0 and 3–8 accept EMC-valued input. The target slot is excluded from every automation face.
- The target must have positive EMC. An absent or invalid target pauses item processing; input stacks remain untouched.
- Input conversion and output production use the pure plans in `engine/src/condenser.rs`. The complete target `ItemStack` is cloned, preserving damage and custom components; output merging requires an exact item/damage/component match and respects `Host::max_stack_size`.
- Collectors may feed adjacent Condensers directly. Relays may feed adjacent Relays only when the destination has a strictly lower fill ratio, or feed adjacent Condensers. Condensers are terminal sinks and cannot forward EMC.
- Neighbour transfers use `Host::change_mod_block_entity` and debit the source only for the amount confirmed as written to the destination.
- The Alchemical Chest is a separate 27-slot block entity with a generic 9x3 menu. Its recipe is provisional pending confirmation against the intended ProjectE version.

**Known limitations**

- Only Condenser MK1 is implemented. Its 10,000 EMC capacity and one-item-per-tick input/output budget are explicit initial balance values. MK2 batching and its separate inventory layout are not implemented.
- The Alchemical Chest recipe is an explicitly marked placeholder and must be checked before release.

## 5. Philosopher's Stone as a crafting catalyst

**Current implementation**

- Every recipe that needs the Philosopher's Stone lists it as an ordinary ingredient in its data-pack JSON (`engine/src/recipes.rs`, `CATALYST_RECIPES`).
- The `item_crafted` hook gives one stone back for each craft whose recipe id is in that list, through `Host::give_stack`.
- If the inventory cannot take the stone, the player is told in chat that the stone could not be returned; the failure is never silent.

**Why this is a workaround**

Vanilla recipe types (`minecraft:crafting_shaped` / `crafting_shapeless`) consume every ingredient, and the pinned data-pack format has no "container item" or "kept ingredient" flag. ProjectE's stone is a catalyst that stays in the grid, which is behavior, not decoration: without the refund a player would lose a 16,384 EMC item on their first fuel craft.

The refund is a listener, not part of the craft, so it is applied after the result has been taken. It therefore depends on the inventory having room at that moment. Crafting frees the ingredient slots first, so the normal case always fits, but a full inventory is still reported to the player instead of being hidden.

**Known risk:** a data pack that redefines one of this mod's recipe ids without the stone as an ingredient would receive free stones. Recipe ids are namespaced `projecte:`, so this is a pack-author trust boundary, not a multiplayer exploit.

**Migration when Horizon supports it**

1. Replace the refund with a recipe type that declares a kept ingredient, if the pinned recipe API grows one.
2. Remove `CATALYST_RECIPES` and the `item_crafted` refund in one change; never keep both.
3. Re-run the recipe tests in `recipes.rs`, which prove each catalyst recipe holds the stone and each non-catalyst recipe does not.

## 6. World transmutation writes block states by name

**Current implementation**

- `use_item_on` resolves a partner block (`engine/src/alchemical.rs`, `world_swap`) and applies it with `Host::edit_blocks`, a single one-block run whose filter is the block that was clicked.
- The player's dimension comes from `NativeServerPlayer::dimension`; the edit helper only accepts the level being ticked, which is the server's authoritative level.
- Refusals (`Err`) and no-ops (`Ok(0)`) are reported to the player; they are never treated as success.

**Why this is a workaround**

`Host::set_block` needs a `BlockStateId`, and the pinned catalog exposes no helper that turns a block name into one (`native_default_state` exists inside Horizon but is not exported). `edit_blocks` takes block states as commands write them, so it is the only name-based write available to a trait mod. Its filter also gives the transform a compare-and-set guarantee that a `set_block` call would have to build by hand.

Trade-offs: the write bypasses neighbour updates and block-entity drops (documented behavior of `fill ... strict`), and it cannot carry NBT. ProjectE's stone only swaps plain blocks, so both limits are acceptable for now.

**Migration when Horizon exposes a state-id helper or a state-aware `set_block` path**

1. Resolve the target state once and use `set_block` with explicit flags.
2. Keep the compare-and-set check by re-reading the block before the write.
3. Delete the single-run `NativeBlockEdit` construction in `lib.rs`.

## 7. EMC values are not data-driven yet

- `data_map_types` can register `projecte:emc` for `minecraft:item`, and a data pack can ship `data/projecte/data_maps/item/emc.json`.
- The pinned `Host` exposes **no** data-map read helper: `native_data_map` and `native_data_map_entries` exist inside `horizon-client` but are not in `native-hooks.json`, so a trait mod cannot call them.
- Until that changes, EMC resolution stays in the static table in `engine/src/emc.rs` plus the vanilla-only family rules. Registering the map type or shipping the JSON alone would not make the resolver data-driven, so this crate does neither yet.
- Do not hand-roll a resource-file parser to imitate data maps: pack precedence, per-entry `replace`, tag expansion and `remove` would have to be re-implemented and would not match the server's merged map.

**Migration when a read helper exists**

1. Register `projecte:emc` through `data_map_types` and ship the default values as a data-pack file.
2. Replace `emc::table_value` lookups with a read of the merged map, keeping the vanilla family rules as a documented fallback for entries the map does not cover.
3. Keep the static table as the shipped default data, not as a second runtime source of truth.
