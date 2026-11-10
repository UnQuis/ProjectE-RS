//! The Transmutation Table screen's one place that calls Horizon.
//!
//! Every symbol here is Horizon's own surface at the pinned revision
//! (`fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca`); the screen's shape, its words
//! and what its controls do live in `super::layout`, `super::localization` and
//! `super::state`, which compile and test without a game. Keeping the API calls
//! in one small file is what makes the screen reviewable: every Horizon
//! dependency is listed here and nowhere else, and `UI_API.md` names each one
//! with the API number it first appeared in.
//!
//! # Why the screen is a mod *menu* screen
//!
//! The pinned API offers two ways to show a custom screen. `native_mod_screen_open`
//! (API 111) opens a *client-only* window with no menu, so it has no slots at
//! all; `native_screen` (API 19) lays out a mod block's **menu**, which the
//! server owns and vanilla's menu engine drives, so a click there is a real
//! click on a real slot. The task needs real slots — items picked up, moved,
//! shift-clicked and returned — so this screen is the second kind: the
//! Transmutation Table's block entity declares a nine-slot container with a
//! vanilla `minecraft:generic_9x1` menu, and the mod draws that menu's screen.
//!
//! # What is a slot and what is not
//!
//! The screen's browser (the 9 x 4 grid of items to choose from) is **not** a
//! set of menu slots. It cannot be: the engine draws a menu slot's own stack
//! from the server over whatever the screen paints, so a catalogue icon and a
//! menu slot in the same rectangle would show an item the player never put
//! there, and the player could pick a catalogue item up out of thin air. The
//! browser's cells are therefore drawn by this screen alone — icons of the
//! items it browses — and a click on one is a placeholder
//! (`localization::KEY_NO_TRANSMUTATION`), never an item movement.
//!
//! The **real** slots are the nine of the menu: two inputs and seven storage
//! slots, all placed by `native_screen` at `Layout::slot`'s coordinates and
//! drawn as frames by this screen. Their items are the engine's: pick up,
//! place, split, swap, shift-click and the return of whatever is left when the
//! screen closes are vanilla's own menu handling on the server, and this file
//! never touches a stack in them.
//!
//! # The title
//!
//! The menu's title is the one the server sends, from
//! `block.projecte.transmutation_table`; the screen draws its own title over
//! the panel (`own_labels`), so the visible title is this mod's text in the
//! player's language. `mod.rs::TITLE_KEY` is the single key both use.

use horizon_client::items::ItemStack;
use horizon_client::mod_api::{Host, HudBatch, NativeScreen, NativeScreenView};

use super::layout::{self, Control, Layout, Part, Rect};
// The menu's shape is geometry, so it lives in `super::layout` with the rest
// of it: how many slots there are, and which of them are the inputs.
pub(crate) use super::layout::{INPUT_SLOTS, MENU_SLOTS, SLOT_COUNT};
use super::localization;
use super::state::TableScreen;

/// This mod's id, for the helpers that name it (settings).
pub(crate) const MOD_ID: &str = "projecte";

/// The menu type the table opens. Vanilla's, so a stock client shows the same
/// menu (its slots in vanilla's own order) instead of being disconnected for a
/// menu type it does not have. `MOD_MENUS` in the pinned engine pairs this
/// name with nine slots.
pub(crate) const MENU: &str = "minecraft:generic_9x1";

// --- colours ---------------------------------------------------------------
//
// ARGB, as `native_fill` takes it (`0xAARRGGBB`). The panel is the original's
// warm violet plate; the ring and the star read as alchemy without copying any
// of the original's art, which is the point of drawing it from rectangles.

