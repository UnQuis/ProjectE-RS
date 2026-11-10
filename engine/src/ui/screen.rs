//! The Transmutation Table screen's lifecycle and its input.
//!
//! This is the only place that registers the screen's hooks. It holds one slot
//! of screen state for the *local* player (the window's own), which is all a
//! client-side screen needs: the menu and the items in it are the server's and
//! are never stored here.
//!
//! # What each hook is for
//!
//! | hook | API | what it does here |
//! |---|---|---|
//! | `screen` | 19 | lays the table's nine-slot menu out |
//! | `screen_draw` | 19 | draws the panel, its parts and the browser |
//! | `screen_tooltip` | 20 | names the item or control under the pointer |
//! | `screen_mouse_button_pressed_pre` | 80 | presses a control, focuses the search field |
//! | `screen_mouse_button_released_pre` | 80 | releases it |
//! | `screen_char_typed_pre` | 104 | types into the search field |
//! | `screen_key_pressed_pre` | 104 | Backspace, Enter, Escape |
//! | `screen_opening` | 367 | starts the state when the table's menu opens |
//! | `screen_closing` | 367 | drops the state, so a reopened table starts clean |
//!
//! The menu's slots are not handled here at all: vanilla's menu engine moves
//! their items on the server, and on the client the engine draws the stacks it
//! was sent. That is the whole reason this screen is a menu screen — see
//! `atlas.rs`.

use std::sync::Mutex;

use horizon_client::mod_api::{Flow, Host, HudBatch, NativeScreen, NativeScreenView, Text};

use super::atlas;
use super::layout;
use super::localization::Language;
use super::state::{Response, TableScreen};

/// The client's screen state, or `None` while the table is closed.
static STATE: Mutex<Option<TableScreen>> = Mutex::new(None);

/// Runs `f` on the screen state, or answers `default` when the table is not
/// open.
///
/// A panic in a previous hook cannot poison the screen for good: a poisoned
/// lock's contents are recovered, so a failed frame does not leave the table
/// unopenable until the game restarts.
fn with_state<R>(default: R, f: impl FnOnce(&mut TableScreen) -> R) -> R {
    let mut guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    match guard.as_mut() {
        Some(state) => f(state),
        None => default,
    }
}

/// The language the screen draws in.
///
/// The pinned Host has no "current language" helper, but the game does have
/// its own phrase for the inventory label — the one string on this screen that
/// is vanilla's. Asking for that phrase and matching the languages this mod
/// ships is how the screen follows the player's language without a new API
/// (`API_WORKAROUNDS.md` §9).
fn language(_host: &dyn Host) -> Language {
    Language::of(&client_locale())
}

fn client_locale() -> String {
    let inventory = Text::translate_with_fallback("container.inventory", "Inventory", &[]);
    let phrase = inventory.plain();
    match phrase.as_str() {
        "Инвентарь" => "ru_ru".to_owned(),
        _ => "en_us".to_owned(),
    }
}

/// `native_screen` (API 19): lay out the table's menu when the server opens it
/// for this block.
///
/// The engine asks on the client *and* on the server, because a slot left out
/// is not in the menu on either side. The answer depends only on the block id
/// and never on the client's window size, so both sides agree: the server
/// places the same 45 slots the client draws.
pub(crate) fn screen(block: &str) -> Option<NativeScreen> {
    if block != crate::recipes::TABLE {
        return None;
    }
    Some(atlas::native_screen())
}

/// `native_screen_draw` (API 19): draw the table's screen over its menu.
pub(crate) fn screen_draw(
    host: &dyn Host,
    batch: &mut HudBatch,
    block: &str,
    _data: &[i32],
    origin: [f32; 2],
) {
    if block != crate::recipes::TABLE {
        return;
    }
    with_state((), |state| atlas::draw(host, batch, state, origin));
}

