# Horizon API notes for ProjectE-RS

> **Pinned source:** Horizon `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`  
> **Native Hooks API:** `411`  
> **Minecraft target:** `26.3`  
> **Status:** implementation notes for this exact Horizon revision; native-mod APIs are experimental and must be rechecked when changing the pin.

This document records the Horizon contracts ProjectE-RS can rely on. It is intentionally tied to the revision in `horizon-mixin.toml`: do not silently replace signatures or behavior with details from Horizon `main`.

## 1. API layers and version numbers

Horizon has two relevant mod surfaces:

- **Content** (`.hmod`): declarative assets and data such as models, textures, blockstates, language files, recipes and tags. No Rust code is run by the content layer.
- **Native** (`.hnmod`, or native content combined in a future-compatible `.hmod`): Rust code compiled into a game build by the launcher. Native code runs with the player's OS permissions; it is not sandboxed.

ProjectE-RS currently uses a **native trait mod**: `engine/src/lib.rs` implements `horizon_client::mod_api::Hooks`, and the root `horizon-mixin.toml` points to the crate through `[api] path = "engine"`.

Do not conflate these version values:

| Value | Meaning |
| --- | --- |
| `NATIVE_HOOKS_API = 411` | Global version of the typed native hook/helper contract at the pinned revision. |
| `version = 1` on an entry in `native-hooks.json` | Version of that individual hook definition, not the global API level. |
| `horizon_mods::api_level::LEVEL = 7` | A separate content/WASM-era API-level constant; it is not the native hooks version. |
| `horizon-mixin.toml`'s `horizon_revision` | Exact Horizon source revision used to test/build this bundle. |

The pinned catalog contains **432 hook definitions and 174 Host/helper definitions**. It is the source of truth for names and contracts.

For a registry native bundle that uses only catalog hooks and `Host` helpers, the native manifest can declare `"hooks": 411`; this allows a Horizon with an equal or later `NATIVE_HOOKS_API` to satisfy the API requirement. Keep the exact `horizon` commit as the tested revision. Raw mixin injections into ordinary game functions are not covered by this compatibility promise and must pin the exact Horizon revision.

## 2. Typed Hooks execution model

The trait is `horizon_client::mod_api::Hooks`. A mod exposes a factory returning `Box<dyn Hooks>` (ProjectE-RS uses `pub fn hooks() -> Box<dyn Hooks>`).

Important dispatch rules:

1. A single mod instance is installed process-wide and serves both the window's game and the server-thread game.
2. Hook methods take `&self`; implementations must be `Send + Sync`. Use interior mutability only when persistent mutable state is genuinely required.
3. Hook event structs are generated and marked `#[non_exhaustive]`. Public fields represent the hook's arguments and mutable outputs. Do not construct or exhaustively match these event structs.
4. A hook receives a per-call `Host` where the contract provides one. It is a contextual capability interface, **not a global game singleton**. Use the host supplied to the current callback.
5. Hooks can be re-entered by nested game operations. Do not hold a lock across a Host call that may trigger another hook.
6. Mixin injections run before trait hooks. An earlier injection can return early or claim cancellation and prevent trait dispatch.
7. For cancellable hooks, `Flow::Continue` lets dispatch continue; `Flow::Cancel` claims the action and stops later mods/game handling. The first `Cancel` wins.
8. For replacement hooks, the first `Some` wins. Collection hooks call every mod and append/modify the shared collection.

The generated `Host` methods default to a panic in test doubles. This is useful: a test fails loudly if a hook unexpectedly calls a helper the double did not implement. Test through the generated `dispatch_<hook>` function where practical, rather than relying only on isolated helper tests.

### ProjectE hooks currently implemented

The current `ProjectE` implementation uses these catalog hooks:

- `blocks`, `items`
- `block_entity`, `block_entity_ticks`, `block_entity_container`, `block_entity_tick`, `component_types`
- `right_click_block`, `use_item`
- `menu_clicked`
- `register_commands`, `perform_command`
- `tooltip`, `data_pack_files`

These are typed catalog hooks; the implementation does not need private engine injections.

## 3. Registration and content

### Blocks and items