const PANEL: u32 = 0xF0_1B_16_28;
const PANEL_EDGE: u32 = 0xFF_0D_0A_14;
const BORDER: u32 = 0xFF_6B_5A_8C;
const BORDER_LIGHT: u32 = 0xFF_A8_97_C8;
const SECTION: u32 = 0xC0_2A_22_3E;
const SECTION_LINE: u32 = 0xFF_4A_3D_66;
const SLOT_FILL: u32 = 0xFF_12_0E_1C;
const SLOT_FRAME: u32 = 0xFF_3B_31_52;
const TEXT: u32 = 0xFF_EF_E7_FF;
const TEXT_DIM: u32 = 0xFF_A9_9C_C4;
const ACCENT: u32 = 0xFF_E8_C5_6A;
const BUTTON: u32 = 0xFF_2E_25_45;
const BUTTON_HOVER: u32 = 0xFF_45_37_66;
const BUTTON_HELD: u32 = 0xFF_24_1C_38;
const BUTTON_ON: u32 = 0xFF_5A_46_86;
const SEARCH_FILL: u32 = 0xFF_0B_08_12;
const SEARCH_FOCUS: u32 = 0xFF_6E_58_9E;
const EMBLEM_INK: u32 = 0xFF_3A_2C_5E;
const EMBLEM_GOLD: u32 = 0xFF_D9_B1_4F;
const EMBLEM_GOLD_DIM: u32 = 0xB0_9C_7A_34;

/// The window's GUI size, from the engine's own state (GUI pixels, not
/// physical pixels — the division and the GUI scale are the game's, so the
/// screen never has to guess either).
pub(crate) fn gui_size(host: &dyn Host) -> Option<[f32; 2]> {
    let state = host.client_state()?;
    let size = state.gui_size;
    (size[0] >= 1.0 && size[1] >= 1.0).then_some(size)
}

/// Lay the screen's menu out for the engine: its size, where each of the
/// menu's 54 slots is, and where the player's inventory starts.
///
/// A slot left out is not in the menu at all, so the 16 slots the screen does
/// not show are absent here — and hidden on the server, so a shift-click skips
/// them (see the module note).
pub(crate) fn native_screen() -> NativeScreen {
    // The browser's cells are **not** menu slots: they show the catalogue this
    // screen browses, and the engine draws a menu slot's own stack on top of
    // whatever a screen paints. So only the real slots are placed here.
    let mut slots = vec![None; MENU_SLOTS];
    for (index, slot) in (0..SLOT_COUNT).enumerate() {
        let rect = Layout::slot(index).expect("a placed slot has a rectangle");
        slots[slot] = Some([rect[0], rect[1]]);
    }
    let inventory = Layout::inventory_origin();
    NativeScreen {
        width: layout::PANEL_W,
        height: layout::PANEL_H,
        slots,
        inventory_x: inventory[0],
        inventory_y: inventory[1],
        // The screen draws its own title and its own "Inventory" label, in the
        // player's language; the engine's copies would be the server's title
        // and vanilla's label drawn a second time.
        own_labels: true,
        // The screen draws the panel, its plate, the item grid's frames and
        // the player's frames (`draw_inventory_frames`): with this set the
        // engine draws no slot at all, so anything missing here is missing.
        own_background: true,
    }
}

