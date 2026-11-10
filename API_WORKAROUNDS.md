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

## 8. The Transmutation Table screen: a vanilla menu screen, not a container screen

**The problem.** The screen needs two things that the pinned API does not give at once: a screen this mod draws itself, and **real** inventory slots — items picked up, placed, split, swapped, shift-clicked, and returned to the player when the screen closes, all server-authoritative.

**What the pinned revision offers.**

- `native_mod_screen_open` (API 111) opens a client-only window with **no menu**. Nothing in it is a slot; a click there cannot move an item, and the server never hears of it.
- `native_screen` (API 19) lays out a mod block's **menu**: it says where each of the container's `Slot`s is and where the player's inventory begins. The menu itself is the server's, driven by vanilla's menu engine, so a click on one of its slots is a real click that the server applies.

There is no third route and no way to combine them: a mod screen has no slots, and a menu screen is the only place real slots exist.

**What this mod does.** The table's block entity declares a `ModContainer` with `menu: Some("minecraft:generic_9x1")` and nine slots, and the screen is drawn with `native_screen`/`native_screen_draw`. `minecraft:generic_9x6` would give 54 slots, but 54 real slots plus a 9 x 4 browser plus the player's inventory do not fit in one 256-wide screen at 1:1, so the smaller vanilla menu is used and every one of its slots is shown.

**The trade-offs, stated plainly.**

1. **The browser is not a set of slots.** The engine draws a menu slot's own stack over whatever a screen paints, so a catalogue cell that were also a slot would show an item the player never put there, and the player could pick a catalogue item up out of thin air. The browser's cells are drawn by this mod alone and a click on one is a placeholder notice. This is the honest shape of the API, not a shortcut: the alternative would be fake clickable slots, which this project does not ship.
2. **The menu type is vanilla's.** A player on a client without Horizon sees a plain nine-slot chest screen: the slots work, the custom drawing does not. The alternative (a mod menu type) disconnects such clients.
3. **The menu's title is the server's.** It comes from `block.projecte.transmutation_table`; the mod draws its own localized title over the panel with `own_labels`, so the visible title is the screen's own text.

**Migration when Horizon exposes slots for a mod screen.** Have `native_mod_screen_open` take a `NativeScreen` with slots (the same `Vec<Option<[f32; 2]>>` `native_screen` already uses), have the server own a `ModContainer` for that screen, and let a cancelled or uncancelled click reach the menu. Then the browser's cells could become real slots directly, and the vanilla menu could go.

## 9. Screen localization: a table in code, because a native mod has no `assets/`

**The problem.** The screen's words must be localized (English at minimum, structured for Russian) and keyed, with no scattered hard-coded strings — the usual answer being `assets/<mod id>/lang/en_us.json`.

**Why that file cannot be shipped here.** In the pinned revision only a **content mod** has assets: "only `horizon-mod.toml` with `assets/` and `data/` is a content mod" (`docs/native-mods.md`). This project is a **native trait mod** — `horizon-mixin.toml`, `engine/` and Rust code compiled into the game — so it has no `assets/` folder for the game to read a `lang/en_us.json` from, and `native_data_pack_files` (API 106) accepts only `recipe/`, `tags/`, `loot_table/`, `advancement/`, `trade_set/`, `villager_trade/` and `dialog/` paths, never `lang/`.

**What this mod does.** `engine/src/ui/localization.rs` holds the tables: `EN_US` and `RU_RU`, keyed by the same `container.projecte.*` keys a resource pack would use, with a test that walks every key and proves both languages cover them. The keys are constants, so a typo is a compile error rather than a missing translation.

**Reading the player's language.** There is no client-locale helper in the pinned API. The screen asks the game to translate `container.inventory` — a phrase every resource pack that supports a language has, and which the engine's own screen drawing reads immediately above the inventory — and matches the result against the languages it ships (`"Инвентарь"` for `ru_ru`, everything else English). This follows the player's language for any locale, and a locale with no table of this mod's falls back to English rather than showing nothing.

