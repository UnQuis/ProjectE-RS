# The Transmutation Table screen: every Horizon symbol it uses

Pinned revision: `fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`.

This file lists every Horizon symbol the screen touches, the API number it first
appeared in, and what this mod does with it. Nothing outside this list is used.
The symbol names, parameter names and parameter types were read from
`crates/horizon-client/src/game/native_hooks.rs`, `crates/horizon-client/src/mod_api.rs`
and `native-hooks.json` at that revision, not inferred from documentation.

The screen's Horizon calls all live in `engine/src/ui/atlas.rs`; the hooks that
receive the events are in `engine/src/ui/screen.rs` and registered in
`engine/src/lib.rs`.

## Hooks (events the engine sends)

All nine are client-side and all nine borrow `Game` immutably at this revision,
so their `Host` is `&dyn Host`.

| Hook | API | Contract | Event fields used | What the screen does |
|---|---|---|---|---|
| `native_screen` | 19 | `replace` (`screen: &mut Option<NativeScreen>`) | `block` | Answers a `NativeScreen` for `projecte:transmutation_table` — the menu's size, its nine placed slots, `inventory_x`/`inventory_y`, `own_labels`, `own_background`. |
| `native_screen_draw` | 19 | `listen` | `block`, `data`, `batch`, `origin` | Draws the panel, emblem, EMC display, slot frames, browser, controls, labels and the notice line. |
| `native_screen_tooltip` | 20 | `collect` (`lines: &mut Vec<String>`) | `block`, `data`, `at` | Adds one line for the part under the pointer, if any. |
| `native_screen_opening` | 367 | `listen` | `screen` | Starts or re-places the screen state when the table's menu opens. |
| `native_screen_closing` | 365 | `listen` | `screen` | Forgets the state, so it cannot leak into the next screen. |
| `native_screen_mouse_button_pressed_pre` | 80 | `cancel` (`-> Flow`) | `screen`, `button`, `modifiers` | Presses the control under the pointer; cancels the click **only** for the screen's own controls. |
| `native_screen_mouse_button_released_pre` | 80 | `cancel` (`-> Flow`) | `screen`, `button`, `modifiers` | Releases a held button. Never cancels. |
| `native_screen_char_typed_pre` | 104 | `cancel` (`-> Flow`) | `screen`, `c`, `modifiers` | Types into the search field while it has focus; cancels the character then. |
| `native_screen_key_pressed_pre` | 104 | `cancel` (`-> Flow`) | `screen`, `key`, `repeat`, `modifiers` | Backspace / Enter / Escape for the search field; cancels only those. |

The block entity and menu side, also in `engine/src/lib.rs`:

| Hook | API | Contract | What the mod does |
|---|---|---|---|
| `native_block_entity` | 15 | `replace` (`entity`) | Gives the table its own block-entity id (`projecte:transmutation_table`), so its saved data is its own. |
| `native_block_entity_container` | 16 | `replace` (`container`) | Declares the nine-slot `ModContainer` with `menu: Some("minecraft:generic_9x1")`. |
| `native_right_click_block` | 21 | `cancel` (`-> Flow`) | Lets the engine open the table's menu by returning `Flow::Continue`. |

`native_hidden_slots` (server-side, defined in `native_hooks.rs` and called by
the engine, not by a mod, so it has no API number of its own) is what
honours `NativeScreen.slots`: a slot left out of the list is not in the
server's menu either, so a shift-click cannot reach it. This screen shows every
slot of its menu, so nothing is hidden.

## Helpers on `Host`

Every one takes `&dyn Host` here; none needs `&mut dyn Host`.

The API column gives the number when the pinned source annotates the helper
with one. Where it does not, the helper belongs to the baseline the screen
hooks were introduced with (API 19/20) and the table says so rather than
guessing a number.