- `blocks` and `items` are collection hooks. Append registrations to the event's collection; do not replace registrations from other mods.
- A mod block/item uses a namespaced id such as `projecte:energy_collector`.
- A block declaration can select a vanilla template for its initial behavior/appearance. That is a bootstrap/template choice, not a substitute for the final ProjectE asset set.
- `component_types` is the registration hook for custom data-component types. A content item can then carry a registered component through its content JSON.

Models, textures, blockstates and language resources belong to the content layer, not to `data_pack_files`. Keep the native code and content assets conceptually separate even if a later package combines them.

## 4. Block entities, machine storage and energy

### Block entity lifecycle

Relevant hooks:

- `block_entity`: supplies the mod block entity identity/initial NBT for a mod block.
- `block_entity_ticks`: opts a mod block into server ticking.
- `block_entity_tick`: called for a ticking mod block entity and receives its `ModBlockEntity`.
- `block_entity_update_tag`: defines the client update tag sent when the chunk is sent or the block changes.
- `block_entity_container`: declares item slots for a mod block entity.
- `block_entity_fluids`: declares native fluid tanks.

A block entity tick receives the entity object as an argument. Work with that object rather than assuming the same entity can be reacquired through ordinary world/container helpers while it is being ticked. Treat the callback as server-authoritative; client rendering should consume the state Horizon synchronizes.

### Native block energy

The pinned API includes `ModEnergyBuffer` and the `block_entity_energy` hook (Native Hooks API 142).

The hook declares a buffer for a mod block: capacity, maximum insertion and extraction per operation, and permitted faces. Horizon stores the amount in the block entity's saved compound under the integer key `energy`; it therefore survives chunk unloads and restarts.

The engine documents `native_energy`, `native_energy_insert`, `native_energy_extract` and `native_move_energy`. However, these are not present as callable methods on the pinned trait-mod `Host` surface / special Host bridge. The `block_entity_energy` registration hook exists, but a trait mod cannot use those engine operations through `dyn Host` at this revision.

**ProjectE implication:** the current implementation stores EMC in a ProjectE-owned `emc` field in the block entity's saved NBT and mutates adjacent mod block entities through `Host::change_mod_block_entity`. This is a documented workaround, not the preferred long-term backend. See [API_WORKAROUNDS.md](API_WORKAROUNDS.md).

### Native item energy

The pinned API includes `ItemEnergyBuffer` and `item_energy_buffer` (Native Hooks API 307).

A mod declares an item buffer's capacity and maximum one-operation insert/extract amounts. Horizon stores its charge in the stack's `minecraft:custom_data` as integer `horizon_energy`, which is saved and synchronized with the item stack.

The engine documents `native_item_energy`, `native_item_energy_insert` and `native_item_energy_extract`, but these methods are not callable through the pinned trait-mod `Host` surface. The `item_energy_buffer` registration hook exists, but the trait mod cannot currently inspect or update its native item-energy state through Host.

**ProjectE implication:** Klein Stars use a registered ProjectE component, `projecte:stored_emc`, read and updated through the component Host bridge. Collector and Relay machine slots charge and discharge that component. This is a deliberate workaround. See [API_WORKAROUNDS.md](API_WORKAROUNDS.md).

## 5. Attachments and persistent state

Attachments are appropriate for mod-owned state that belongs to a player, block entity or level and is not already represented by a native capability.

The core API is in `horizon_world::attachments` and is re-exported to native mods:

- `AttachmentType<T>` defines `namespace`, `name`, a default factory, an optional codec, `copy_on_death` and `sync`.
- The stable key is the pair `(namespace, name)`. Two statics with the same pair address the same stored value.
- `get`, `get_existing`, `set` and `remove` return `Result`; propagate or deliberately handle `AttachmentError`. Do not silently turn storage failures into a default value or ignore failed writes.
- A codec is required for saved/copy-on-death state and for synchronized state.
- `copy_on_death = true` copies the attachment to the respawned player.
- `sync = true` sends attachment changes to clients. With `sync = false`, clients do not receive that attachment through the sync channel.
- `sync_data` queues the current value for synchronization; it has no effect for an unsynchronized type.

For ProjectE, player EMC and learned knowledge can remain server-authoritative player attachments. Keep them unsynchronized unless a client-side UI actually needs a replicated view; if so, define the synchronization contract intentionally. Avoid using an attachment for a capability Horizon already owns, such as machine or item energy.

## 6. Data Maps and data-pack files

### Data Maps (Native Hooks API 143)