**A notice is drawn, not sent.** Every screen hook is `&self` at this revision, so `Host::action_bar` and `Host::chat`, which take `&mut dyn Host`, cannot be called from a screen. A control's notice is therefore drawn on the screen's own bottom line, where the click happened.

**Migration when Horizon exposes a mod's own assets or a locale helper.** Ship `assets/projecte/lang/en_us.json` and `ru_ru.json` from the same tables, and replace the `container.inventory` probe with the locale helper. Nothing else in the screen changes: the keys already match.

## 10. Deposit-and-learn: a mod-owned menu, and why the credit is read back

**The problem.** Depositing is the flow that turns items into EMC and knowledge. It must remove *exactly* the accepted items, credit their value, add them to the learned set, and never mint EMC for an item that did not actually leave the player's hands. In ProjectE it happens on the Transmutation Table, so the obvious first idea is a click on that table's existing screen.

**Why a click on the table's screen cannot do it.** The table's menu is a vanilla menu the engine owns, drawn over by `native_screen` (§8). Its slots hold the player's own items and a shift-click or drag moves them by the engine's rules. Two bad options follow: mutate items inside `menu_clicked` without knowing what the engine will actually do, or make the slots inert so the click can be observed. The first can lose items when the engine's own move differs from the mod's assumption; the second makes every slot decoration and the table unusable. Both were rejected.

**What the pinned API provides.** `native_open_menu_with_input(uuid, rows, title, items, input)` (API `menu_input`) opens a menu whose `input` slots behave **exactly** as a chest's: the player fills them by vanilla's click rules, the engine tracks them, and `native_menu_contents` reads them back. Slots outside `input` are the mod's display slots, and a click on one arrives as `native_menu_clicked` precisely because it "moved nothing". On close, `native_menu_input_closed` hands the mod the input slots and whatever the mod leaves goes back to the player as `clearContainer` would.

**The chosen shape.** A 27-slot menu: slots 0–8 are the engine-owned input range, 9–17 display cells, and 18 is the confirm control. A deposit is *only* a click on slot 18. `apply` then:

1. reads the menu and plans the deposit (**pure**, `deposit::plan`);
2. refuses the whole thing, before touching a slot, if any stack has no value or the arithmetic would overflow;
3. clears exactly the planned slots and writes them back;
4. **re-reads** `native_menu_contents` and credits only what verifiably left (`deposit::settle`);
5. writes EMC and knowledge in a **single** `data::set`, so the two can never disagree.

Step 4 is the part that makes the flow safe. `native_set_menu_items` deliberately preserves input slots (it copies back every slot the store holds), so a mod write **cannot** eat a deposit — but a write can also fail to take. Crediting only the observed departure means a write that did nothing credits nothing, and `Deposited::NotApplied` tells the player their items are still in the slots.

**Recognising the mod's own menu.** Every native menu in the engine is the same `Slots::Native`, and the API does not say which mod opened one. Slot 18 of *another* mod's menu is not a confirm control, and treating it as one would empty that menu into EMC. The mod therefore records the window id it opened, per player, in `deposit_menu::OPEN`, and `forget`s it in both `menu_closed` and `menu_input_closed`. A click is a deposit only when the player, the window and the slot all match.

**What is not done, and why.** Shift-clicking into the input slots works because the engine owns them; there is no custom drag handling, no per-slot filter, and no "deposit all from inventory" button — each of those would have to move items itself. The menu shows the balance and the known-item count as stack counts on icons, which caps their display at 64; `/projecte emc` remains the exact readout.

**Migration if Horizon exposes per-mod menu ownership.** Drop `OPEN` and read the owner from the menu, which removes the one piece of bookkeeping here. Nothing else changes: the plan/settle split is independent of how the window is recognised.
