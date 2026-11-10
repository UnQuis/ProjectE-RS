//! The Transmutation Table's custom screen.
//!
//! The screen is a **menu screen**: the table's block entity declares a
//! nine-slot container with a vanilla single-row chest menu, and this module draws
//! that menu. Doing it that way is what makes the slots real — item pickup,
//! placement, splitting, shift-clicking and the return of whatever is still in
//! them when the screen closes are the game's own menu engine acting on the
//! server, not something this mod imitates. `API_WORKAROUNDS.md` §8 records
//! why the pinned API leaves no other way to have both a custom screen and
//! real slots, and `UI_API.md` lists every Horizon symbol used.
//!
//! What is here:
//!
//! * [`layout`] — where every part goes, and what is under the pointer. Pure
//!   geometry, sized in GUI pixels and centred in whatever window it is given.
//! * [`localization`] — the screen's words, by key, in English and Russian.
//! * [`state`] — the screen's model: the search text, the sort order, the
//!   filter, the page and what each control does. No Horizon type at all, so
//!   its behaviour is covered by `cargo test`.
//! * [`atlas`] — every Horizon call, in one file: the drawing and the menu
//!   layout.
//! * [`screen`] — the hooks' entry points and the one screen state they share.
//!
//! The shell is deliberately not connected to the mod's gameplay: this screen
//! browses a preview list, shows a placeholder balance and answers a click on
//! an item with a note that transmutation is not connected yet. Nothing here
//! learns an item, spends EMC, or changes the progression or alchemy code.

pub(crate) mod atlas;
pub(crate) mod layout;
pub(crate) mod localization;
pub(crate) mod screen;
pub(crate) mod state;

/// This mod's namespace, as the screen's block and translation keys use it.
pub(crate) const MOD_ID: &str = "projecte";

/// The translation key of the screen's title: the table block's own name, so
/// the screen, the item and the server's menu title cannot disagree.
pub(crate) const TITLE_KEY: &str = "block.projecte.transmutation_table";
