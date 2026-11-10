//! Where everything on the Transmutation Table screen goes.
//!
//! This module is pure geometry: it names the screen's parts, their sizes and
//! the rectangles they occupy, and it turns a pointer position into the part
//! under it. It knows nothing about Horizon, drawing or items, so the layout
//! can be reasoned about and checked without a game — and it is checked: the
//! tests below prove the parts never overlap, never leave the panel, and that a
//! click lands on the part drawn there.
//!
//! Every length is in GUI pixels, the unit vanilla screens are laid out in (one
//! GUI pixel is `window / gui_scale` physical pixels). The panel is a fixed
//! 320 x 240 pixel grid and is drawn at 1:1 in those units, because the engine
//! places the menu's real slots the same way (`Layout::fit`); GUI scale is what
//! makes it bigger on a bigger window.
//!
//! # The grid
//!
//! ```text
//!  0        8        100            278      320
//!  +---------------------------------------------+
//!  |            Transmutation Table              |  0..24 header
//!  |  +--------+   +---------------------+      |
//!  |  | emblem |   | search              |      |  28..42 search
//!  |  |        |   +---------------------+      |
//!  |  |        |   | 9 x 4 item grid     |      |  48..126 grid
//!  |  +--------+   +---------------------+      |
//!  |  | EMC    |   |sort|filter|< 1/2 >  |      |  132..146 footer
//!  |  +--------+                                |
//!  |  |[][ ]  |   Inventory                    |  151 label
//!  |  +--------+   [ 9 x 3 main inventory  ]    |  162..220
//!  |               [ hotbar                ]    |  220..238
//!  +---------------------------------------------+
//! ```

/// The table menu's slot count: vanilla's single-row `generic_9x1`.
///
/// Nine slots is what fits beside a 9 x 4 browser inside one 256-wide screen.
/// Every one of them is a **real slot**: a frame placed by `native_screen`, a
/// real container slot on the server, and an item that moves with vanilla's
/// own menu handling and comes back when the screen closes. The browser's
/// cells are deliberately *not* slots (see `super::atlas`'s module note).
pub(crate) const MENU_SLOTS: usize = 9;

/// The menu's two input slots; the remaining seven follow as storage.
pub(crate) const INPUT_SLOTS: [usize; 2] = [0, 1];

/// The storage slots after the inputs. Together with the inputs this is the
/// whole menu, so no slot is hidden and nothing is wasted.
pub(crate) const STORAGE_SLOTS: usize = MENU_SLOTS - INPUT_SLOTS.len();

/// Every real slot the screen shows: the inputs and the storage.
pub(crate) const SLOT_COUNT: usize = MENU_SLOTS;

/// The panel's size, the one every part below is placed inside.
pub(crate) const PANEL_W: f32 = 320.0;
pub(crate) const PANEL_H: f32 = 260.0;

/// A one-slot frame is a vanilla 18 x 18 rounded frame around a 16 x 16 icon.
pub(crate) const SLOT: f32 = 18.0;
/// GUI pixels between two slot frames in a row or column.
pub(crate) const SLOT_GAP: f32 = 2.0;

/// The item grid: 9 columns by 4 rows, 36 items a page.
pub(crate) const GRID_COLS: usize = 9;
pub(crate) const GRID_ROWS: usize = 4;
pub(crate) const PAGE_SIZE: usize = GRID_COLS * GRID_ROWS;

// The left column, in panel coordinates.
const LEFT_X: f32 = 8.0;
const PLATE_X: f32 = 4.0;
const PLATE_Y: f32 = 24.0;
const PLATE_W: f32 = 92.0;
const PLATE_H: f32 = 136.0;
const EMBLEM_Y: f32 = 26.0;
const EMBLEM_SIZE: f32 = 76.0;
const EMC_LABEL_Y: f32 = 104.0;
const EMC_BAR_Y: f32 = 114.0;
const EMC_BAR_H: f32 = 12.0;

