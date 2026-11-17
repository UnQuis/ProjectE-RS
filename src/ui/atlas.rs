//! The Transmutation Table screen's one place that calls the drawing API.
//!
//! All layout, words and control behaviour live in `super::layout`,
//! `super::localization` and `super::state`, which compile and test without a
//! game. This file only turns that model into SDK 0.2 `DrawContext` calls.

use mod_api::{DrawContext, ModContainer, Screen, ServerHost};

use super::layout::{self, Control, Layout, Part, Rect};
pub(crate) use super::layout::{INPUT_SLOTS, MENU_SLOTS, SLOT_COUNT};
use super::localization;
use super::state::TableScreen;

pub(crate) const MOD_ID: &str = "projecte";

pub(crate) const MENU: &str = "projecte:transmutation_table";

/// Read the item in one slot of the open table menu, as `(item id, count)`.
///
/// SDK 0.2 exposes menu slots through `MenuRef`; there is no
/// `menu_contents(uuid)` helper. This is the server-side read the consume,
/// unlearn and lock slot handlers need.
pub(crate) fn menu_item(
    host: &mut dyn ServerHost,
    uuid: u128,
    window: i32,
    slot: usize,
) -> Option<(String, u32)> {
    let menu = host.menu(uuid, window)?;
    let stack = menu.slot(host, slot)??;
    Some((stack.item().id(host), stack.count()))
}

/// The table's container, with the ProjectE slot roles exposed to automation.
pub(crate) fn table_container() -> ModContainer {
    let inputs: Vec<usize> = (super::table::INPUT_FIRST..super::table::INPUT_LAST).collect();
    let outputs: Vec<usize> = (super::table::OUTPUT_FIRST..super::table::OUTPUT_LAST).collect();
    let mut faces = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for face in &mut faces {
        *face = inputs.clone();
    }
    ModContainer {
        slots: MENU_SLOTS,
        faces,
        inputs,
        outputs,
        menu: Some(MENU.to_owned()),
    }
}

// ARGB colours (0xAARRGGBB).
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

/// Lay the screen's menu out for the engine. The browser cells are not menu
/// slots: a menu slot's stack is drawn by the engine over whatever we paint.
pub(crate) fn native_screen() -> Screen {
    let mut slots = vec![None; MENU_SLOTS];
    for (index, slot) in (0..SLOT_COUNT).enumerate() {
        let rect = Layout::slot(index).expect("a placed slot has a rectangle");
        slots[slot] = Some([rect[0], rect[1]]);
    }
    let inventory = Layout::inventory_origin();
    Screen {
        width: layout::PANEL_W,
        height: layout::PANEL_H,
        slots,
        inventory_x: inventory[0],
        inventory_y: inventory[1],
        own_labels: true,
        own_background: true,
        recipe_book: None,
        recipe_book_toggle: None,
    }
}

/// Draws the whole screen for the menu that is open.
pub(crate) fn draw(draw: &mut dyn DrawContext, state: &TableScreen, origin: [f32; 2]) {
    let layout = Layout {
        gui: state.layout.gui,
        origin,
        scale: state.layout.scale,
    };
    let panel = layout.panel();

    panel_plate(draw, &layout, panel);
    draw_inventory_frames(draw, &layout);
    draw_emblem(draw, &layout, state);
    draw_emc(draw, &layout, state);
    draw_slots(draw, &layout, state);
    draw_browser(draw, &layout, state);
    draw_controls(draw, &layout, state);

    let title = state.text(localization::KEY_TITLE);
    let at = centred(draw, &layout, title, layout::PANEL_W / 2.0, Layout::title_y());
    text(draw, title, at, TEXT, true);
    text(
        draw,
        state.text(localization::KEY_INPUT),
        layout.point(Layout::slots_label()),
        TEXT_DIM,
        false,
    );
    text(
        draw,
        state.text(localization::KEY_INVENTORY),
        layout.point(Layout::inventory_label()),
        TEXT_DIM,
        false,
    );

    let notice = match state.notice {
        Some(key) => localization::format(
            state.text(localization::KEY_NOTICE_PREFIX),
            &[state.text(key)],
        ),
        None => state.text(localization::KEY_HINT_PLACEHOLDER).to_owned(),
    };
    text(
        draw,
        &notice,
        layout.point(Layout::notice_line()),
        if state.notice.is_some() { TEXT } else { EMBLEM_GOLD_DIM },
        false,
    );
}