Data Maps are the preferred data-driven direction for item EMC values.

A native mod registers a `NativeDataMapType` through the `data_map_types` collection hook. The type identifies:

- a namespaced map id such as `projecte:emc` (not in the `minecraft` namespace);
- the registry it applies to, currently `minecraft:item` or `minecraft:block`.

The registration hook is called when the server's data is loaded: on the world's first open and on every `/reload`.

Data comes from either:

1. World/data-pack files at `data/<pack namespace>/data_maps/<registry path>/<map name>.json`; or
2. Files added by the native mod's `data_pack_files` hook, under the key `data_maps/<mod id>:<registry path>/<map name>`.

For an item map `projecte:emc`, the conceptual file location is:

```text
data/projecte/data_maps/item/emc.json
```

The pinned file schema is:

```json
{
  "replace": false,
  "values": {
    "minecraft:coal": 32,
    "#minecraft:logs": 32,
    "minecraft:diamond": 8192
  },
  "remove": ["minecraft:example_item"]
}
```

Rules:

- `replace: true` clears values accumulated from earlier files before this file is applied.
- `values` maps registry entries or tags (keys prefixed with `#`) to JSON values.
- An entry can use `{"value": ..., "replace": false}` to control replacement for that entry.
- A tag assigns the value to every entry in that tag.
- `remove` is applied after the current file's values.
- Later values override earlier values.
- The server can read an entry through `native_data_map(registry, type, entry)` and enumerate values through `native_data_map_entries(registry, type)`.
- These maps are server-side and are not synchronized to clients in this revision. Registry support, mergers and conditions are limited; do not assume the full data-map feature set of another loader.

This is the correct extension point for modpack-defined EMC overrides. Do not use suffix heuristics as a substitute for explicit values: an unknown mod item ending in `_ingot` or `_ore` is not necessarily worth the vanilla heuristic amount.

### `data_pack_files`

The native hook contributes data-pack files at load/reload time. It is for data consumed by the server's data-pack machinery, not textures, models, blockstates or localization assets. Use it for small built-in defaults when a separate world data pack should be able to override them.

World data packs have the higher priority over native mod-provided files in the loading order described by the pinned API.

## 7. Menus and screens

- `open_menu` opens a vanilla menu for a player. A chest-style menu supports up to six rows; the current ProjectE table's 54 slots fit the six-row limit.
- `menu_clicked` receives the player UUID, menu/window id, slot, button and click mode. It is a notification hook: the mod is responsible for the transaction semantics it implements.
- `menu_closed` reports that a mod-opened menu closed.
- The menu's server-side slot movement is not automatically a custom transmutation transaction. Validate slot indices, requested amount, inventory room and EMC before mutating player state or granting items.
- Custom menu types are available through the content layer; native menu slot rules can further define which slots accept/take items.
- `screen`, `screen_draw`, `screen_click` and `screen_tooltip` support native custom screens. A custom menu screen is a client-side rendering/input implementation and must be designed alongside the server-authoritative menu logic.

A chest menu is a reasonable first implementation. A full Transmutation Table UI (search, categories, paging and a learned-item browser) should move to a custom screen rather than overloading the semantics of a generic chest.

## 8. Recipes, commands, networking and rendering

### Recipes

The pinned Host surface includes helpers for querying recipe data, including `recipes_of_type`, `recipe_ingredient`, `recipe_result` and `known_recipes`; the hook catalog also includes recipe-related hooks such as `recipes_received`, `ingredient` and `ingredient_test`.

Use the actual recipe types and recipe results returned by the world. Do not assume that the presence of a recipe alone implies an EMC value: EMC assignment, recipe learning and recipe-derived value calculation are separate policies. A future EMC resolver should be deterministic, cycle-safe and explicit about ingredients with no value.

### Commands

`register_commands` is a collection hook for server commands; `perform_command` is the command execution hook. Keep command parsing and validation on the server and route player-facing feedback through the supplied Host. Never trust client-supplied item ids, counts or balances.

### Networking

The API includes client/server native message helpers such as `net_send_to_player` and `net_send_to_server`, alongside client and server hooks. Treat the server as authoritative for EMC, learned knowledge, machine state and transmutation. A client message is a request, not proof that the requested operation is valid.

### Rendering