/// Draws the whole screen for the menu that is open.
///
/// `origin` is the panel's top-left corner in GUI pixels, which the engine
/// computes from the menu and the size `native_screen` gave; the layout is the
/// same one the input handlers hit-test against, so what is drawn and what is
/// clickable cannot drift apart.
pub(crate) fn draw(host: &dyn Host, batch: &mut HudBatch, state: &TableScreen, origin: [f32; 2]) {
    let layout = Layout {
        gui: state.layout.gui,
        origin,
        scale: state.layout.scale,
    };
    let panel = layout.panel();

    panel_plate(host, batch, &layout, panel);
    draw_inventory_frames(host, batch, &layout);
    draw_emblem(host, batch, &layout, state);
    draw_emc(host, batch, &layout, state);
    draw_slots(host, batch, &layout, state);
    draw_browser(host, batch, &layout, state);
    draw_controls(host, batch, &layout, state);

    // The title, centred in the header band: part of the screen, not the menu
    // title the server sent.
    let title = state.text(localization::KEY_TITLE);
    let at = centred(
        host,
        &layout,
        title,
        layout::PANEL_W / 2.0,
        Layout::title_y(),
    );
    text(host, batch, title, at, TEXT, true);

    // The slots' label, above the real slots.
    text(
        host,
        batch,
        state.text(localization::KEY_INPUT),
        layout.point(Layout::slots_label()),
        TEXT_DIM,
        false,
    );

    // The inventory label, above the player's own rows.
    text(
        host,
        batch,
        state.text(localization::KEY_INVENTORY),
        layout.point(Layout::inventory_label()),
        TEXT_DIM,
        false,
    );

    // The screen's bottom line: the note a control's click left, while it is
    // there, and otherwise the reminder that the browser is a preview.
    //
    // A click's notice has to be shown somewhere: the screen hooks are all
    // `&self` (`native_screen_mouse_button_pressed_pre` borrows `Game`
    // immutably), so the chat and action-bar helpers — which take
    // `&mut dyn Host` — cannot be called from them. Drawing the line is the
    // one route the pinned API leaves open, and it keeps the feedback on the
    // screen the click happened on.
    let notice = match state.notice {
        Some(key) => localization::format(
            state.text(localization::KEY_NOTICE_PREFIX),
            &[state.text(key)],
        ),
        None => state.text(localization::KEY_HINT_PLACEHOLDER).to_owned(),
    };
    text(
        host,
        batch,
        &notice,
        layout.point(Layout::notice_line()),
        if state.notice.is_some() {
            TEXT
        } else {
            EMBLEM_GOLD_DIM
        },
        false,
    );
}

/// Push the tooltip for the part under the pointer, if there is one.
///
/// `lines` is collected by the engine and drawn in vanilla's tooltip box, so
/// these tooltips wrap and position like the game's own.
pub(crate) fn tooltip(state: &TableScreen, items: &[&'static str], lines: &mut Vec<String>) {
    if let Some(line) = state.tooltip(items) {
        lines.push(line);
    }
}

/// Whether an open screen is the table's menu screen.
///
/// The screen hooks fire for every screen, so each one asks this first. The
/// menu type is vanilla's `generic_9x6`, so a plain chest a player has open is
/// the same menu; the block the menu belongs to (`native_screen_pos`, the
/// table) tells them apart.
///
/// `view.slots` is the menu's slot **count as the engine reports it**:
/// `Game::slot_count` gives the player's 36 plus the container slots the
/// screen lays out (`native_screen_slot_count`), so the table is 36 + 38 = 74,
/// not the menu's 54. Checking `>= MENU_SLOTS` would be true for any chest, so
/// this compares against the same number `native_screen` builds.
pub(crate) fn is_table(view: &NativeScreenView<'_>, host: &dyn Host) -> bool {
    view.id == "inventory"
        && view.menu == Some(MENU)
        && view.slots == SHOWN_SLOTS + PLAYER_SLOTS
        && table_pos(host).is_some()
}

/// The player's slots the engine counts for every menu screen.
const PLAYER_SLOTS: usize = 36;
/// The container slots this screen lays out: every slot of its menu.
pub(crate) const SHOWN_SLOTS: usize = SLOT_COUNT;

/// The block position the open menu belongs to, when that block is the table.
fn table_pos(host: &dyn Host) -> Option<[i32; 3]> {
    let pos = host.screen_pos()?;
    (host.block(pos) == crate::recipes::TABLE).then_some(pos)
}

/// The browser's item list as the screen browses it today: the preview
/// catalogue, filtered and sorted. One function so the drawing code, the
/// tooltips and the click handler never disagree about what is shown.
pub(crate) fn browse_items(state: &TableScreen) -> Vec<&'static str> {
    state.browser.visible(&super::state::PREVIEW_ITEMS, |id| {
        i64::try_from(crate::emc::value(id)).unwrap_or(i64::MAX)
    })
}

// --- the panels ------------------------------------------------------------