| Helper | API | Signature as used | What it draws / answers |
|---|---|---|---|
| `native_fill` | ≤19 (baseline) | `fill(batch, rect: [f32; 4], argb: u32)` | Every panel, plate, frame, bar and fill. |
| `native_text` | ≤19 (baseline) | `text(batch, text: &str, at: [f32; 2], rgb: u32, shadow: bool)` | Every label, button caption, the title and the notice line. |
| `native_text_width` | ≤19 (baseline) | `text_width(text: &str) -> f32` | Centring text and cutting the search field's overflow. |
| `native_item_stack` | 313 | `item_stack(batch, stack: &ItemStack, at: [f32; 2], size: f32) -> bool` | The browser's item icons, with the engine's own decorations. |
| `native_screen_pos` | ≤20 (baseline) | `screen_pos() -> Option<[i32; 3]>` | Which block the open menu belongs to. |
| `native_block` | 20 | `block(pos: [i32; 3]) -> String` | Confirms the menu's block is the table (`projecte:transmutation_table`). |
| `native_client_state` | see note | `client_state() -> Option<NativeClientState>` | The window's GUI size (`gui_size`), for centring. |

`native_client_state` is defined in `game/native_client_services.rs` and
re-exported from `mod_api.rs`; it is not in `native-hooks.json`'s helper list,
so no API number is attributed to it and none is claimed here. `NativeClientState`
is a `#[non_exhaustive]`-free struct whose `gui_size` field is the one used.

## Types

| Type | Where | Fields used |
|---|---|---|
| `NativeScreen` | `mod_api.rs:395` | `width`, `height`, `slots: Vec<Option<[f32; 2]>>`, `inventory_y`, `inventory_x`, `own_labels`, `own_background` |
| `NativeScreenView<'a>` | `mod_api.rs:297` | `id`, `menu`, `slots`, `mouse` |
| `HudBatch` | re-exported in `mod_api.rs` | passed straight through |
| `ItemStack` | `horizon-items`, re-exported | built with `ItemStack::new(id, 1)` for a browser icon |
| `Text` | re-exported | `translate_with_fallback`, `translation_key`, `plain` |
| `ModContainer` | `horizon-client-render/.../block_entity.rs:249` | `slots`, `faces`, `inputs`, `outputs`, `menu` |

## Engine behaviour this screen depends on

These are not API calls, but the screen's correctness rests on them, and each
was read from the pinned source:

- **Placement.** The panel is centred at
  `HudBatch::centered_origin(width, height)` — `((gui - size) / 2).trunc()` — and
  drawn at **1:1** in GUI pixels (`game.rs`, `inventory_origin`). `Layout::fit`
  reproduces exactly that and never rescales; scaling the drawing would part it
  from the engine's own slot positions.
- **Slot coordinates.** `Game::slot_position` places player slots from
  `native_screen_inventory_x`/`_y` (hotbar at `y + 58`), and container slot
  `300 + n` from `NativeScreen.slots[n]`. `Layout::slot` and
  `Layout::inventory_slot` use the same numbers.
- **`own_background` skips all slot frames**, including the player's
  (`draw_native_screen`). This screen therefore draws the player's 36 frames
  itself; the engine still draws every slot's item over them.
- **A cancelled press goes no further** — "no slot, no button, no drag"
  (`game.rs`, the `MouseInput` arm). So only a click on the screen's own controls
  is cancelled; a click on a real slot is left to the engine.
- **`view.slots` is `Game::slot_count`**: the player's 36 plus the container
  slots the screen lays out, i.e. 36 + 9 = 45 here. `atlas::is_table` compares
  against exactly that, not against the menu's 9.
- **`MOD_MENUS`** pairs `minecraft:generic_9x1` with 9 slots, which is why that
  menu type was chosen.

## What is verified and what is not

- **Not compiled and not run.** No Rust toolchain and no launcher are available
  in this environment, so no `cargo build`, `cargo test` or in-game screenshot
  exists for this work. Every claim above is from reading the pinned source and
  from the static checks listed in the pull request.
- **Static checks that were run:** a tree-sitter parse of all 15
  `engine/src/**/*.rs` files (0 syntax errors); a script that confirms all 27
  `Host` methods called exist in the pinned helper catalog and all 25
  implemented hooks exist in `native-hooks.json`; a script that compares every
  implemented hook's parameter names and types against the real
  `native_*` signature (including the `replace`/`cancel`/`collect` out-argument
  rule and `&self` vs `&mut self`); and a mirror of the layout constants that
  proves no two parts overlap and nothing leaves the panel.