The catalog includes screen and block-entity drawing hooks/helpers (for example `screen_draw`, `draw_item`, `draw_block`, `draw_quad` and `draw_cube`). Render code belongs on the client/render side; it should read synchronized state or a client snapshot, not mutate authoritative EMC or machine energy.

## 9. Packaging and build workflow

The repository's current root is a local native bundle:

```text
ProjectE-RS/
├── horizon-mixin.toml
└── engine/
    ├── Cargo.toml
    └── src/
```

The `[api] path` must be a relative path to a crate inside the bundle and that directory must contain `Cargo.toml`. The pinned launcher stages the API crate into its generated workspace and rewrites the `horizon-client` dependency to the staged Horizon source before building. This is why the local absolute Cargo path is not itself the bundle's API path; it still makes standalone Cargo invocations less portable.

For a distributable native-only bundle, Horizon's file format expects:

```text
<bundle>/
├── horizon-native.json
└── mixins/projecte/
    ├── horizon-mixin.toml
    └── engine/
        ├── Cargo.toml
        └── src/
```

The native manifest requires `schema`, `id`, `version`, `game`, `horizon` and `mixins`; `hooks` is optional. Its id/version and Horizon commit must match the nested `horizon-mixin.toml`. A combined content/native `.hmod` can carry `horizon-mod.toml`, `assets/`, `data/`, `horizon-native.json` and `mixins/<id>/` together, with both manifests naming the same mod id. The registry's native distribution in this revision still uses `.hnmod`.

For the current checkout, build with the Horizon-enabled launcher and the pinned source:

```bash
horizon-launcher native-build .
```

Run Rust unit tests in the same Horizon-enabled toolchain environment. A successful Python model or static inspection is not a substitute for `cargo test` or a successful native build.

## 10. ProjectE-RS architectural consequences

| Area | Current implementation | Horizon-aligned direction |
| --- | --- | --- |
| Player EMC / knowledge | Player attachment | Keep server-authoritative; handle attachment errors explicitly. |
| Collector / Relay energy | ProjectE-owned block-entity NBT field `emc` | Migrate to `ModEnergyBuffer` when trait-mod Host access is exposed. |
| Klein Stars | Registered `projecte:stored_emc` component | Migrate to `ItemEnergyBuffer` when trait-mod Host access is exposed. |
| EMC values | Hardcoded table plus suffix fallback | Add `projecte:emc` Data Map and explicit defaults; remove unsafe assumptions for unknown modded items. |
| Transmutation UI | 54-slot chest menu | Keep as initial UI; custom screen for advanced browser/search. |
| Recipes | Small fixed recipe set | Use real world recipe APIs and/or data-driven recipe definitions. |
| Models/textures | Not yet integrated in this native crate | Ship as content assets, ideally in the combined content/native package when supported by the target launcher. |

### Known limitations and cautions

- Native mods are experimental and execute with the player's permissions.
- Data Maps in this pinned API are server-side and not synced to clients.
- The current machine buffer attachment duplicates a native capability; migrate rather than layering both systems.
- Attachment helpers must not hide `AttachmentError` with `unwrap_or_default()` or `let _ = ...` in production paths.
- Do not assume that a block entity's world lookup is valid during its tick callback; use the passed `ModBlockEntity`.
- Do not keep locks across Host calls that may re-enter hooks.
- Do not infer EMC for arbitrary modded items from naming suffixes.

## 11. Pinned source references

All links below point to the exact Horizon revision recorded in this document.

- [Native hook catalog](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/native-hooks.json) — hook/helper names and contracts.
- [Native mod guide](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/docs/native-mods.md) — typed Hooks semantics, data maps, energy, menus, packaging and compatibility.
- [Typed mod API](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/horizon-client/src/mod_api.rs) — public API exports and `Flow`.
- [Hook dispatch implementation](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/horizon-client/src/game/native_hooks.rs) — actual hook dispatch and event paths.
- [Attachment implementation](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/crates/horizon-world/src/attachments.rs) — attachment codecs, errors, persistence and synchronization.
- [Native bundle manifest format](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/docs/api/mod-files.md) — `.hnmod` / combined `.hmod` structure and manifest schema.
- [Launcher trait-mod staging](https://github.com/horizon-rust/horizon/blob/fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca/launcher/src/native_mods/api.rs) — how the `[api]` crate is validated, copied and built.