/// `native_screen_tooltip` (API 20): the tooltip for what is under the pointer.
///
/// A `collect` hook: every mod adds its lines to the one list the engine
/// draws, so nothing is returned.
pub(crate) fn screen_tooltip(
    _host: &dyn Host,
    block: &str,
    _data: &[i32],
    at: [f32; 2],
    lines: &mut Vec<String>,
) {
    if block != crate::recipes::TABLE {
        return;
    }
    with_state((), |state| {
        let _ = state.moved(at);
        let items = atlas::browse_items(state);
        atlas::tooltip(state, &items, lines);
    });
}

/// The window's GUI size, or the panel's own when the game cannot say.
fn gui_size(host: &dyn Host) -> [f32; 2] {
    atlas::gui_size(host).unwrap_or([layout::PANEL_W, layout::PANEL_H])
}

/// `native_screen_opening` (API 367): a screen opened. Starts the table's state
/// when it is the table's menu screen.
pub(crate) fn screen_opening(host: &dyn Host, view: &NativeScreenView<'_>) {
    if !atlas::is_table(view, host) {
        return;
    }
    let gui = gui_size(host);
    let language = language(host);
    let mut guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    match guard.as_mut() {
        // Open again for a changed window: keep what was typed, move the panel
        // to the new window's middle.
        Some(state) => {
            state.resize(gui);
            state.language = language;
        }
        None => *guard = Some(TableScreen::new(language, gui)),
    }
}

/// `native_screen_closing` (API 365): the screen closed. Drops the state, so
/// opening the table again does not show the last session's search.
pub(crate) fn screen_closing(host: &dyn Host, view: &NativeScreenView<'_>) {
    if !atlas::is_table(view, host) {
        return;
    }
    reset();
}

/// `native_screen_mouse_button_pressed_pre` (API 80): a click on the screen.
///
/// The engine fires this before anything reads the click, and a cancelled
/// click goes no further — "no slot, no button, no drag"
/// (`game.rs`, the `MouseInput` arm). So what is cancelled matters:
///
/// * A click on one of this screen's **controls** (the search field, the sort
///   and filter buttons, the pager) is cancelled, so the same click cannot
///   also reach whatever the menu has beneath a button.
/// * A click on a **real slot** — one of the menu's nine, or one of the
///   player's own — is *not* cancelled: the engine's menu handling (pick up,
///   place, split, swap, shift-click) must see it, and cancelling it would
///   make every slot decoration.
/// * A click on a **browser cell** is not cancelled either, but it is no slot:
///   the screen answers it with the placeholder notice
///   (`localization::KEY_NO_TRANSMUTATION`) and nothing moves. There is no
///   slot under a cell, so the engine does nothing more with the click.
/// * A click on empty panel space is not cancelled; no slot is under it, so
///   vanilla's menu does nothing with it anyway.
///
/// The screen's bookkeeping (which part the pointer is over, for the hovered
/// frame and the tooltip, and the notice a click on a cell leaves) runs on
/// every press, whichever branch it takes.
pub(crate) fn mouse_pressed(
    host: &dyn Host,
    view: &NativeScreenView<'_>,
    _button: i32,
    _modifiers: u32,
) -> Flow {
    if !atlas::is_table(view, host) {
        return Flow::Continue;
    }
    let mouse = view.mouse;
    let cancel = with_state(false, |state| {
        let _ = state.moved(mouse);
        let items = atlas::browse_items(state);
        let response = state.press(mouse, &items);
        if let Response::Notice(key) = response {
            state.notice = Some(key);
        }
        // Only the screen's own controls swallow the click; a cell or a
        // player slot is the menu's, and the engine must act on it.
        state.hit().is_some_and(|hit| hit.part.is_control())
    });
    if cancel { Flow::Cancel } else { Flow::Continue }
}

/// `native_screen_mouse_button_released_pre` (API 80): the button came up.
pub(crate) fn mouse_released(
    host: &dyn Host,
    view: &NativeScreenView<'_>,
    _button: i32,
    _modifiers: u32,
) -> Flow {
    if !atlas::is_table(view, host) {
        return Flow::Continue;
    }
    let mouse = view.mouse;
    with_state((), |state| {
        let _ = state.release(mouse);
    });
    // The release is not cancelled: the engine's own bookkeeping (a drag it
    // ended, a button it tracks) must still see it.
    Flow::Continue
}