fn panel_plate(host: &dyn Host, batch: &mut HudBatch, layout: &Layout, panel: Rect) {
    fill(host, batch, panel, PANEL);
    let unit = layout.len(1.0);
    outline(host, batch, panel, BORDER, unit);
    // The header band and the left column's plate, which separate the parts of
    // the layout without drawing a line around each one.
    fill(
        host,
        batch,
        layout.at([0.0, 0.0, layout::PANEL_W, 24.0]),
        PANEL_EDGE,
    );
    fill(host, batch, layout.at([4.0, 24.0, 92.0, 136.0]), SECTION);
}

fn draw_browser(host: &dyn Host, batch: &mut HudBatch, layout: &Layout, state: &TableScreen) {
    let items = browse_items(state);
    let page = state.browser.page_items(&items);
    let grid = Layout::grid();

    fill(host, batch, layout.at(expand(grid, 3.0)), SECTION);
    // The cells are the browser's own: a frame and an icon from
    // `native_item_stack`, and no menu slot underneath (see the module note).
    for index in 0..layout::PAGE_SIZE {
        let Some(cell) = Layout::cell(index) else {
            continue;
        };
        let rect = layout.at(cell);
        let hovered = state.hit().is_some_and(|hit| hit.part == Part::Cell(index));
        slot_frame(host, batch, rect, hovered, layout.len(1.0));
        if let Some(item) = page.get(index) {
            draw_item(host, batch, item, rect);
        }
    }

    if page.is_empty() {
        let key = if state.browser.search.is_empty() {
            localization::KEY_EMPTY
        } else {
            localization::KEY_NO_MATCH
        };
        let line = state.text(key);
        let at = centred(host, layout, line, grid_centre_x(), grid_centre_y());
        text(host, batch, line, at, TEXT_DIM, false);
    }

    // The search field: its text, or its hint, and a caret while it is typing.
    let field = layout.at(Layout::control(Control::Search));
    fill(host, batch, field, SEARCH_FILL);
    outline(
        host,
        batch,
        field,
        if state.typing {
            SEARCH_FOCUS
        } else {
            SLOT_FRAME
        },
        layout.len(1.0),
    );
    let pad = layout.len(3.0);
    if state.browser.search.is_empty() && !state.typing {
        text(
            host,
            batch,
            state.text(localization::KEY_SEARCH_HINT),
            [field[0] + pad, field[1] + pad],
            TEXT_DIM,
            false,
        );
    } else {
        let shown = fit_text(
            host,
            layout,
            &state.browser.search,
            field[2] - pad * 2.0 - layout.len(2.0),
        );
        text(
            host,
            batch,
            &shown,
            [field[0] + pad, field[1] + pad],
            TEXT,
            false,
        );
        if state.typing {
            let after = host.text_width(&shown);
            fill(
                host,
                batch,
                [
                    field[0] + pad + after + 1.0,
                    field[1] + pad,
                    layout.len(1.0),
                    layout.len(8.0),
                ],
                ACCENT,
            );
        }
    }
}

fn draw_controls(host: &dyn Host, batch: &mut HudBatch, layout: &Layout, state: &TableScreen) {
    let items = browse_items(state);

    // Sort and filter show the state they are in, so a press is visible.
    button(
        host,
        batch,
        layout,
        state,
        Part::Sort,
        Control::Sort,
        state.text(state.browser.sort.key()),
        false,
    );
    button(
        host,
        batch,
        layout,
        state,
        Part::Filter,
        Control::Filter,
        state.text(state.browser.filter.key()),
        state.browser.filter != super::state::Filter::All,
    );

    // Pages: an arrow each side of the indicator. An arrow with nowhere to go
    // is drawn dim, but it still answers a press (with no change), so no
    // control on this screen ignores a click.
    button(
        host,
        batch,
        layout,
        state,
        Part::PagePrev,
        Control::PagePrev,
        "<",
        false,
    );
    button(
        host,
        batch,
        layout,
        state,
        Part::PageNext,
        Control::PageNext,
        ">",
        false,
    );

    let rect = layout.at(Layout::control(Control::PageLabel));
    let label = state.page_label(&items);
    let at = centred_in(host, rect, &label, 0.0);
    text(host, batch, &label, at, TEXT_DIM, false);
}

