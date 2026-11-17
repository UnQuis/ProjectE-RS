//! The Transmutation Table screen's lifecycle and its input (SDK 0.2).
//!
//! The block-scoped moments (`screen`, `screen_draw`, `screen_tooltip`) are
//! delivered through the table block entity's `BlockEntityPart`; the global
//! client hooks forward the rest here with a `ScreenView`. Only the local
//! player's screen state is kept: the menu and its stacks are the server's.

use std::sync::Mutex;

use mod_api::{DrawContext, Flow, Screen, ScreenOpening, ScreenView};

use super::atlas;
use super::layout::Part;
use super::localization::Language;
use super::table::{BUTTON_PAGE_NEXT, BUTTON_PAGE_PREV};
use super::state::{Response, TableScreen};

static STATE: Mutex<Option<TableScreen>> = Mutex::new(None);

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

/// The SDK has no "current language" helper for screens; English is the
/// deterministic fallback until one is exposed.
fn language() -> Language {
    Language::English
}

pub(crate) fn screen(block: &str) -> Option<Screen> {
    if block != crate::recipes::TABLE {
        return None;
    }
    Some(atlas::native_screen())
}

pub(crate) fn screen_draw(
    _host: &dyn mod_api::ClientHost,
    draw: &mut dyn DrawContext,
    block: &str,
    data: &[i32],
    origin: [f32; 2],
) {
    if block != crate::recipes::TABLE {
        return;
    }
    with_state((), |state| {
        // Data slot 0 is the server's page; the menu syncs it to the client.
        if let Some(&page) = data.first() {
            state.browser.page = page.max(0) as usize;
        }
        atlas::draw(draw, state, origin);
    });
}

/// The scoped `BlockEntityPart::screen_click` answer for the pager.
///
/// Horizon turns the returned id into a server-side `menu_button`, which is
/// the only way the authoritative page (block-entity data slot 0) can change.
/// `at` is relative to the screen's top left; the layout hit-tests in window
/// GUI pixels, so the panel origin is added.
pub(crate) fn screen_click(block: &str, at: [f32; 2]) -> Option<i32> {
    if block != crate::recipes::TABLE {
        return None;
    }
    with_state(None, |state| {
        // `ScreenClickEvent::at` is relative to the screen's top left; the
        // layout hit-tests in window GUI pixels.
        let origin = state.layout.origin;
        state.moved([at[0] + origin[0], at[1] + origin[1]]);
        match state.hit()?.part {
            Part::PagePrev => Some(BUTTON_PAGE_PREV),
            Part::PageNext => Some(BUTTON_PAGE_NEXT),
            _ => None,
        }
    })
}

pub(crate) fn screen_tooltip(
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

/// Whether an open screen is one of this mod's table menus.
///
/// `screen_pos` is not on `ScreenView`; the menu type and slot count are the
/// only shape information the global hooks get. A vanilla one-row chest has
/// the same type, so this is a deliberate, documented limitation until the
/// scoped `BlockEntityPart` input hooks are available.
fn is_table(view: &ScreenView<'_>) -> bool {
    view.id == "inventory"
        && view.menu == Some(atlas::MENU)
        && view.slots == atlas::SHOWN_SLOTS + atlas::PLAYER_SLOTS
}

/// Starts the table's screen state. Returns the SDK hook's replacement answer:
/// `None` lets the engine open its normal container screen — the menu is ours,
/// so there is no reason to replace or cancel it.
pub(crate) fn screen_opening(view: &ScreenView<'_>) -> Option<ScreenOpening> {
    if !is_table(view) {
        return None;
    }
    let gui = gui_size(view);
    let language = language();
    let mut guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    match guard.as_mut() {
        Some(state) => {
            state.resize(gui);
            state.language = language;
        }
        None => *guard = Some(TableScreen::new(language, gui)),
    }
    None
}

pub(crate) fn screen_closing(view: &ScreenView<'_>) {
    if !is_table(view) {
        return;
    }
    reset();
}

fn gui_size(view: &ScreenView<'_>) -> [f32; 2] {
    [view.rect[2].max(1.0), view.rect[3].max(1.0)]
}

pub(crate) fn mouse_pressed(
    view: &ScreenView<'_>,
    _button: i32,
    _modifiers: u32,
) -> Flow {
    if !is_table(view) {
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
        state.hit().is_some_and(|hit| hit.part.is_control())
    });
    if cancel { Flow::Cancel } else { Flow::Continue }
}

pub(crate) fn mouse_released(
    view: &ScreenView<'_>,
    _button: i32,
    _modifiers: u32,
) -> Flow {
    if !is_table(view) {
        return Flow::Continue;
    }
    with_state((), |state| {
        let _ = state.release(view.mouse);
    });
    Flow::Continue
}

pub(crate) fn char_typed(view: &ScreenView<'_>, c: char, _modifiers: u32) -> Flow {
    if !is_table(view) {
        return Flow::Continue;
    }
    let taken = with_state(false, |state| {
        let response = state.typed(c);
        matches!(response, Response::Changed) || state.typing
    });
    if taken { Flow::Cancel } else { Flow::Continue }
}

pub(crate) fn key_pressed(
    view: &ScreenView<'_>,
    key: &str,
    _repeat: bool,
    _modifiers: u32,
) -> Flow {
    if !is_table(view) {
        return Flow::Continue;
    }
    let handled = with_state(false, |state| state.key(key));
    if handled { Flow::Cancel } else { Flow::Continue }
}

#[allow(dead_code)]
pub(crate) fn is_open() -> bool {
    let guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.is_some()
}

pub(crate) fn reset() {
    let mut guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = None;
}

#[cfg(test)]
fn take_notice() -> Option<&'static str> {
    with_state(None, |state| state.notice.take())
}

#[cfg(test)]
fn install(state: TableScreen) {
    let mut guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests share the one process-wide screen state; serialize them so
    // a parallel test cannot install state between another test's assertions.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn serial() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn the_state_starts_empty_and_resets() {
        let _serial = serial();
        reset();
        assert!(!is_open());
        install(TableScreen::new(Language::English, [640.0, 360.0]));
        assert!(is_open());
        reset();
        assert!(!is_open());
    }

    #[test]
    fn a_notice_is_taken_once() {
        let _serial = serial();
        reset();
        let mut state = TableScreen::new(Language::English, [640.0, 360.0]);
        state.notice = Some(crate::ui::localization::KEY_NO_TRANSMUTATION);
        install(state);
        assert_eq!(take_notice(), Some(crate::ui::localization::KEY_NO_TRANSMUTATION));
        assert_eq!(take_notice(), None);
        reset();
    }

    #[test]
    fn the_layout_the_server_places_does_not_depend_on_the_window() {
        let _serial = serial();
        let native = screen(crate::recipes::TABLE).expect("the table is laid out");
        let again = screen(crate::recipes::TABLE).unwrap();
        assert_eq!(native, again);
        assert!(screen("minecraft:chest").is_none());
    }

    #[test]
    fn a_poisoned_lock_does_not_close_the_screen_for_good() {
        let _serial = serial();
        reset();
        let _ = std::panic::catch_unwind(|| {
            let _guard = STATE.lock().unwrap();
            panic!("a hook failed while holding the screen");
        });
        assert!(!is_open());
        install(TableScreen::new(Language::English, [640.0, 360.0]));
        assert!(is_open());
        reset();
    }
}