// The right column, in panel coordinates: the grid is 178 wide, and the search
// field and the footer line up with it.
const RIGHT_X: f32 = 104.0;
const RIGHT_W: f32 = GRID_COLS as f32 * (SLOT + SLOT_GAP) - SLOT_GAP;
const TITLE_Y: f32 = 8.0;
const SEARCH_Y: f32 = 28.0;
const SEARCH_H: f32 = 14.0;
const GRID_Y: f32 = 48.0;
/// The real slots' first row, below the browser's last.
const SLOTS_Y: f32 = 148.0;
const FOOTER_Y: f32 = 132.0;
const BUTTON_H: f32 = 14.0;

/// The player inventory's first main slot. Vanilla puts the hotbar 58 pixels
/// below it; the last hotbar row ends at `220 + 18 = 238`, inside the panel.
const INVENTORY_Y: f32 = 172.0;
/// The inventory's label sits one text line above its first row.
const INVENTORY_LABEL_Y: f32 = 161.0;
/// The hotbar's distance below the main rows, as `Game::slot_position` puts it:
/// 3 rows of 18 plus 4. The last hotbar row ends at `220 + 18 = 238`, inside
/// the panel.
const HOTBAR_OFFSET: f32 = 58.0;

/// The parts of the screen a pointer can be over.
///
/// The grid is one part rather than `PAGE_SIZE` parts: the cell under the
/// pointer is worked out from the index, which makes "the cell at index n" a
/// single source of truth for drawing and clicking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// A cell of the item browser; the payload is its index on the page.
    Cell(usize),
    /// The search field, which takes typed characters.
    Search,
    /// The sort control: EMC order, then name order.
    Sort,
    /// The filter control: cycles which kinds of item are shown.
    Filter,
    /// The previous page.
    PagePrev,
    /// The next page.
    PageNext,
    /// The page indicator: shows `current / total`.
    PageLabel,
    /// A real menu slot: the payload is its index, the inputs first. Its
    /// clicks are the menu's, not the screen's.
    Slot(usize),
}

/// Every control that is always at a fixed place. The item grid is handled by
/// [`Layout::cell`] and [`Layout::hit`] instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    Search,
    Sort,
    Filter,
    PagePrev,
    PageNext,
    PageLabel,
}

impl Part {
    /// Whether this part is one of the screen's own **controls** rather than a
    /// real menu slot.
    ///
    /// It decides which clicks the screen swallows (`ui::screen::mouse_pressed`):
    /// a control's click is cancelled so it cannot also reach the menu, while a
    /// browser cell is a real slot and the engine's menu handling must see it.
    pub(crate) fn is_control(self) -> bool {
        !matches!(self, Part::Cell(_) | Part::Slot(_))
    }

    /// Whether this part is a real menu slot, whose item the engine moves.
    pub(crate) fn is_slot(self) -> bool {
        matches!(self, Part::Slot(_))
    }
}

impl Control {
    /// Every fixed control, in drawing order.
    pub(crate) const ALL: [Control; 6] = [
        Control::Search,
        Control::Sort,
        Control::Filter,
        Control::PagePrev,
        Control::PageLabel,
        Control::PageNext,
    ];

    /// The part this control is.
    pub(crate) fn part(self) -> Part {
        match self {
            Control::Search => Part::Search,
            Control::Sort => Part::Sort,
            Control::Filter => Part::Filter,
            Control::PagePrev => Part::PagePrev,
            Control::PageNext => Part::PageNext,
            Control::PageLabel => Part::PageLabel,
        }
    }
}

/// The part a pointer is over, with the frame the part was drawn in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hit {
    pub part: Part,
    pub rect: Rect,
}

/// A rectangle in GUI pixels: `[x, y, width, height]`.
pub(crate) type Rect = [f32; 4];

