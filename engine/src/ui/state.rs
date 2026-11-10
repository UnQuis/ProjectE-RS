//! What the Transmutation Table screen remembers, and what its controls do.
//!
//! This module is the screen's model: the search text, the sort order, the
//! filter, the page, the button states and the list of items shown. It holds
//! no Horizon type and draws nothing, so every rule below is exercised by
//! `cargo test` without a game — which is the only way this screen's behaviour
//! is verified, since the pinned API cannot be built or run here.
//!
//! The screen is a **shell**: it browses a preview list of items and reports
//! what a click would have done, but it never learns an item, never spends
//! EMC and never moves an item. The item slots are real (Horizon's, over the
//! table's own menu) and are handled by `ui::screen`; nothing in this module
//! touches them.

use super::layout::{Control, Layout, PAGE_SIZE, Part};
use super::localization::{self, Language};

/// The preview catalogue the screen browses until the knowledge system is
/// connected.
///
/// It is deliberately its own tiny table rather than `emc.rs`'s, so that
/// removing it later is deleting this constant and the branch that reads it:
/// no EMC value or item id here reaches the gameplay registry, and the screen
/// never claims these are learned items (`localization::KEY_HINT_PLACEHOLDER`
/// says so on screen).
pub(crate) const PREVIEW_ITEMS: [&str; 12] = [
    "minecraft:cobblestone",
    "minecraft:coal",
    "minecraft:iron_ingot",
    "minecraft:gold_ingot",
    "minecraft:glowstone_dust",
    "minecraft:diamond",
    "minecraft:emerald",
    "minecraft:redstone",
    "projecte:philosophers_stone",
    "projecte:alchemical_coal",
    "projecte:mobius_fuel",
    "projecte:aeternalis_fuel",
];

/// How the browser orders its items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Sort {
    /// Cheapest first — the original table's order.
    #[default]
    Emc,
    Name,
}

impl Sort {
    /// The order after clicking the sort control.
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Emc => Self::Name,
            Self::Name => Self::Emc,
        }
    }

    /// The label key this order draws.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Emc => localization::KEY_SORT_EMC,
            Self::Name => localization::KEY_SORT_NAME,
        }
    }
}

/// Which items the browser shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Filter {
    #[default]
    All,
    /// Dark and red matter, and the matter blocks.
    Matter,
    /// The mod's tools and armour.
    Tools,
}

impl Filter {
    /// The filter after clicking the filter control.
    pub(crate) fn next(self) -> Self {
        match self {
            Self::All => Self::Matter,
            Self::Matter => Self::Tools,
            Self::Tools => Self::All,
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::All => localization::KEY_FILTER_ALL,
            Self::Matter => localization::KEY_FILTER_MATTER,
            Self::Tools => localization::KEY_FILTER_TOOLS,
        }
    }

    /// Whether `item` belongs to this filter.
    ///
    /// The rules are name-based on purpose: the preview list is the only input
    /// for now, and a match by path keeps this independent of the item
    /// registry, which the shell does not read.
    pub(crate) fn shows(self, item: &str) -> bool {
        let path = item.rsplit(':').next().unwrap_or(item);
        match self {
            Self::All => true,
            Self::Matter => path.contains("matter"),
            Self::Tools => {
                path.contains("_sword")
                    || path.contains("_pickaxe")
                    || path.contains("_axe")
                    || path.contains("_shovel")
                    || path.contains("_hoe")
                    || path.contains("_helmet")
                    || path.contains("_chestplate")
                    || path.contains("_leggings")
                    || path.contains("_boots")
            }
        }
    }
}

/// A control as the pointer left it: which buttons are held down, and the
/// short confirmation shown after a press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Press {
    /// The button under the pointer that is held down, if any.
    pub held: Option<Part>,
}

/// The item browser: the whole preview list, filtered and sorted, paged.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Browser {
    /// The text typed into the search field.
    pub search: String,
    pub sort: Sort,
    pub filter: Filter,
    /// The page shown, from 0.
    pub page: usize,
}