#[allow(clippy::too_many_arguments)]
fn button(
    host: &dyn Host,
    batch: &mut HudBatch,
    layout: &Layout,
    state: &TableScreen,
    part: Part,
    control: Control,
    label: &str,
    on: bool,
) {
    let rect = layout.at(Layout::control(control));
    let hovered = state.hit().is_some_and(|hit| hit.part == part);
    let colour = if state.is_held(part) {
        BUTTON_HELD
    } else if hovered {
        BUTTON_HOVER
    } else if on {
        BUTTON_ON
    } else {
        BUTTON
    };
    fill(host, batch, rect, colour);
    outline(
        host,
        batch,
        rect,
        if hovered || on {
            BORDER_LIGHT
        } else {
            SLOT_FRAME
        },
        layout.len(1.0),
    );
    // A held button's label sinks by one GUI pixel, as vanilla's buttons do.
    let sink = if state.is_held(part) {
        layout.len(1.0)
    } else {
        0.0
    };
    let at = centred_in(host, rect, label, layout.len(1.0));
    text(
        host,
        batch,
        label,
        [at[0], at[1] + sink],
        if hovered || on { TEXT } else { TEXT_DIM },
        false,
    );
}

/// The alchemical emblem: a ringed disc with a four-point star, drawn from
/// rectangles so it needs no texture and stays crisp at every GUI scale.
///
/// Its outer ring brightens while the browser is being searched, so the left
/// panel reflects the right panel's state rather than sitting inert.
fn draw_emblem(host: &dyn Host, batch: &mut HudBatch, layout: &Layout, state: &TableScreen) {
    let rect = layout.at(Layout::emblem());
    let unit = layout.len(1.0);
    let centre = [rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0];
    let active = !state.browser.search.is_empty() || state.typing;
    let ring = if active { ACCENT } else { EMBLEM_GOLD_DIM };

    fill(host, batch, rect, EMBLEM_INK);
    outline(host, batch, rect, EMBLEM_GOLD_DIM, unit);

    for (inset, colour) in [
        (unit * 4.0, ring),
        (unit * 10.0, EMBLEM_GOLD_DIM),
        (unit * 16.0, ring),
    ] {
        let ring_rect = [
            rect[0] + inset,
            rect[1] + inset,
            rect[2] - inset * 2.0,
            rect[3] - inset * 2.0,
        ];
        outline(host, batch, ring_rect, colour, unit);
    }

    let arm = rect[2] * 0.32;
    fill(
        host,
        batch,
        [
            centre[0] - arm,
            centre[1] - unit * 1.5,
            arm * 2.0,
            unit * 3.0,
        ],
        EMBLEM_GOLD,
    );
    fill(
        host,
        batch,
        [
            centre[0] - unit * 1.5,
            centre[1] - arm,
            unit * 3.0,
            arm * 2.0,
        ],
        EMBLEM_GOLD,
    );
    fill(
        host,
        batch,
        [
            centre[0] - unit * 4.0,
            centre[1] - unit * 4.0,
            unit * 8.0,
            unit * 8.0,
        ],
        EMBLEM_INK,
    );
    fill(
        host,
        batch,
        [
            centre[0] - unit * 2.0,
            centre[1] - unit * 2.0,
            unit * 4.0,
            unit * 4.0,
        ],
        ring,
    );

    let stud = unit * 3.0;
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let at = [
            centre[0] + dx * (rect[2] / 2.0 - unit * 5.0) - stud / 2.0,
            centre[1] + dy * (rect[3] / 2.0 - unit * 5.0) - stud / 2.0,
        ];
        fill(host, batch, [at[0], at[1], stud, stud], EMBLEM_GOLD_DIM);
    }
}