/// `native_screen_char_typed_pre` (API 104): a character typed at the screen.
///
/// The search field is drawn by this screen, so the character is cancelled once
/// it has been taken. While the field does not have the focus every character
/// is left to the game.
pub(crate) fn char_typed(
    host: &dyn Host,
    view: &NativeScreenView<'_>,
    c: char,
    _modifiers: u32,
) -> Flow {
    if !atlas::is_table(view, host) {
        return Flow::Continue;
    }
    let taken = with_state(false, |state| {
        let response = state.typed(c);
        matches!(response, Response::Changed) || state.typing
    });
    if taken { Flow::Cancel } else { Flow::Continue }
}

/// `native_screen_key_pressed_pre` (API 104): a named key on the screen.
///
/// Backspace edits the field, Enter ends typing and Escape clears the field
/// before it closes the screen (`state.key`). Everything else is the game's,
/// Escape on an empty field included, so the screen closes the way a player
/// expects.
pub(crate) fn key_pressed(
    host: &dyn Host,
    view: &NativeScreenView<'_>,
    key: &str,
    _repeat: bool,
    _modifiers: u32,
) -> Flow {
    if !atlas::is_table(view, host) {
        return Flow::Continue;
    }
    let handled = with_state(false, |state| state.key(key));
    if handled {
        Flow::Cancel
    } else {
        Flow::Continue
    }
}

/// Whether the table's screen is open.
pub(crate) fn is_open() -> bool {
    let guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.is_some()
}

/// Forgets the screen state. Called when the table closes and when the player
/// leaves the world, so a screen that never reported closing cannot leak into
/// the next one.
pub(crate) fn reset() {
    let mut guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = None;
}

/// The notice a control left, if any, taking it.
///
/// The screen draws its notice itself (`atlas::draw`), so nothing in
/// production calls this; it exists so the state behind that line can be
/// asserted. The screen hooks are all `&self` at the pinned revision, so the
/// chat and action-bar helpers, which need `&mut dyn Host`, are out of reach
/// from them (`API_WORKAROUNDS.md` §9).
#[cfg(test)]
fn take_notice() -> Option<&'static str> {
    with_state(None, |state| state.notice.take())
}

/// Installs a state, for the tests below and for `screen_opening`. Kept private
/// to the module so the only way in production is a screen actually opening.
#[cfg(test)]
fn install(state: TableScreen) {
    let mut guard = STATE.lock().unwrap();
    *guard = Some(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_starts_empty_and_resets() {
        reset();
        assert!(!is_open());
        install(TableScreen::new(Language::English, [640.0, 360.0]));
        assert!(is_open());
        reset();
        assert!(!is_open());
    }

    #[test]
    fn a_notice_is_taken_once() {
        reset();
        let mut state = TableScreen::new(Language::English, [640.0, 360.0]);
        state.notice = Some(crate::ui::localization::KEY_NO_TRANSMUTATION);
        install(state);
        assert_eq!(
            take_notice(),
            Some(crate::ui::localization::KEY_NO_TRANSMUTATION)
        );
        assert_eq!(take_notice(), None);
        reset();
    }

    #[test]
    fn the_layout_the_server_places_does_not_depend_on_the_window() {
        let native = screen_of(crate::recipes::TABLE).expect("the table is laid out");
        let again = screen_of(crate::recipes::TABLE).unwrap();
        assert_eq!(native, again);
        // Another block is not ours.
        assert!(screen_of("minecraft:chest").is_none());
    }

    #[test]
    fn a_poisoned_lock_does_not_close_the_screen_for_good() {
        reset();
        let _ = std::panic::catch_unwind(|| {
            let _guard = STATE.lock().unwrap();
            panic!("a hook failed while holding the screen");
        });
        // The lock is poisoned now; the accessors recover instead of failing.
        assert!(!is_open());
        install(TableScreen::new(Language::English, [640.0, 360.0]));
        assert!(is_open());
        reset();
    }

    fn screen_of(block: &str) -> Option<NativeScreen> {
        super::screen(block)
    }
}
