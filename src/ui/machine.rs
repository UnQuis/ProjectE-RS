//! Shared screen for the ProjectE machines (collector, relay, condenser,
//! alchemical chest). The table has its own browser screen in `super::atlas`;
//! here the whole container is laid out as real slots and a data-slot-driven
//! progress bar shows the machine's EMC fill.

use mod_api::{DrawContext, Screen};

use crate::machines;

const COLS: usize = 9;
const PITCH: f32 = 20.0;
const SLOT: f32 = 18.0;
const MARGIN: f32 = 8.0;
const SLOTS_Y: f32 = 20.0;

const PANEL: u32 = 0xF0_1B_16_28;
const FRAME: u32 = 0xFF_6B_5A_8C;
const SLOT_FILL: u32 = 0xFF_12_0E_1C;
const SLOT_EDGE: u32 = 0xFF_3B_31_52;
const TEXT: u32 = 0xFF_EF_E7_FF;
const BAR_BG: u32 = 0xFF_0B_08_12;
const BAR_FILL: u32 = 0xFF_E8_C5_6A;

pub(crate) fn is_machine_screen(block: &str) -> bool {
    block != crate::recipes::TABLE && machines::container(block).is_some()
}

pub(crate) fn screen(block: &str) -> Option<Screen> {
    let container = machines::container(block)?;
    if block == crate::recipes::TABLE {
        return None;
    }
    let rows = container.slots.div_ceil(COLS);
    let inventory_y = SLOTS_Y + rows as f32 * PITCH + 14.0;
    let width = (COLS as f32 * PITCH + MARGIN * 2.0).max(176.0);
    let height = inventory_y + 3.0 * SLOT + 58.0 + 10.0;
    let slots = (0..container.slots)
        .map(|index| {
            Some([
                MARGIN + (index % COLS) as f32 * PITCH,
                SLOTS_Y + (index / COLS) as f32 * PITCH,
            ])
        })
        .collect();
    Some(Screen {
        width,
        height,
        slots,
        inventory_x: MARGIN,
        inventory_y,
        own_labels: true,
        own_background: true,
        recipe_book: None,
        recipe_book_toggle: None,
    })
}

pub(crate) fn draw(draw: &mut dyn DrawContext, block: &str, data: &[i32], origin: [f32; 2]) {
    let Some(layout) = screen(block) else {
        return;
    };
    fill(draw, [origin[0], origin[1], layout.width, layout.height], PANEL);
    let title = crate::emc::display_name(block);
    draw.text(&title, [origin[0] + MARGIN, origin[1] + 5.0], TEXT, true);

    for (index, slot) in layout.slots.iter().enumerate() {
        let Some([x, y]) = slot else { continue };
        let rect = [origin[0] + x, origin[1] + y, SLOT, SLOT];
        fill(draw, rect, SLOT_FILL);
        outline(draw, rect, SLOT_EDGE, 1.0);
        // Result/aux slots get a brighter edge, as the table's inputs do.
        if machines::container(block).is_some_and(|container| container.outputs.contains(&index)) {
            outline(draw, rect, FRAME, 1.0);
        }
    }

    // Data slots are 16-bit on the wire. Slot 0 is EMC fill (0..1000);
    // collectors also send sun level, star charge and fuel progress.
    let bars = data.len().clamp(1, 4);
    let bar_w = (layout.width - MARGIN * 2.0 - (bars as f32 - 1.0) * 4.0) / bars as f32;
    for (index, value) in data.iter().take(4).enumerate() {
        let maximum = if index == 1 && bars == 4 { 16.0 } else { 1000.0 };
        let filled = (*value as f32).clamp(0.0, maximum) / maximum;
        let bar = [
            origin[0] + MARGIN + index as f32 * (bar_w + 4.0),
            origin[1] + layout.inventory_y - 12.0,
            bar_w,
            8.0,
        ];
        fill(draw, bar, BAR_BG);
        fill(draw, [bar[0], bar[1], bar[2] * filled, bar[3]], BAR_FILL);
        outline(draw, bar, SLOT_EDGE, 1.0);
    }
}

fn fill(draw: &mut dyn DrawContext, rect: [f32; 4], argb: u32) {
    if rect[2] > 0.0 && rect[3] > 0.0 {
        draw.fill(rect, argb);
    }
}

fn outline(draw: &mut dyn DrawContext, rect: [f32; 4], argb: u32, width: f32) {
    fill(draw, [rect[0], rect[1], rect[2], width], argb);
    fill(draw, [rect[0], rect[1] + rect[3] - width, rect[2], width], argb);
    fill(draw, [rect[0], rect[1], width, rect[3]], argb);
    fill(draw, [rect[0] + rect[2] - width, rect[1], width, rect[3]], argb);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condenser;

    #[test]
    fn every_machine_gets_a_screen_with_all_of_its_container_slots() {
        for block in machines::COLLECTORS
            .into_iter()
            .chain(machines::RELAYS)
            .chain([
                condenser::ENERGY_CONDENSER,
                condenser::ENERGY_CONDENSER_MK2,
                machines::ALCHEMICAL_CHEST,
            ])
        {
            let view = screen(block).unwrap_or_else(|| panic!("no screen for {block}"));
            let container = machines::container(block).unwrap();
            assert_eq!(view.slots.len(), container.slots, "{block}");
            assert!(view.slots.iter().all(Option::is_some), "{block}");
            assert!(view.inventory_y > SLOTS_Y, "{block}");
            assert!(view.height > view.inventory_y + 58.0, "{block}");
        }
    }

    #[test]
    fn the_table_is_not_a_machine_screen() {
        assert!(!is_machine_screen(crate::recipes::TABLE));
        assert!(screen(crate::recipes::TABLE).is_none());
    }
}