/// The screen as placed in one frame: the window's GUI size, the top-left
/// corner the panel was centred on, and the scale the panel is drawn at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layout {
    /// The GUI size of the window.
    pub gui: [f32; 2],
    /// The panel's top-left corner in window GUI pixels.
    pub origin: [f32; 2],
    /// Always 1: the panel is drawn at 1:1 in GUI pixels, as the engine draws
    /// the menu's slots (see [`Layout::fit`]).
    pub scale: f32,
}

impl Layout {
    /// Places the panel in a window of `gui` GUI pixels.
    ///
    /// This is the engine's own placement, not the screen's: the panel is put
    /// at `HudBatch::centered_origin(width, height)`
    /// (`((gui - size) / 2).trunc()`) and drawn at **1:1**, whatever the window
    /// size. That is not a simplification — the engine places the menu's slots
    /// from `NativeScreen.slots` through the same `inventory_origin`, so a
    /// screen that scaled its own drawing would draw its frames somewhere the
    /// engine's slots are not, and every click would land off by the scale.
    ///
    /// `scale` therefore stays 1, and the panel is exactly
    /// [`PANEL_W`] x [`PANEL_H`]. Minecraft's own scaling is what makes the
    /// panel bigger on a bigger window: GUI scale multiplies the whole
    /// coordinate space, so the panel is drawn in more device pixels without
    /// any pixel in its own coordinates changing. A window whose GUI space is
    /// smaller than the panel clips it, exactly as a vanilla chest screen does
    /// at GUI scale 4 on a small window.
    pub(crate) fn fit(gui: [f32; 2]) -> Self {
        Self {
            gui,
            origin: [
                ((gui[0] - PANEL_W) / 2.0).trunc(),
                ((gui[1] - PANEL_H) / 2.0).trunc(),
            ],
            scale: 1.0,
        }
    }

    /// The panel's rectangle in window GUI pixels.
    pub(crate) fn panel(&self) -> Rect {
        [
            self.origin[0],
            self.origin[1],
            PANEL_W * self.scale,
            PANEL_H * self.scale,
        ]
    }

    /// A rectangle given in panel coordinates, moved and scaled into window
    /// GUI pixels.
    pub(crate) fn at(&self, rect: Rect) -> Rect {
        [
            self.origin[0] + rect[0] * self.scale,
            self.origin[1] + rect[1] * self.scale,
            rect[2] * self.scale,
            rect[3] * self.scale,
        ]
    }

    /// A point in panel coordinates, in window GUI pixels.
    pub(crate) fn point(&self, at: [f32; 2]) -> [f32; 2] {
        [
            self.origin[0] + at[0] * self.scale,
            self.origin[1] + at[1] * self.scale,
        ]
    }

    /// A length in panel pixels, in window GUI pixels.
    pub(crate) fn len(&self, length: f32) -> f32 {
        length * self.scale
    }

    /// The item grid's frame in panel coordinates.
    pub(crate) fn grid() -> Rect {
        [
            RIGHT_X,
            GRID_Y,
            RIGHT_W,
            GRID_ROWS as f32 * (SLOT + SLOT_GAP) - SLOT_GAP,
        ]
    }

    /// The frame of the cell at `index` on the page, in panel coordinates.
    /// `None` past the last cell.
    pub(crate) fn cell(index: usize) -> Option<Rect> {
        if index >= PAGE_SIZE {
            return None;
        }
        let col = index % GRID_COLS;
        let row = index / GRID_COLS;
        Some([
            RIGHT_X + col as f32 * (SLOT + SLOT_GAP),
            GRID_Y + row as f32 * (SLOT + SLOT_GAP),
            SLOT,
            SLOT,
        ])
    }