/// The EMC balance.
///
/// The bar and the number are a fixed **placeholder** picture: this screen does
/// not read, spend or earn EMC, and the note under the panel says the browser
/// is a preview. The real balance and the real transmutation are the existing
/// `/projecte` paths, untouched by this screen.
fn draw_emc(host: &dyn Host, batch: &mut HudBatch, layout: &Layout, state: &TableScreen) {
    let bar = layout.at(Layout::emc_bar());
    fill(host, batch, bar, SECTION_LINE);
    fill(host, batch, [bar[0], bar[1], bar[2] * 0.2, bar[3]], ACCENT);
    outline(host, batch, bar, SLOT_FRAME, layout.len(1.0));
    text(
        host,
        batch,
        state.text(localization::KEY_EMC),
        layout.point(Layout::emc_label()),
        TEXT_DIM,
        false,
    );
    let value = "1 000".to_owned();
    let at = centred_in(host, bar, &value, 0.0);
    // Dark ink on the gold fill: the number has to read against it.
    text(host, batch, &value, at, PANEL_EDGE, false);
}

/// The real slots: the two inputs, then the storage.
///
/// These frames and the placed slots in `native_screen` are the same
/// rectangles, so the frame is exactly where the engine's slot is. The items
/// in them are the engine's to draw (`draw_inventory_slots` covers every slot
/// in the menu), so nothing here paints a stack — a second copy would double
/// every item and drift from the server's truth.
fn draw_slots(host: &dyn Host, batch: &mut HudBatch, layout: &Layout, state: &TableScreen) {
    fill(host, batch, layout.at(Layout::slots_area()), SECTION);
    for index in 0..SLOT_COUNT {
        let Some(rect) = Layout::slot(index) else {
            continue;
        };
        // An input slot is marked by a warmer frame, so the two the task calls
        // inputs are told apart from the storage at a glance.
        let input = index < INPUT_SLOTS.len();
        let hovered = state.hit().is_some_and(|hit| hit.part == Part::Slot(index));
        let rect = layout.at(rect);
        slot_frame(host, batch, rect, hovered, layout.len(1.0));
        if input {
            fill(
                host,
                batch,
                [rect[0] + 1.0, rect[1] + 1.0, rect[2] - 2.0, layout.len(2.0)],
                ACCENT,
            );
        }
    }
}

/// The player's 36 slot frames and their items.
///
/// With `own_background` the engine draws **no** slot at all — not even the
/// player's, which it would otherwise draw behind every mod screen — so the
/// only place these frames can come from is here. The items are not drawn
/// here: the engine draws every slot's stack itself (`draw_inventory_slots`),
/// so these are frames only and nothing is painted twice.
fn draw_inventory_frames(host: &dyn Host, batch: &mut HudBatch, layout: &Layout) {
    for index in 0..36 {
        let Some(rect) = Layout::inventory_slot(index) else {
            continue;
        };
        slot_frame(host, batch, layout.at(rect), false, layout.len(1.0));
    }
}

// --- primitives ------------------------------------------------------------

fn slot_frame(host: &dyn Host, batch: &mut HudBatch, rect: Rect, hovered: bool, unit: f32) {
    fill(host, batch, rect, SLOT_FILL);
    outline(
        host,
        batch,
        rect,
        if hovered { BORDER_LIGHT } else { SLOT_FRAME },
        unit,
    );
    if hovered {
        fill(
            host,
            batch,
            [
                rect[0] + unit,
                rect[1] + unit,
                rect[2] - unit * 2.0,
                rect[3] - unit * 2.0,
            ],
            0x30_FF_FF_FF,
        );
    }
}

/// An item's icon in its slot, with the engine's own decorations
/// (`native_item_stack`, API 313), so a stack looks as it does anywhere else.
fn draw_item(host: &dyn Host, batch: &mut HudBatch, item: &str, slot: Rect) {
    let stack = ItemStack::new(item, 1);
    let at = [slot[0] + 1.0, slot[1] + 1.0];
    host.item_stack(batch, &stack, at, 16.0);
}