pub(crate) fn tooltip(state: &TableScreen, items: &[&'static str], lines: &mut Vec<String>) {
    if let Some(line) = state.tooltip(items) {
        lines.push(line);
    }
}

/// The player's slots the engine counts for every menu screen.
pub(crate) const PLAYER_SLOTS: usize = 36;
pub(crate) const SHOWN_SLOTS: usize = SLOT_COUNT;

pub(crate) fn browse_items(state: &TableScreen) -> Vec<&'static str> {
    state
        .browser
        .visible(&super::state::PREVIEW_ITEMS, crate::emc::value)
}

fn panel_plate(draw: &mut dyn DrawContext, layout: &Layout, panel: Rect) {
    fill(draw, panel, PANEL);
    let unit = layout.len(1.0);
    outline(draw, panel, BORDER, unit);
    fill(
        draw,
        layout.at([0.0, 0.0, layout::PANEL_W, 24.0]),
        PANEL_EDGE,
    );
    fill(draw, layout.at([4.0, 24.0, 92.0, 136.0]), SECTION);
}

fn draw_browser(draw: &mut dyn DrawContext, layout: &Layout, state: &TableScreen) {
    let items = browse_items(state);
    let page = state.browser.page_items(&items);
    let grid = Layout::grid();

    fill(draw, layout.at(expand(grid, 3.0)), SECTION);
    for index in 0..layout::PAGE_SIZE {
        let Some(cell) = Layout::cell(index) else {
            continue;
        };
        let rect = layout.at(cell);
        let hovered = state.hit().is_some_and(|hit| hit.part == Part::Cell(index));
        slot_frame(draw, rect, hovered, layout.len(1.0));
        if let Some(item) = page.get(index) {
            draw_item(draw, item, rect);
        }
    }

    if page.is_empty() {
        let key = if state.browser.search.is_empty() {
            localization::KEY_EMPTY
        } else {
            localization::KEY_NO_MATCH
        };
        let line = state.text(key);
        let at = centred(draw, layout, line, grid_centre_x(), grid_centre_y());
        text(draw, line, at, TEXT_DIM, false);
    }

    let field = layout.at(Layout::control(Control::Search));
    fill(draw, field, SEARCH_FILL);
    outline(
        draw,
        field,
        if state.typing { SEARCH_FOCUS } else { SLOT_FRAME },
        layout.len(1.0),
    );
    let pad = layout.len(3.0);
    if state.browser.search.is_empty() && !state.typing {
        text(
            draw,
            state.text(localization::KEY_SEARCH_HINT),
            [field[0] + pad, field[1] + pad],
            TEXT_DIM,
            false,
        );
    } else {
        let shown = fit_text(draw, layout, &state.browser.search, field[2] - pad * 2.0 - layout.len(2.0));
        text(draw, &shown, [field[0] + pad, field[1] + pad], TEXT, false);
        if state.typing {
            let after = draw.text_width(&shown);
            fill(
                draw,
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

fn draw_controls(draw: &mut dyn DrawContext, layout: &Layout, state: &TableScreen) {
    let items = browse_items(state);
    button(
        draw, layout, state, Part::Sort, Control::Sort,
        state.text(state.browser.sort.key()), false,
    );
    button(
        draw, layout, state, Part::Filter, Control::Filter,
        state.text(state.browser.filter.key()),
        state.browser.filter != super::state::Filter::All,
    );
    button(draw, layout, state, Part::PagePrev, Control::PagePrev, "<", false);
    button(draw, layout, state, Part::PageNext, Control::PageNext, ">", false);

    let rect = layout.at(Layout::control(Control::PageLabel));
    let label = state.page_label(&items);
    let at = centred_in(draw, rect, &label, 0.0);
    text(draw, &label, at, TEXT_DIM, false);
}

#[allow(clippy::too_many_arguments)]
fn button(
    draw: &mut dyn DrawContext,
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
    fill(draw, rect, colour);
    outline(
        draw,
        rect,
        if hovered || on { BORDER_LIGHT } else { SLOT_FRAME },
        layout.len(1.0),
    );
    let sink = if state.is_held(part) { layout.len(1.0) } else { 0.0 };
    let at = centred_in(draw, rect, label, layout.len(1.0));
    text(
        draw,
        label,
        [at[0], at[1] + sink],
        if hovered || on { TEXT } else { TEXT_DIM },
        false,
    );
}

fn draw_emblem(draw: &mut dyn DrawContext, layout: &Layout, state: &TableScreen) {
    let rect = layout.at(Layout::emblem());
    let unit = layout.len(1.0);
    let centre = [rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0];
    let active = !state.browser.search.is_empty() || state.typing;
    let ring = if active { ACCENT } else { EMBLEM_GOLD_DIM };

    fill(draw, rect, EMBLEM_INK);
    outline(draw, rect, EMBLEM_GOLD_DIM, unit);
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
        outline(draw, ring_rect, colour, unit);
    }
    let arm = rect[2] * 0.32;
    fill(draw, [centre[0] - arm, centre[1] - unit * 1.5, arm * 2.0, unit * 3.0], EMBLEM_GOLD);
    fill(draw, [centre[0] - unit * 1.5, centre[1] - arm, unit * 3.0, arm * 2.0], EMBLEM_GOLD);
    fill(draw, [centre[0] - unit * 4.0, centre[1] - unit * 4.0, unit * 8.0, unit * 8.0], EMBLEM_INK);
    fill(draw, [centre[0] - unit * 2.0, centre[1] - unit * 2.0, unit * 4.0, unit * 4.0], ring);
    let stud = unit * 3.0;
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let at = [
            centre[0] + dx * (rect[2] / 2.0 - unit * 5.0) - stud / 2.0,
            centre[1] + dy * (rect[3] / 2.0 - unit * 5.0) - stud / 2.0,
        ];
        fill(draw, [at[0], at[1], stud, stud], EMBLEM_GOLD_DIM);
    }
}

fn draw_emc(draw: &mut dyn DrawContext, layout: &Layout, state: &TableScreen) {
    let bar = layout.at(Layout::emc_bar());
    fill(draw, bar, SECTION_LINE);
    fill(draw, [bar[0], bar[1], bar[2] * 0.2, bar[3]], ACCENT);
    outline(draw, bar, SLOT_FRAME, layout.len(1.0));
    text(draw, state.text(localization::KEY_EMC), layout.point(Layout::emc_label()), TEXT_DIM, false);
    let value = localization::format_emc(1_000);
    let at = centred_in(draw, bar, &value, 0.0);
    text(draw, &value, at, PANEL_EDGE, false);
}

fn draw_slots(draw: &mut dyn DrawContext, layout: &Layout, state: &TableScreen) {
    fill(draw, layout.at(Layout::slots_area()), SECTION);
    for index in 0..SLOT_COUNT {
        let Some(rect) = Layout::slot(index) else { continue };
        let input = index < INPUT_SLOTS.len();
        let hovered = state.hit().is_some_and(|hit| hit.part == Part::Slot(index));
        let rect = layout.at(rect);
        slot_frame(draw, rect, hovered, layout.len(1.0));
        if input {
            fill(draw, [rect[0] + 1.0, rect[1] + 1.0, rect[2] - 2.0, layout.len(2.0)], ACCENT);
        }
    }
}

fn draw_inventory_frames(draw: &mut dyn DrawContext, layout: &Layout) {
    for index in 0..36 {
        let Some(rect) = Layout::inventory_slot(index) else { continue };
        slot_frame(draw, layout.at(rect), false, layout.len(1.0));
    }
}

fn slot_frame(draw: &mut dyn DrawContext, rect: Rect, hovered: bool, unit: f32) {
    fill(draw, rect, SLOT_FILL);
    outline(draw, rect, if hovered { BORDER_LIGHT } else { SLOT_FRAME }, unit);
    if hovered {
        fill(draw, [rect[0] + unit, rect[1] + unit, rect[2] - unit * 2.0, rect[3] - unit * 2.0], 0x30_FF_FF_FF);
    }
}

fn draw_item(draw: &mut dyn DrawContext, _item: &str, slot: Rect) {
    // The SDK's DrawContext draws an already-resolved stack; the preview
    // catalogue is ids, so no stack is painted here until it is wired to a
    // registry-backed item.
    let _ = (draw, slot);
}

fn outline(draw: &mut dyn DrawContext, rect: Rect, argb: u32, width: f32) {
    if width <= 0.0 || rect[2] <= 0.0 || rect[3] <= 0.0 {
        return;
    }
    fill(draw, [rect[0], rect[1], rect[2], width], argb);
    fill(draw, [rect[0], rect[1] + rect[3] - width, rect[2], width], argb);
    fill(draw, [rect[0], rect[1], width, rect[3]], argb);
    fill(draw, [rect[0] + rect[2] - width, rect[1], width, rect[3]], argb);
}

fn fill(draw: &mut dyn DrawContext, rect: Rect, argb: u32) {
    if rect[2] > 0.0 && rect[3] > 0.0 {
        draw.fill(rect, argb);
    }
}

fn text(draw: &mut dyn DrawContext, line: &str, at: [f32; 2], rgb: u32, shadow: bool) {
    if !line.is_empty() {
        draw.text(line, at, rgb, shadow);
    }
}

fn centred(draw: &mut dyn DrawContext, layout: &Layout, line: &str, x: f32, y: f32) -> [f32; 2] {
    let at = layout.point([x, y]);
    [at[0] - draw.text_width(line) / 2.0, at[1]]
}

fn centred_in(draw: &mut dyn DrawContext, rect: Rect, line: &str, pad: f32) -> [f32; 2] {
    let width = draw.text_width(line);
    [rect[0] + (rect[2] - width) / 2.0, rect[1] + (rect[3] - 8.0) / 2.0 - pad]
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
    [rect[0] - by, rect[1] - by, rect[2] + by * 2.0, rect[3] + by * 2.0]
}

fn fit_text(draw: &mut dyn DrawContext, layout: &Layout, line: &str, width: f32) -> String {
    let width = width / layout.scale.max(0.01);
    if draw.text_width(line) <= width {
        return line.to_owned();
    }
    let mut start = 0;
    while start < line.chars().count() {
        let candidate: String = line.chars().skip(start).collect();
        if draw.text_width(&candidate) <= width {
            return candidate;
        }
        start += 1;
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::state::TableScreen;
    use crate::ui::localization::Language;

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
        for index in 0..SLOT_COUNT {
            let rect = Layout::slot(index).expect("a shown slot");
            assert_eq!(native.slots[index], Some([rect[0], rect[1]]), "slot {index}");
        }
    }

    #[test]
    fn the_browser_cells_are_not_menu_slots() {
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
        }
    }

    #[test]
    fn the_inventory_starts_below_the_panels_own_rows() {
        let native = native_screen();
        assert_eq!(native.inventory_x, Layout::inventory_origin()[0]);
        assert!(native.inventory_y > 126.0);
        assert!(native.inventory_y < layout::PANEL_H);
    }

    #[test]
    fn the_table_uses_its_own_menu_type_with_a_data_slot() {
        assert_eq!(MENU, "projecte:transmutation_table");
        assert_eq!(MENU_SLOTS, 27);
    }

    #[test]
    fn the_browser_list_and_the_drawn_list_are_the_same_call() {
        let mut state = screen();
        let items = browse_items(&state);
        assert_eq!(
            items,
            state.browser.visible(&crate::ui::state::PREVIEW_ITEMS, crate::emc::value)
        );
        state.browser.filter = crate::ui::state::Filter::Tools;
        assert!(browse_items(&state).is_empty());
    }
}