    /// The frame of real menu slot `index` (0 is the first input, the storage
    /// follows), in panel coordinates.
    ///
    /// These slots — and only these, plus the player's own — are drawn by the
    /// engine as real slots at these coordinates, and are the ones an item can
    /// be put in. `None` past the last one.
    pub(crate) fn slot(index: usize) -> Option<Rect> {
        let total = SLOT_COUNT;
        if index >= total {
            return None;
        }
        let col = index % GRID_COLS;
        let row = index / GRID_COLS;
        Some([
            RIGHT_X + col as f32 * (SLOT + SLOT_GAP),
            SLOTS_Y + row as f32 * (SLOT + SLOT_GAP),
            SLOT,
            SLOT,
        ])
    }

    /// The preview notice's line: under the hotbar, on the panel's last rows,
    /// so it is never over a slot. The hotbar ends at 248 and the panel at 260,
    /// so 250 leaves a text line clear of both.
    pub(crate) fn notice_line() -> [f32; 2] {
        [LEFT_X, PANEL_H - 10.0]
    }

    /// The real slots' label, one text line above the first row.
    pub(crate) fn slots_label() -> [f32; 2] {
        [RIGHT_X, SLOTS_Y - 10.0]
    }

    /// The real slot area's frame in panel coordinates, for the plate behind
    /// the slots and the label above them.
    pub(crate) fn slots_area() -> Rect {
        let total = SLOT_COUNT;
        let rows = total.div_ceil(GRID_COLS) as f32;
        [
            RIGHT_X,
            SLOTS_Y,
            RIGHT_W,
            rows * (SLOT + SLOT_GAP) - SLOT_GAP,
        ]
    }

    /// A control's frame in panel coordinates.
    pub(crate) fn control(control: Control) -> Rect {
        match control {
            Control::Search => [RIGHT_X, SEARCH_Y, RIGHT_W, SEARCH_H],
            Control::Sort => [RIGHT_X, FOOTER_Y, 48.0, BUTTON_H],
            Control::Filter => [RIGHT_X + 52.0, FOOTER_Y, 46.0, BUTTON_H],
            Control::PagePrev => [RIGHT_X + 102.0, FOOTER_Y, 18.0, BUTTON_H],
            Control::PageLabel => [RIGHT_X + 124.0, FOOTER_Y, 32.0, BUTTON_H],
            Control::PageNext => [RIGHT_X + 160.0, FOOTER_Y, 18.0, BUTTON_H],
        }
    }

    /// The left column's regions, in panel coordinates.
    pub(crate) fn plate() -> Rect {
        [PLATE_X, PLATE_Y, PLATE_W, PLATE_H]
    }

    pub(crate) fn emblem() -> Rect {
        [LEFT_X, EMBLEM_Y, EMBLEM_SIZE, EMBLEM_SIZE]
    }

    pub(crate) fn emc_label() -> [f32; 2] {
        [LEFT_X, EMC_LABEL_Y]
    }

    pub(crate) fn emc_bar() -> Rect {
        [LEFT_X, EMC_BAR_Y, EMBLEM_SIZE, EMC_BAR_H]
    }

    /// Where the player's inventory block starts in panel coordinates: its
    /// first main slot column. `native_screen` lays the inventory out from
    /// here, so the drawing code must not place those slots itself.
    pub(crate) fn inventory_origin() -> [f32; 2] {
        [LEFT_X, INVENTORY_Y]
    }

    /// The player's slot `index` (0 is the hotbar's first, 9 the main rows'
    /// first, as the engine numbers them), in panel coordinates.
    ///
    /// The engine places these slots itself from `NativeScreen.inventory_x`
    /// and `inventory_y` — 18 pixels apart, the hotbar 58 below the main rows —
    /// and with `own_background` the screen also draws their frames, so these
    /// numbers have to be the engine's, not the grid's 20-pixel pitch.
    pub(crate) fn inventory_slot(index: usize) -> Option<Rect> {
        if index >= 36 {
            return None;
        }
        // `Game::slot_position`: slots 0..9 are the hotbar at +58, 9..36 the
        // three main rows from +0.
        let (col, y) = if index < 9 {
            (index, INVENTORY_Y + HOTBAR_OFFSET)
        } else {
            (
                (index - 9) % 9,
                INVENTORY_Y + ((index - 9) / 9) as f32 * SLOT,
            )
        };
        Some([LEFT_X + col as f32 * SLOT, y, SLOT, SLOT])
    }