/// Four thin strips: an outline with a transparent middle, unlike two
/// overlapping fills.
fn outline(host: &dyn Host, batch: &mut HudBatch, rect: Rect, argb: u32, width: f32) {
    if width <= 0.0 || rect[2] <= 0.0 || rect[3] <= 0.0 {
        return;
    }
    fill(host, batch, [rect[0], rect[1], rect[2], width], argb);
    fill(
        host,
        batch,
        [rect[0], rect[1] + rect[3] - width, rect[2], width],
        argb,
    );
    fill(host, batch, [rect[0], rect[1], width, rect[3]], argb);
    fill(
        host,
        batch,
        [rect[0] + rect[2] - width, rect[1], width, rect[3]],
        argb,
    );
}

fn fill(host: &dyn Host, batch: &mut HudBatch, rect: Rect, argb: u32) {
    if rect[2] > 0.0 && rect[3] > 0.0 {
        host.fill(batch, rect, argb);
    }
}

fn text(host: &dyn Host, batch: &mut HudBatch, line: &str, at: [f32; 2], rgb: u32, shadow: bool) {
    if line.is_empty() {
        return;
    }
    host.text(batch, line, at, rgb, shadow);
}

/// Centred text at `x` in panel coordinates, at panel `y`.
fn centred(host: &dyn Host, layout: &Layout, line: &str, x: f32, y: f32) -> [f32; 2] {
    let at = layout.point([x, y]);
    [at[0] - host.text_width(line) / 2.0, at[1]]
}

/// Centred text inside a drawn rectangle, raised by `pad`.
fn centred_in(host: &dyn Host, rect: Rect, line: &str, pad: f32) -> [f32; 2] {
    let width = host.text_width(line);
    [
        rect[0] + (rect[2] - width) / 2.0,
        rect[1] + (rect[3] - 8.0) / 2.0 - pad,
    ]
}

fn grid_centre_x() -> f32 {
    let grid = Layout::grid();
    grid[0] + grid[2] / 2.0
}

fn grid_centre_y() -> f32 {
    let grid = Layout::grid();
    grid[1] + grid[3] / 2.0
}

fn expand(rect: Rect, by: f32) -> Rect {
    [
        rect[0] - by,
        rect[1] - by,
        rect[2] + by * 2.0,
        rect[3] + by * 2.0,
    ]
}

/// The end of `line` that fits in `width` screen pixels, at least one char.
///
/// A search field keeps the newest text visible while it is typed, so the cut
/// is at the start: a long query shows its end.
fn fit_text(host: &dyn Host, layout: &Layout, line: &str, width: f32) -> String {
    let width = width / layout.scale.max(0.01);
    if host.text_width(line) <= width {
        return line.to_owned();
    }
    let mut start = 0;
    while start < line.chars().count() {
        let candidate: String = line.chars().skip(start).collect();
        if host.text_width(&candidate) <= width {
            return candidate;
        }
        start += 1;
    }
    String::new()
}