impl Browser {
    /// The items that pass the filter and the search, in the sort order.
    ///
    /// `emc` answers an item's value for the EMC order. It is passed in rather
    /// than read here so the browser stays independent of the value table; the
    /// screen passes the mod's own `emc::value`.
    pub(crate) fn visible(
        &self,
        all: &[&'static str],
        emc: impl Fn(&str) -> i64,
    ) -> Vec<&'static str> {
        let needle = self.search.trim().to_lowercase();
        let mut items: Vec<&'static str> = all
            .iter()
            .copied()
            .filter(|item| self.filter.shows(item))
            .filter(|item| {
                needle.is_empty()
                    || item.to_lowercase().contains(&needle)
                    || item
                        .rsplit(':')
                        .next()
                        .is_some_and(|path| path.replace('_', " ").contains(&needle))
            })
            .collect();
        match self.sort {
            Sort::Emc => items.sort_by(|a, b| emc(b).cmp(&emc(a)).then_with(|| a.cmp(b))),
            Sort::Name => items.sort_by(|a, b| a.cmp(b)),
        }
        items
    }

    /// How many pages `items` need; at least one, so an empty list still shows
    /// "page 1 / 1" rather than "1 / 0".
    pub(crate) fn pages(items: &[&'static str]) -> usize {
        items.len().div_ceil(PAGE_SIZE).max(1)
    }

    /// The page shown, kept in range as the list shrinks.
    pub(crate) fn page_of(&self, items: &[&'static str]) -> usize {
        self.page.min(Self::pages(items) - 1)
    }

    /// The items of the page shown.
    pub(crate) fn page_items<'a>(&self, items: &'a [&'static str]) -> &'a [&'static str] {
        let page = self.page_of(items);
        let start = page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(items.len());
        items.get(start..end).unwrap_or(&[])
    }

    pub(crate) fn turn(&mut self, items: &[&'static str], forward: bool) {
        let pages = Self::pages(items);
        let page = self.page_of(items);
        self.page = if forward {
            (page + 1).min(pages - 1)
        } else {
            page.saturating_sub(1)
        };
    }

    /// A row was clicked in the browser: nothing is transmuted yet, and the
    /// screen says so. Kept as a method so the wiring point exists.
    pub(crate) fn choose(&mut self, _index: usize) -> Response {
        Response::Notice(localization::KEY_NO_TRANSMUTATION)
    }

    fn type_char(&mut self, c: char) -> Response {
        // A field holds one line of plain text; nothing else is accepted.
        if c.is_control() || self.search.chars().count() >= MAX_SEARCH {
            return Response::None;
        }
        self.search.push(c);
        Response::Changed
    }

    fn backspace(&mut self) -> Response {
        if self.search.pop().is_some() {
            Response::Changed
        } else {
            Response::None
        }
    }

    fn clear(&mut self) -> Response {
        if self.search.is_empty() {
            Response::None
        } else {
            self.search.clear();
            self.page = 0;
            Response::Changed
        }
    }
}

/// The longest search text the field holds, so a held key cannot grow it
/// without end.
pub(crate) const MAX_SEARCH: usize = 32;

/// What a press or a key did, so the screen knows whether to redraw and what
/// to say in the action bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Response {
    /// Nothing changed and nothing to say.
    None,
    /// The visible list changed; redraw it.
    Changed,
    /// A short line to show the player.
    Notice(&'static str),
}

/// The whole screen's state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TableScreen {
    pub language: Language,
    pub browser: Browser,
    pub press: Press,
    /// Whether the search field has the keyboard's focus.
    pub typing: bool,
    /// The last harmless message a control produced, for the action bar.
    pub notice: Option<&'static str>,
    /// The pointer, in window GUI pixels, as of the last mouse event.
    pub mouse: [f32; 2],
    /// The layout of the last frame drawn; the input handlers hit-test
    /// against it, so drawing and clicking always agree on where a part is.
    pub layout: Layout,
}

impl TableScreen {
    /// A screen in `language` for a window of `gui` GUI pixels.
    pub(crate) fn new(language: Language, gui: [f32; 2]) -> Self {
        Self {
            language,
            browser: Browser::default(),
            press: Press::default(),
            typing: false,
            notice: None,
            mouse: [0.0, 0.0],
            layout: Layout::fit(gui),
        }
    }

    /// Re-places the screen in a window of `gui` GUI pixels.
    pub(crate) fn resize(&mut self, gui: [f32; 2]) {
        self.layout = Layout::fit(gui);
    }

    /// A key's text in the screen's language.
    pub(crate) fn text(&self, key: &'static str) -> &'static str {
        localization::text(self.language, key)
    }

    /// The page label, e.g. `Page 1 / 2`, in the screen's language.
    pub(crate) fn page_label(&self, items: &[&'static str]) -> String {
        let page = self.browser.page_of(items).saturating_add(1);
        let pages = Browser::pages(items);
        localization::format(
            self.text(localization::KEY_PAGE),
            &[&page.to_string(), &pages.to_string()],
        )
    }

    /// The item under the pointer, if it is over a cell of the page.
    pub(crate) fn cell_under(&self, items: &[&'static str]) -> Option<&'static str> {
        match self.hit().map(|hit| hit.part) {
            Some(Part::Cell(index)) => self.browser.page_items(items).get(index).copied(),
            _ => None,
        }
    }

    /// What the pointer is over, from the last mouse position and the last
    /// frame's layout.
    pub(crate) fn hit(&self) -> Option<super::layout::Hit> {
        self.layout.hit(self.mouse)
    }

    /// The pointer moved.
    pub(crate) fn moved(&mut self, at: [f32; 2]) -> Response {
        self.mouse = at;
        Response::None
    }

    /// A button went down at `at`: presses the control there and answers what
    /// it did. Where the press lands is part of the answer, because a button
    /// that is held down draws differently.
    pub(crate) fn press(&mut self, at: [f32; 2], items: &[&'static str]) -> Response {
        self.mouse = at;
        let Some(hit) = self.hit() else {
            self.press.held = None;
            self.typing = false;
            return Response::None;
        };
        self.press.held = Some(hit.part);
        match hit.part {
            Part::Search => {
                self.typing = true;
                Response::None
            }
            Part::Cell(index) => {
                self.typing = false;
                self.browser.choose(index)
            }
            Part::Sort => {
                self.typing = false;
                self.browser.sort = self.browser.sort.next();
                Response::Changed
            }
            Part::Filter => {
                self.typing = false;
                self.browser.filter = self.browser.filter.next();
                self.browser.page = 0;
                Response::Changed
            }
            Part::PagePrev => {
                self.typing = false;
                self.browser.turn(items, false);
                Response::Changed
            }
            Part::PageNext => {
                self.typing = false;
                self.browser.turn(items, true);
                Response::Changed
            }
            Part::PageLabel => {
                self.typing = false;
                Response::None
            }
            Part::Slot(_) => {
                // A real slot: the item is the engine's to move, so the screen
                // only gives up the keyboard focus and stays out of the way.
                self.typing = false;
                Response::None
            }
        }
    }

    /// A button came up: releases whatever was held, at `at`.
    pub(crate) fn release(&mut self, at: [f32; 2]) -> Response {
        self.mouse = at;
        self.press.held = None;
        Response::None
    }

    /// The pointer left the window or the screen: nothing stays held, or a
    /// button would draw pressed forever.
    pub(crate) fn focus_lost(&mut self) -> Response {
        self.press.held = None;
        Response::None
    }

    /// A character was typed. Only the search field takes text while it has
    /// the focus; the answer is `Changed` when the text changed, so the
    /// caller knows whether to swallow the character.
    pub(crate) fn typed(&mut self, c: char) -> Response {
        if !self.typing {
            return Response::None;
        }
        self.browser.type_char(c)
    }

    /// A key that is not a character: Backspace edits the field, Enter ends
    /// typing, Escape clears the field before it closes the screen.
    ///
    /// The answer is `true` when the key was the screen's and must not reach
    /// the game or vanilla's own screen handling.
    pub(crate) fn key(&mut self, key: &str) -> bool {
        match key {
            "key.keyboard.backspace" if self.typing => {
                let _ = self.browser.backspace();
                true
            }
            "key.keyboard.enter" | "key.keyboard.keypad.enter" if self.typing => {
                self.typing = false;
                true
            }
            "key.keyboard.escape" if !self.browser.search.is_empty() => {
                let _ = self.browser.clear();
                true
            }
            // Ctrl-combinations are not the field's: the engine reports the
            // modifiers with each key, and `ui::screen` decides there.
            _ => false,
        }
    }

    /// The tooltip for the part under the pointer, in the screen's language.
    pub(crate) fn tooltip(&self, items: &[&'static str]) -> Option<String> {
        let part = self.hit()?.part;
        Some(match part {
            Part::Cell(index) => {
                let item = self.browser.page_items(items).get(index)?;
                item_tooltip(item)
            }
            Part::Search => self.text(localization::KEY_HINT_SEARCH).to_owned(),
            Part::Sort => format!(
                "{} {} ({})",
                self.text(localization::KEY_HINT_SORT),
                self.text(localization::KEY_SORTED_BY),
                self.text(self.browser.sort.key())
            ),
            Part::Filter => format!(
                "{} {}",
                self.text(localization::KEY_HINT_FILTER),
                self.text(self.browser.filter.key())
            ),
            Part::PagePrev | Part::PageNext | Part::PageLabel => {
                self.text(localization::KEY_HINT_PAGE).to_owned()
            }
            Part::Slot(index) if index < super::layout::INPUT_SLOTS.len() => {
                self.text(localization::KEY_HINT_INPUT).to_owned()
            }
            Part::Slot(_) => self.text(localization::KEY_HINT_STORAGE).to_owned(),
        })
    }

    /// Whether the given control is held down, for the pressed drawing.
    pub(crate) fn is_held(&self, part: Part) -> bool {
        self.press.held == Some(part)
    }
}

/// A tooltip for an item id: its name in the player's language where the game
/// can give one, which `ui::screen` supplies, and the id otherwise.
fn item_tooltip(item: &str) -> String {
    item.rsplit(':').next().unwrap_or(item).replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> TableScreen {
        TableScreen::new(Language::English, [640.0, 360.0])
    }

    fn visible(screen: &TableScreen) -> Vec<&'static str> {
        screen.browser.visible(&PREVIEW_ITEMS, |_| 0)
    }

    fn at_part(screen: &TableScreen, part: Part) -> [f32; 2] {
        let rect = match part {
            Part::Cell(index) => Layout::cell(index).unwrap(),
            Part::Search => Layout::control(Control::Search),
            Part::Sort => Layout::control(Control::Sort),
            Part::Filter => Layout::control(Control::Filter),
            Part::PagePrev => Layout::control(Control::PagePrev),
            Part::PageNext => Layout::control(Control::PageNext),
            Part::PageLabel => Layout::control(Control::PageLabel),
            Part::Slot(index) => Layout::slot(index).unwrap(),
        };
        let rect = screen.layout.at(rect);
        [rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0]
    }

    #[test]
    fn the_preview_list_draws_and_is_not_empty() {
        let screen = screen();
        assert!(!visible(&screen).is_empty());
        assert_eq!(Browser::pages(&visible(&screen)), 1);
        assert_eq!(screen.page_label(&visible(&screen)), "Page 1 / 1");
    }

    #[test]
    fn typing_filters_the_list_and_is_visible() {
        let mut screen = screen();
        let items = visible(&screen);
        let before = screen.browser.page_items(&items).len();
        screen.press(at_part(&screen, Part::Search), &items);
        assert!(screen.typing);
        for c in "diamond".chars() {
            assert_eq!(screen.typed(c), Response::Changed);
        }
        assert_eq!(screen.browser.search, "diamond");
        let items = visible(&screen);
        assert_eq!(items, vec!["minecraft:diamond"]);
        assert!(before > 1);
    }

    #[test]
    fn backspace_edits_the_field_and_enter_stops_typing() {
        let mut screen = screen();
        let items = visible(&screen);
        screen.press(at_part(&screen, Part::Search), &items);
        for c in "coal".chars() {
            let _ = screen.typed(c);
        }
        assert_eq!(screen.browser.search, "coal");
        assert!(screen.key("key.keyboard.backspace"));
        assert_eq!(screen.browser.search, "coa");
        assert!(screen.key("key.keyboard.enter"));
        assert!(!screen.typing);
        // With no focus, keys are the game's again.
        assert!(!screen.key("key.keyboard.backspace"));
        assert_eq!(screen.browser.search, "coa");
    }

    #[test]
    fn typing_outside_the_field_does_nothing() {
        let mut screen = screen();
        assert_eq!(screen.typed('x'), Response::None);
        assert_eq!(screen.browser.search, "");
        assert!(!screen.key("key.keyboard.backspace"));
    }

    #[test]
    fn escape_clears_the_search_before_it_closes_the_screen() {
        let mut screen = screen();
        screen.typing = true;
        screen.browser.search = "gold".to_owned();
        assert!(screen.key("key.keyboard.escape"));
        assert_eq!(screen.browser.search, "");
        // The next Escape is not the field's: the game closes the screen.
        assert!(!screen.key("key.keyboard.escape"));
    }

    #[test]
    fn the_search_field_has_a_length_limit() {
        let mut screen = screen();
        screen.typing = true;
        for _ in 0..(MAX_SEARCH + 20) {
            let _ = screen.typed('a');
        }
        assert_eq!(screen.browser.search.chars().count(), MAX_SEARCH);
        // Control characters are not text.
        let mut screen = screen();
        screen.typing = true;
        let _ = screen.typed('\u{7}');
        assert_eq!(screen.browser.search, "");
    }

    #[test]
    fn every_button_answers_a_click() {
        let mut screen = screen();
        let items = visible(&screen);

        // Sort: EMC, then name, each press changing what the button says.
        let sort = at_part(&screen, Part::Sort);
        assert_eq!(screen.press(sort, &items), Response::Changed);
        assert_eq!(screen.browser.sort, Sort::Name);
        assert!(screen.is_held(Part::Sort));
        let _ = screen.release(sort);
        assert!(!screen.is_held(Part::Sort));
        assert_eq!(screen.press(sort, &items), Response::Changed);
        assert_eq!(screen.browser.sort, Sort::Emc);

        // Filter: All, Matter, Tools, All.
        let filter = at_part(&screen, Part::Filter);
        for expected in [Filter::Matter, Filter::Tools, Filter::All] {
            assert_eq!(screen.press(filter, &items), Response::Changed);
            assert_eq!(screen.browser.filter, expected);
            let _ = screen.release(filter);
        }

        // Pages: a press that changes nothing still answers, never silently.
        let next = at_part(&screen, Part::PageNext);
        let prev = at_part(&screen, Part::PagePrev);
        assert_eq!(screen.press(prev, &items), Response::Changed);
        assert_eq!(screen.browser.page, 0);
        assert_eq!(screen.press(next, &items), Response::Changed);
        assert_eq!(screen.browser.page, 0);

        // The page label is a control but has no action.
        let label = at_part(&screen, Part::PageLabel);
        assert_eq!(screen.press(label, &items), Response::None);

        // A cell says that transmutation is not connected: not silence.
        let cell = at_part(&screen, Part::Cell(0));
        assert_eq!(
            screen.press(cell, &items),
            Response::Notice(localization::KEY_NO_TRANSMUTATION)
        );
        assert!(!screen.typing);
    }

    #[test]
    fn the_filter_keeps_only_what_it_names() {
        assert!(Filter::All.shows("minecraft:diamond"));
        assert!(Filter::Matter.shows("projecte:dark_matter"));
        assert!(Filter::Matter.shows("projecte:red_matter_block"));
        assert!(!Filter::Matter.shows("minecraft:diamond"));
        assert!(Filter::Tools.shows("projecte:dark_matter_pickaxe"));
        assert!(Filter::Tools.shows("projecte:red_matter_chestplate"));
        assert!(!Filter::Tools.shows("projecte:dark_matter"));
        // Every preview item is reachable by some filter.
        for item in PREVIEW_ITEMS {
            assert!(
                Filter::All.shows(item),
                "the All filter must never hide {item}"
            );
        }
    }

    #[test]
    fn the_page_turns_only_where_there_is_a_page_and_never_leaves_the_list() {
        // 100 items, so 3 pages.
        let many: Vec<&'static str> = (0..100).map(|_| "minecraft:cobblestone").collect();
        // `page_items` needs a slice of the list's own lifetime; borrow it.
        let many: &[&'static str] = &many;
        let mut screen = screen();
        assert_eq!(Browser::pages(many), 3);
        assert_eq!(screen.browser.page_of(many), 0);
        assert_eq!(screen.browser.page_items(many).len(), PAGE_SIZE);
        for expected in [1, 2, 2] {
            screen.browser.turn(many, true);
            assert_eq!(screen.browser.page_of(many), expected);
        }
        assert_eq!(screen.browser.page_items(many).len(), 100 - 2 * PAGE_SIZE);
        for expected in [1, 0, 0] {
            screen.browser.turn(many, false);
            assert_eq!(screen.browser.page_of(many), expected);
        }
    }

    #[test]
    fn a_filter_that_empties_the_list_still_shows_one_page() {
        let mut screen = screen();
        screen.browser.filter = Filter::Tools;
        let items = visible(&screen);
        assert!(items.is_empty());
        assert_eq!(Browser::pages(&items), 1);
        assert_eq!(screen.page_label(&items), "Page 1 / 1");
        assert_eq!(screen.browser.page_of(&items), 0);
        assert!(screen.browser.page_items(&items).is_empty());
    }

    #[test]
    fn the_page_resets_when_the_list_shrinks_under_it() {
        let mut screen = screen();
        screen.browser.search = "zzz".to_owned();
        let items = visible(&screen);
        screen.browser.page = 7;
        assert_eq!(screen.browser.page_of(&items), 0);
        assert_eq!(screen.page_label(&items), "Page 1 / 1");
    }

    #[test]
    fn a_cell_reports_the_item_under_the_pointer() {
        let mut screen = screen();
        let items = visible(&screen);
        for index in 0..items.len() {
            let at = at_part(&screen, Part::Cell(index));
            let _ = screen.moved(at);
            assert_eq!(screen.cell_under(&items), Some(items[index]));
            assert_eq!(
                screen.tooltip(&items),
                Some(items[index].rsplit(':').next().unwrap().replace('_', " ")),
                "a cell's tooltip names the item"
            );
        }
        let at = at_part(&screen, Part::Search);
        let _ = screen.moved(at);
        assert_eq!(screen.cell_under(&items), None);
        assert_eq!(
            screen.tooltip(&items),
            Some(screen.text(localization::KEY_HINT_SEARCH).to_owned())
        );
    }

    #[test]
    fn every_control_has_a_tooltip() {
        let mut screen = screen();
        let items = visible(&screen);
        for part in [
            Part::Search,
            Part::Sort,
            Part::Filter,
            Part::PagePrev,
            Part::PageNext,
            Part::PageLabel,
        ] {
            let at = at_part(&screen, part);
            let _ = screen.moved(at);
            let tooltip = screen.tooltip(&items).unwrap_or_default();
            assert!(!tooltip.trim().is_empty(), "{part:?} has no tooltip");
        }
    }

    #[test]
    fn a_press_outside_every_part_releases_what_was_held() {
        let mut screen = screen();
        let items = visible(&screen);
        let sort = at_part(&screen, Part::Sort);
        let _ = screen.press(sort, &items);
        assert!(screen.is_held(Part::Sort));
        let _ = screen.press([0.0, 0.0], &items);
        assert!(!screen.is_held(Part::Sort));
        assert!(!screen.typing);
    }

    #[test]
    fn the_screen_shows_its_words_in_the_chosen_language() {
        let mut screen = TableScreen::new(Language::Russian, [640.0, 360.0]);
        assert_eq!(screen.text(localization::KEY_TITLE), "Стол трансмутации");
        let items = visible(&screen);
        assert_eq!(screen.page_label(&items), "Страница 1 / 1");
        let sort = at_part(&screen, Part::Sort);
        let _ = screen.moved(sort);
        assert!(screen.tooltip(&items).unwrap().contains("Сортировка"));
        // The control's own label follows the language too.
        assert_eq!(screen.text(screen.browser.sort.key()), "ЭМК");
        screen.browser.sort = Sort::Name;
        assert_eq!(screen.text(screen.browser.sort.key()), "Название");
    }

    #[test]
    fn a_resize_keeps_the_hit_test_on_what_is_drawn() {
        let mut screen = screen();
        let sort = at_part(&screen, Part::Sort);
        let _ = screen.moved(sort);
        assert_eq!(screen.hit().map(|hit| hit.part), Some(Part::Sort));
        screen.resize([320.0, 240.0]);
        // The old position now points at the shrunken panel's own place, and
        // the hit test agrees with the new layout, not the old one.
        let sort = at_part(&screen, Part::Sort);
        let _ = screen.moved(sort);
        assert_eq!(screen.hit().map(|hit| hit.part), Some(Part::Sort));
    }

    #[test]
    fn losing_focus_does_not_leave_a_button_held() {
        let mut screen = screen();
        let items = visible(&screen);
        let _ = screen.press(at_part(&screen, Part::Sort), &items);
        assert!(screen.is_held(Part::Sort));
        let _ = screen.focus_lost();
        assert!(!screen.is_held(Part::Sort));
    }

    #[test]
    fn sorting_by_name_and_by_emc_are_different_orders() {
        let mut screen = screen();
        let values = |item: &str| match item {
            "minecraft:coal" => 128,
            "minecraft:diamond" => 8192,
            "projecte:philosophers_stone" => 1 << 40,
            _ => 1,
        };
        screen.browser.sort = Sort::Emc;
        let by_emc = screen.browser.visible(&PREVIEW_ITEMS, values);
        assert_eq!(by_emc[0], "projecte:philosophers_stone");
        assert_eq!(by_emc[1], "minecraft:diamond");
        screen.browser.sort = Sort::Name;
        let by_name = screen.browser.visible(&PREVIEW_ITEMS, values);
        let mut expected = by_emc.clone();
        expected.sort();
        assert_eq!(by_name, expected);
        assert_ne!(by_emc, by_name);
    }

    #[test]
    fn a_cell_is_the_screens_and_a_slot_is_the_menus() {
        // This is the rule the click handler turns on: a control or a cell
        // swallows the click (a cell answers with the placeholder notice), a
        // real slot never does, so the engine can move its item.
        let mut screen = screen();
        let items = visible(&screen);
        // A cell: not a slot, and a press leaves the placeholder notice.
        assert!(!Part::Cell(0).is_slot());
        assert!(!Part::Cell(0).is_control());
        let response = screen.press(at_part(&screen, Part::Cell(0)), &items);
        assert_eq!(
            response,
            Response::Notice(localization::KEY_NO_TRANSMUTATION)
        );
        // A real slot: the menu's, and a press changes nothing of the screen's.
        assert!(Part::Slot(0).is_slot());
        assert!(!Part::Slot(0).is_control());
        assert_eq!(
            screen.press(at_part(&screen, Part::Slot(0)), &items),
            Response::None
        );
        assert!(screen.press.held == Some(Part::Slot(0)));
        // A control: the screen's, and swallowed.
        assert!(Part::Search.is_control());
        assert!(!Part::Search.is_slot());
        // No part is both.
        for index in 0..36 {
            assert!(!Part::Cell(index).is_slot());
        }
        for index in 0..super::layout::SLOT_COUNT {
            assert!(!Part::Slot(index).is_control());
        }
    }

    #[test]
    fn every_real_slot_has_a_tooltip() {
        let screen = screen();
        let items = visible(&screen);
        let mut screen = screen;
        for index in 0..super::layout::SLOT_COUNT {
            screen.mouse = at_part(&screen, Part::Slot(index));
            assert!(
                screen.tooltip(&items).is_some(),
                "slot {index} has no tooltip"
            );
        }
    }

    #[test]
    fn the_search_matches_a_path_word_and_ignores_case() {
        let mut screen = screen();
        screen.browser.search = "GOLD".to_owned();
        assert_eq!(
            screen.browser.visible(&PREVIEW_ITEMS, |_| 0),
            vec!["minecraft:gold_ingot"]
        );
        // Underscores read as spaces, so "alchemical coal" finds the fuel.
        screen.browser.search = "alchemical coal".to_owned();
        assert_eq!(
            screen.browser.visible(&PREVIEW_ITEMS, |_| 0),
            vec!["projecte:alchemical_coal"]
        );
        // Surrounding spaces do not matter.
        screen.browser.search = "  fuel  ".to_owned();
        assert_eq!(screen.browser.visible(&PREVIEW_ITEMS, |_| 0).len(), 2);
    }
}