    pub(crate) fn inventory_label() -> [f32; 2] {
        [LEFT_X, INVENTORY_LABEL_Y]
    }

    /// The title's position: centred across the panel, in the header band.
    pub(crate) fn title_y() -> f32 {
        TITLE_Y
    }

    /// The part under `at` (window GUI pixels), or `None` over nothing.
    ///
    /// Cells are asked first so that a cell overlapping a control would still
    /// win, then the fixed controls.
    pub(crate) fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        if !contains(self.panel(), at) {
            return None;
        }
        for index in 0..PAGE_SIZE {
            let rect = self.at(Self::cell(index)?);
            if contains(rect, at) {
                return Some(Hit {
                    part: Part::Cell(index),
                    rect,
                });
            }
        }
        for index in 0..SLOT_COUNT {
            let Some(rect) = Self::slot(index) else {
                continue;
            };
            let rect = self.at(rect);
            if contains(rect, at) {
                return Some(Hit {
                    part: Part::Slot(index),
                    rect,
                });
            }
        }
        for control in Control::ALL {
            let rect = self.at(Self::control(control));
            if contains(rect, at) {
                return Some(Hit {
                    part: control.part(),
                    rect,
                });
            }
        }
        None
    }
}

/// Whether `rect` contains `at`; the right and bottom edges are outside, as
/// vanilla's `Screen.isHovering` treats them.
pub(crate) fn contains(rect: Rect, at: [f32; 2]) -> bool {
    at[0] >= rect[0] && at[0] < rect[0] + rect[2] && at[1] >= rect[1] && at[1] < rect[1] + rect[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every part that is drawn at a fixed place, for the geometry checks.
    fn parts() -> Vec<Rect> {
        let mut rects = vec![
            Layout::plate(),
            Layout::emblem(),
            Layout::emc_bar(),
            Layout::grid(),
        ];
        rects.extend(Control::ALL.map(Layout::control));
        rects.extend((0..SLOT_COUNT).filter_map(Layout::slot));
        rects.push(Layout::slots_area());
        rects.extend((0..PAGE_SIZE).filter_map(Layout::cell));
        // The inventory block: its 27 main slots then the hotbar's 9, 58 below.
        let inventory = Layout::inventory_origin();
        rects.push([
            inventory[0],
            inventory[1],
            9.0 * (SLOT + SLOT_GAP) - SLOT_GAP,
            3.0 * (SLOT + SLOT_GAP) - SLOT_GAP,
        ]);
        rects.push([
            inventory[0],
            inventory[1] + 58.0,
            9.0 * (SLOT + SLOT_GAP) - SLOT_GAP,
            SLOT,
        ]);
        rects
    }

    #[test]
    fn the_panel_is_centred_and_whole_in_a_normal_window() {
        // 1920 x 1080 at GUI scale 3 is 640 x 360.
        let layout = Layout::fit([640.0, 360.0]);
        assert_eq!(layout.scale, 1.0);
        assert_eq!(layout.panel(), [160.0, 60.0, 320.0, 240.0]);
        assert!(layout.panel()[0] >= 0.0 && layout.panel()[1] >= 0.0);
    }

    #[test]
    fn the_panel_is_never_rescaled_because_the_engines_slots_are_not() {
        // The engine draws the menu at `centered_origin(320, 240)` and places
        // the slots at 1:1 in the same units. If this screen scaled itself,
        // its frames and the engine's slots would part company, so every
        // window size must give scale 1 and the same panel size.
        for gui in [
            [640.0, 360.0],
            [320.0, 240.0],
            [300.0, 240.0],
            [260.0, 200.0],
            [180.0, 130.0],
            [100.0, 80.0],
            [1920.0, 1080.0],
        ] {
            let layout = Layout::fit(gui);
            assert_eq!(layout.scale, 1.0, "{gui:?}");
            assert_eq!(layout.panel()[2], PANEL_W, "{gui:?}");
            assert_eq!(layout.panel()[3], PANEL_H, "{gui:?}");
            // Centred, and truncating like `centered_origin` rather than
            // rounding: a half pixel would land between device pixels.
            assert_eq!(layout.origin[0], ((gui[0] - PANEL_W) / 2.0).trunc());
            assert_eq!(layout.origin[1], ((gui[1] - PANEL_H) / 2.0).trunc());
        }
        // A GUI space smaller than the panel puts it at a negative corner,
        // which clips it exactly as vanilla clips a chest screen; the slots
        // still answer where they are drawn.
        let tiny = Layout::fit([100.0, 80.0]);
        assert!(tiny.origin[0] < 0.0 && tiny.origin[1] < 0.0);
    }

    #[test]
    fn every_part_stays_inside_the_panel() {
        for rect in parts() {
            assert!(
                rect[0] >= 0.0 && rect[1] >= 0.0,
                "{rect:?} starts at a negative coordinate"
            );
            assert!(
                rect[0] + rect[2] <= PANEL_W && rect[1] + rect[3] <= PANEL_H,
                "{rect:?} runs past the panel"
            );
        }
    }

    #[test]
    fn the_hotbar_still_fits_below_the_panel() {
        let inventory = Layout::inventory_origin();
        let hotbar_bottom = inventory[1] + 58.0 + SLOT;
        assert!(
            hotbar_bottom <= PANEL_H,
            "the hotbar ends at {hotbar_bottom}"
        );
        assert!(hotbar_bottom > PANEL_H - 8.0, "the hotbar leaves a margin");
    }

    #[test]
    fn the_parts_that_hold_items_do_not_overlap() {
        // The plate, the emblem and the emc bar are decorations drawn behind
        // one another; everything that holds or reacts to the pointer must be
        // disjoint, or a click would be ambiguous.
        let mut rects: Vec<(String, Rect)> = vec![("grid".to_owned(), Layout::grid())];
        for control in Control::ALL {
            rects.push((format!("{control:?}"), Layout::control(control)));
        }
        for index in 0..SLOT_COUNT {
            rects.push((format!("slot {index}"), Layout::slot(index).unwrap()));
        }
        // The inventory block and its hotbar.
        let inventory = Layout::inventory_origin();
        let column = 9.0 * (SLOT + SLOT_GAP) - SLOT_GAP;
        rects.push((
            "inventory".to_owned(),
            [
                inventory[0],
                inventory[1],
                column,
                3.0 * (SLOT + SLOT_GAP) - SLOT_GAP,
            ],
        ));
        rects.push((
            "hotbar".to_owned(),
            [inventory[0], inventory[1] + 58.0, column, SLOT],
        ));
        for (i, (a_name, a)) in rects.iter().enumerate() {
            for (b_name, b) in rects.iter().skip(i + 1) {
                assert!(!overlaps(*a, *b), "{a_name} {a:?} overlaps {b_name} {b:?}");
            }
        }
    }

    #[test]
    fn the_grid_is_full_and_the_page_holds_every_cell() {
        assert_eq!(PAGE_SIZE, 36);
        assert!(Layout::cell(PAGE_SIZE - 1).is_some());
        assert!(Layout::cell(PAGE_SIZE).is_none());
        // Nine columns of 18 with 2 between them: 178 wide; four rows: 78.
        assert_eq!(&Layout::grid()[2..], &[178.0, 78.0]);
        // The footer starts below the grid and ends above the inventory.
        let footer_bottom = Layout::control(Control::Sort)[1] + BUTTON_H;
        assert!(footer_bottom > Layout::grid()[1] + Layout::grid()[3]);
        assert!(footer_bottom < Layout::inventory_origin()[1]);
    }

    #[test]
    fn the_search_field_and_the_footer_line_up_with_the_grid() {
        let grid = Layout::grid();
        for control in Control::ALL {
            let rect = Layout::control(control);
            assert!(rect[0] >= grid[0], "{control:?} starts left of the grid");
            assert!(
                rect[0] + rect[2] <= grid[0] + grid[2] + 0.001,
                "{control:?} runs past the grid's right edge"
            );
        }
    }

    #[test]
    fn a_hit_finds_the_cell_and_the_control_under_the_pointer() {
        let layout = Layout::fit([640.0, 360.0]);
        for index in [0, 5, 8, 9, 35] {
            let rect = layout.at(Layout::cell(index).unwrap());
            let at = [rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0];
            assert_eq!(layout.hit(at).map(|h| h.part), Some(Part::Cell(index)));
        }
        for control in Control::ALL {
            let rect = layout.at(Layout::control(control));
            let at = [rect[0] + 1.0, rect[1] + 1.0];
            assert_eq!(layout.hit(at).map(|h| h.part), Some(control.part()));
        }
        // Outside the panel: nothing.
        assert!(layout.hit([0.0, 0.0]).is_none());
        // Inside the panel but over no part: nothing.
        assert!(layout.hit(layout.point([96.0, 232.0])).is_none());
    }

    #[test]
    fn a_hit_matches_what_was_drawn_in_a_window_of_any_size() {
        // A hit test and the drawing share `at`, so a pointer put at the
        // middle of the rectangle that was drawn finds that part, whatever the
        // window: the panel is not rescaled, only moved.
        for gui in [[640.0, 360.0], [200.0, 160.0], [1920.0, 1080.0]] {
            let layout = Layout::fit(gui);
            for part in [Part::Cell(0), Part::Cell(17), Part::Search, Part::PageNext] {
                let rect = layout.at(match part {
                    Part::Cell(index) => Layout::cell(index).unwrap(),
                    Part::Search => Layout::control(Control::Search),
                    Part::PageNext => Layout::control(Control::PageNext),
                    _ => unreachable!(),
                });
                let inside = [rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5];
                assert!(contains(rect, inside), "{gui:?} {part:?}");
                assert_eq!(
                    layout.hit(inside).map(|hit| hit.part),
                    Some(part),
                    "{gui:?}"
                );
            }
        }
    }

    #[test]
    fn the_player_slots_are_where_the_engine_puts_them() {
        // `Game::slot_position`: the hotbar at `inventory_y + 58`, the main
        // rows from `inventory_y`, 18 apart, from `inventory_x`.
        for index in 0..36 {
            let rect = Layout::inventory_slot(index).expect("36 slots");
            let (col, y) = if index < 9 {
                (index, INVENTORY_Y + 58.0)
            } else {
                (
                    (index - 9) % 9,
                    INVENTORY_Y + ((index - 9) / 9) as f32 * 18.0,
                )
            };
            assert_eq!(rect[0], LEFT_X + col as f32 * 18.0);
            assert_eq!(rect[1], y);
            assert_eq!((rect[2], rect[3]), (18.0, 18.0));
        }
        assert!(Layout::inventory_slot(36).is_none());
        // The hotbar's last row ends inside the panel.
        let last = Layout::inventory_slot(8).unwrap();
        assert!(last[1] + last[3] <= PANEL_H);
    }

    #[test]
    fn the_player_slots_do_not_overlap_the_browser_or_the_inputs() {
        let grid = Layout::grid();
        for index in 0..36 {
            let rect = Layout::inventory_slot(index).unwrap();
            assert!(!overlaps(rect, grid), "player slot {index} covers the grid");
            for i in 0..2 {
                assert!(
                    !overlaps(rect, Layout::slot(i).unwrap()),
                    "player slot {index} covers real slot {i}"
                );
            }
        }
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3]
    }
}