// --- tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::localization::Language;
    use crate::ui::state::TableScreen;

    fn screen() -> TableScreen {
        TableScreen::new(Language::English, [640.0, 360.0])
    }

    #[test]
    fn every_real_slot_is_placed_and_none_is_hidden() {
        let native = native_screen();
        assert_eq!(native.width, layout::PANEL_W);
        assert_eq!(native.height, layout::PANEL_H);
        assert!(native.own_labels && native.own_background);
        assert_eq!(native.slots.len(), MENU_SLOTS);

        // Every slot of the menu is placed, at `Layout::slot`'s corner.
        for index in 0..SLOT_COUNT {
            let rect = Layout::slot(index).expect("a shown slot");
            assert_eq!(
                native.slots[index],
                Some([rect[0], rect[1]]),
                "slot {index}"
            );
        }
        // Nothing is hidden, so the server's menu is the whole menu.
        let shown = native.slots.iter().filter(|slot| slot.is_some()).count();
        assert_eq!(shown, SHOWN_SLOTS);
        assert_eq!(shown, MENU_SLOTS);
    }

    #[test]
    fn the_browser_cells_are_not_menu_slots() {
        // A menu slot's stack is drawn by the engine over whatever the screen
        // paints, so a catalogue cell that were also a slot would show an item
        // the player never put there. The cells are the browser's own.
        let native = native_screen();
        for index in 0..layout::PAGE_SIZE {
            let cell = Layout::cell(index).expect("a browser cell");
            for placed in native.slots.iter().flatten() {
                assert_ne!(*placed, [cell[0], cell[1]], "cell {index} is a menu slot");
            }
        }
    }

    #[test]
    fn the_real_slots_do_not_overlap_the_browser_or_the_inputs() {
        let grid = Layout::grid();
        for index in 0..SLOT_COUNT {
            let rect = Layout::slot(index).unwrap();
            let covers = |a: Rect, b: Rect| {
                a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
            };
            assert!(!covers(rect, grid), "slot {index} covers the browser");
            assert!(
                rect[0] + rect[2] <= layout::PANEL_W && rect[1] + rect[3] <= layout::PANEL_H,
                "slot {index} leaves the panel"
            );
        }
    }

    #[test]
    fn the_inventory_starts_below_the_panels_own_rows() {
        let native = native_screen();
        assert_eq!(native.inventory_x, Layout::inventory_origin()[0]);
        assert_eq!(native.inventory_y, Layout::inventory_origin()[1]);
        // Below the last grid row (48 + 4 * 20 - 2 = 126) and inside the panel.
        assert!(native.inventory_y > 126.0);
        assert!(native.inventory_y < layout::PANEL_H);
    }

    #[test]
    fn the_menu_type_is_vanillas_single_row_chest() {
        // `MOD_MENUS` in the pinned engine pairs this name with nine slots.
        assert_eq!(MENU, "minecraft:generic_9x1");
        assert_eq!(MENU_SLOTS, 9);
    }

    #[test]
    fn the_slot_count_the_guard_checks_is_the_one_the_layout_shows() {
        // `Game::slot_count` answers 36 + `native_screen_slot_count`, and
        // `is_table` compares `view.slots` against exactly that, so this keeps
        // the two in step.
        let native = native_screen();
        let shown = native.slots.iter().filter(|slot| slot.is_some()).count();
        assert_eq!(shown, SHOWN_SLOTS);
        assert_eq!(SHOWN_SLOTS, MENU_SLOTS);
        assert_eq!(SHOWN_SLOTS + PLAYER_SLOTS, MENU_SLOTS + 36);
    }

    #[test]
    fn the_browser_list_and_the_drawn_list_are_the_same_call() {
        let mut state = screen();
        let items = browse_items(&state);
        assert_eq!(
            items,
            state
                .browser
                .visible(&crate::ui::state::PREVIEW_ITEMS, |id| {
                    i64::try_from(crate::emc::value(id)).unwrap_or(i64::MAX)
                })
        );
        state.browser.filter = crate::ui::state::Filter::Tools;
        assert!(browse_items(&state).is_empty());
    }

    #[test]
    fn the_drawn_geometry_is_the_engines_geometry() {
        // The engine centres the screen at `centered_origin(320, 240)` and
        // places the menu's slots at 1:1 from there. This screen draws through
        // the same numbers, so at any window size the first cell is drawn
        // exactly where `NativeScreen.slots` says the slot is.
        for gui in [[640.0, 360.0], [200.0, 160.0], [1920.0, 1080.0]] {
            let layout = Layout::fit(gui);
            let native = native_screen();
            for index in [0usize, 3, 8] {
                let cell = Layout::slot(index).unwrap();
                let placed = native.slots[index].expect("the slot is placed");
                assert_eq!(placed, [cell[0], cell[1]]);
                let drawn = layout.at(cell);
                assert_eq!(drawn[0], layout.origin[0] + placed[0]);
                assert_eq!(drawn[1], layout.origin[1] + placed[1]);
                assert_eq!((drawn[2], drawn[3]), (layout::SLOT, layout::SLOT));
            }
        }
    }
}
