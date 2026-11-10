//! The deposit menu: the server-side half of deposit-and-learn.
//!
//! # Why a menu of its own
//!
//! The Transmutation Table's own screen is a `native_screen` over a mod
//! block's menu (`ui`). That menu's slots hold the player's own items, and a
//! cancelled click there would make the table unusable — so a deposit cannot be
//! a click on that screen without either an unguarded mutation or a screen that
//! swallows its slots. Neither is acceptable: the first risks losing items, the
//! second makes every slot decoration.
//!
//! This module therefore uses the API's **dedicated** menu facility for a menu
//! a mod opens (`native_open_menu_with_input`, API `menu_input`):
//!
//! * **Nine input slots** take items from the player by vanilla's own click
//!   rules, including shift-click and drag. The engine owns them, so a deposit
//!   can never be half-applied by a click we did not see.
//! * **Nine display slots** are the mod's to draw. The engine delivers a click
//!   on one of them as `native_menu_clicked`, where a click "moved nothing" —
//!   by construction, since the mod owns that slot.
//! * **Slot 18** is the confirm control, drawn as a bright button. Clicking it
//!   is the only thing that deposits anything.
//!
//! So the player's confirm click is a *real* server-side click, the items are
//! *really* in a container the engine moves, and the EMC credit happens only
//! after the engine's own removal is read back. There is no visual-only button
//! that mints EMC anywhere in this path.
//!
//! # The mutation, and why it cannot lose an item
//!
//! `apply` never trusts its own write. It writes the emptied deposit slots
//! back with `native_set_menu_items` — which, by design, leaves input slots
//! exactly as the player left them — then reads `native_menu_contents` again
//! and credits only the items that verifiably left ([`deposit::settle`]). If
//! the write did not take, nothing is credited, nothing is learned, and the
//! player is told; the items are still in the slots they can see.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use horizon_client::mod_api::{Host, ItemStack, Text};

use crate::data;

/// The module's menu shape, so the layout and the click handler cannot drift.
pub(crate) mod layout {
    /// Rows of the opened menu: 3 gives 27 slots.
    pub(crate) const ROWS: u8 = 3;
    /// The whole menu's slots: `ROWS * 9`.
    pub(crate) const SIZE: usize = ROWS as usize * 9;
    /// The nine slots the player drops items into.
    pub(crate) const INPUT_FIRST: usize = 0;
    pub(crate) const INPUT_LAST: usize = 9;
    /// The nine slots that show what the current deposit is worth.
    pub(crate) const DISPLAY_FIRST: usize = 9;
    pub(crate) const DISPLAY_LAST: usize = 18;
    /// The confirm control: a display slot the player clicks to deposit.
    pub(crate) const CONFIRM: usize = 18;
    /// The slot that shows whether the deposit would be accepted.
    pub(crate) const STATUS: usize = 19;
    /// The slots that show the player's balance and knowledge count.
    pub(crate) const BALANCE: usize = 20;
    pub(crate) const KNOWN: usize = 21;
    /// The item drawn on each control slot; they are labels, not invitations.
    pub(crate) const CONFIRM_ITEM: &str = "minecraft:lime_dye";
    pub(crate) const STATUS_ITEM: &str = "minecraft:paper";
    pub(crate) const BALANCE_ITEM: &str = "minecraft:emerald";
    pub(crate) const KNOWN_ITEM: &str = "minecraft:book";

    /// Whether `slot` is one the player fills.
    pub(crate) fn is_input(slot: usize) -> bool {
        (INPUT_FIRST..INPUT_LAST).contains(&slot)
    }
}

use layout::*;

/// The windows this mod opened, so a click is only a deposit in *its* menu.
///
/// Every native menu in the engine is `Slots::Native`, and the engine does not
/// say which mod opened one. A confirm click therefore cannot be recognised by
/// the slot alone: slot 18 of some other mod's menu is not a deposit, and
/// treating it as one would empty that menu into EMC. So a window counts only
/// after this module opened it, and stops counting the moment it closes.
static OPEN: Mutex<BTreeMap<u128, i32>> = Mutex::new(BTreeMap::new());

fn lock() -> MutexGuard<'static, BTreeMap<u128, i32>> {
    // A poisoned lock means another thread panicked mid-write; the map is a
    // plain `u128 -> i32` and cannot be left half-updated, so the data is
    // still sound and the deposit flow must keep working.
    OPEN.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether this click landed on the confirm control of the menu *this* mod
/// opened for `uuid`.
///
/// A missing entry means the mod has no menu open for that player — because it
/// never opened one, or because it closed — and then no click is a deposit.
pub(crate) fn confirm_slot(uuid: u128, window: i32, slot: i32) -> Option<usize> {
    let slot = usize::try_from(slot).ok()?;
    if slot != CONFIRM {
        return None;
    }
    (lock().get(&uuid).copied() == Some(window)).then_some(slot)
}

/// Forgets a window, so its slot 18 is no longer a confirm control.
pub(crate) fn forget(uuid: u128, window: i32) {
    let mut open = lock();
    if open.get(&uuid).copied() == Some(window) {
        open.remove(&uuid);
    }
}

/// Opens the deposit menu for `uuid`, if that player is here and the world can
/// take one. The menu replaces whatever the player had open, as any menu does.
///
/// Returns the window id the engine assigned, or None when the player is not
/// present (a dedicated server's guest who left, a spectator, the wrong side).
pub(crate) fn open(host: &mut dyn Host, uuid: u128) -> Option<i32> {
    let mut items = vec![None; SIZE];
    // The controls are drawn with their own icons, so a click on one is a
    // click on something visible rather than on an empty square.
    items[CONFIRM] = Some(ItemStack::new(CONFIRM_ITEM, 1));
    items[STATUS] = Some(ItemStack::new(STATUS_ITEM, 1));
    items[BALANCE] = Some(ItemStack::new(BALANCE_ITEM, 1));
    items[KNOWN] = Some(ItemStack::new(KNOWN_ITEM, 1));
    let window = host.open_menu_with_input(
        uuid,
        ROWS,
        Text::translate_with_fallback("container.projecte.deposit", "Deposit", &[]),
        items,
        INPUT_FIRST..INPUT_LAST,
    )?;
    // Remember it, so only this window's confirm slot can deposit. Recorded
    // after the open succeeded: a window that never opened has no clicks.
    lock().insert(uuid, window);
    Some(window)
}

/// Reads the deposit slots: the stacks the player has put in, with the menu
/// slot each sits in.
///
/// One entry per non-empty input slot. Nothing else in the menu is read, so a
/// click on a control cannot be mistaken for a deposit.
pub(crate) fn offered(contents: &[Option<ItemStack>]) -> Vec<deposit::Offered> {
    let mut out = Vec::new();
    for (slot, stack) in contents.iter().enumerate() {
        if !is_input(slot) {
            // A control or display slot holds a mod-placed icon, never a
            // deposit, so it is not read even if something is there.
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        if stack.count == 0 {
            continue;
        }
        out.push(deposit::Offered {
            slot,
            item: stack.item.to_string(),
            count: stack.count,
        });
    }
    out
}

/// What happened, so the caller can tell the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Deposited {
    /// Credited: `emc` gained, `learned` newly known.
    Ok { emc: i64, learned: usize },
    /// Nothing was in the slots.
    Empty,
    /// Refused before anything moved; the message says why.
    Refused(String),
    /// The write did not take, so nothing was credited.
    NotApplied,
}

/// Applies the deposit the player confirmed.
///
/// The sequence is fixed, and each step is checked:
///
/// 1. read the menu; nothing to deposit answers [`Deposited::Empty`];
/// 2. plan the whole deposit ([`deposit::plan`]); a refusal changes nothing;
/// 3. write the cleared input slots back;
/// 4. read the menu **again** and settle against what really left
///    ([`deposit::settle`]);
/// 5. credit EMC and knowledge only for what left, in one documented write.
///
/// Step 5 happens even for a partial departure, because that part is gone from
/// the player's slots and would otherwise be silently destroyed — which the
/// task forbids just as firmly as minting.
pub(crate) fn apply(
    host: &mut dyn Host,
    uuid: u128,
    mut items: Vec<Option<ItemStack>>,
) -> Deposited {
    let want = offered(&items);
    if want.is_empty() {
        return Deposited::Empty;
    }
    let before = data::get(host, uuid);
    let plan = match deposit::plan(&want, before.emc, crate::emc::value) {
        Ok(plan) => plan,
        Err(err) => return Deposited::Refused(err.message()),
    };
    // `plan` has already refused a balance that cannot hold the deposit, so
    // nothing has moved when this returns an error.

    // Clear exactly the slots the plan takes, leaving everything else — the
    // other input slots, the controls — as they are.
    for stack in &plan.taken {
        if let Some(slot) = items.get_mut(stack.slot) {
            *slot = None;
        }
    }
    host.set_menu_items(uuid, items);

    let after = host.menu_contents(uuid).unwrap_or_default();
    let still = |stack: &deposit::Offered| -> u32 {
        match after.get(stack.slot).and_then(|s| s.as_ref()) {
            Some(left) if left.item.as_ref() == stack.item.as_str() => left.count,
            // A different item in the slot means ours really is gone; counting
            // the new item as "still here" would refuse a valid deposit.
            _ => 0,
        }
    };
    let settlement = deposit::settle(&plan, crate::emc::value, still, &before.learned);
    if settlement.credited == 0 {
        return Deposited::NotApplied;
    }

    // The up-front check bounds the credit, so this cannot fail; `checked_add`
    // rather than `saturating_add` means a future change that breaks that
    // bound refuses the deposit instead of silently losing EMC.
    let Some(balance) = before.emc.checked_add(settlement.credited) else {
        return Deposited::NotApplied;
    };

    // One write for both halves so EMC and knowledge can never disagree.
    let mut after_data = before.clone();
    after_data.emc = balance;
    let mut added = 0;
    for item in &settlement.newly_learned {
        if after_data.learned.insert(item.clone()) {
            added += 1;
        }
    }
    data::set(host, uuid, after_data.clone());

    // Redraw the controls so the shown balance is the credited one. The
    // controls are mod-owned display slots, so writing them cannot disturb an
    // item the player is still holding in an input slot.
    let mut redraw: Vec<Option<ItemStack>> = after;
    redraw.resize(SIZE, None);
    redraw[BALANCE] = Some(balance_stack(after_data.emc));
    redraw[KNOWN] = Some(ItemStack::new(
        KNOWN_ITEM,
        count_shown(after_data.learned.len()),
    ));
    host.set_menu_items(uuid, redraw);

    Deposited::Ok {
        emc: settlement.credited,
        learned: added,
    }
}

/// The item drawn on the balance slot, so the number is not the only cue.
fn balance_stack(emc: i64) -> ItemStack {
    // The icon's count is the whole message a stack can carry: a large balance
    // shows a full stack, and `/projecte emc` remains the exact number.
    let shown = usize::try_from(emc).unwrap_or(usize::MAX);
    ItemStack::new(BALANCE_ITEM, count_shown(shown))
}

/// The stack size used as a small on-slot indicator, clamped to a stack.
fn count_shown(value: usize) -> u32 {
    u32::try_from(value.clamp(1, 64)).unwrap_or(64)
}

/// Tells the player what their confirm click did.
///
/// Every branch says something: a deposit is never silently ignored, and a
/// failure never claims success. `feedback` is the action bar, which the
/// screen hooks cannot reach (`&dyn Host`) but a menu hook can (`&mut dyn
/// Host`).
pub(crate) fn report(host: &mut dyn Host, uuid: u128, result: Deposited) {
    match result {
        Deposited::Ok { emc, learned } => {
            let known = data::get(host, uuid).learned.len();
            host.feedback(Text::literal(format!(
                "§aDeposited §e+{emc} EMC§a; §e{learned}§a new item(s); §e{known}§a known."
            )));
        }
        Deposited::Empty => host.feedback(Text::literal(
            "§cPut items in the deposit slots first, then click the confirm slot.",
        )),
        Deposited::Refused(message) => host.feedback(Text::literal(message)),
        Deposited::NotApplied => host.feedback(Text::literal(
            "§cThe deposit did not go through, so nothing was taken and no EMC was credited. Your items are still in the slots.",
        )),
    }
}

/// The item ids a deposit of these stacks would teach, for the tooltip.

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: &str, count: u32) -> Option<ItemStack> {
        Some(ItemStack::new(id, count))
    }

    #[test]
    fn only_the_nine_input_slots_are_read() {
        let mut contents = vec![None; SIZE];
        contents[0] = stack("minecraft:diamond", 2);
        contents[8] = stack("minecraft:coal", 4);
        // Controls and displays are not deposits, even when something sits
        // there: the mod put it there.
        contents[CONFIRM] = stack("minecraft:diamond", 64);
        contents[DISPLAY_FIRST] = stack("minecraft:diamond", 64);

        let offered = offered(&contents);
        assert_eq!(offered.len(), 2);
        assert_eq!(offered[0].slot, 0);
        assert_eq!(offered[1].slot, 8);
        assert!(offered.iter().all(|o| o.slot < INPUT_LAST));
    }

    #[test]
    fn an_empty_menu_offers_nothing() {
        assert!(offered(&vec![None; SIZE]).is_empty());
    }

    #[test]
    fn a_zero_count_slot_is_not_offered() {
        let mut contents = vec![None; SIZE];
        contents[3] = stack("minecraft:diamond", 0);
        assert!(offered(&contents).is_empty());
    }

    #[test]
    fn only_the_confirm_slot_is_a_confirm_click() {
        assert_eq!(confirm_slot(CONFIRM as i32, 1), Some(CONFIRM));
        for other in [0i32, 9, 17, 19, 20, 26, 27, -999] {
            assert_eq!(confirm_slot(other, 1), None, "slot {other}");
        }
        // A negative window slot is not a confirm either.
        assert_eq!(confirm_slot(-1, 1), None);
    }

    #[test]
    fn a_confirm_click_only_counts_in_the_window_this_mod_opened() {
        let uuid = 0xdead_beef_u128;
        // No menu open for this player: slot 18 of any other menu is not a
        // deposit, so another mod's menu cannot be emptied into EMC.
        assert_eq!(confirm_slot(uuid, 7, CONFIRM as i32), None);

        // Another player's window does not authorise this one either.
        lock().insert(0x1_u128, 7);
        assert_eq!(confirm_slot(uuid, 7, CONFIRM as i32), None);
        lock().remove(&0x1_u128);

        lock().insert(uuid, 7);
        assert_eq!(confirm_slot(uuid, 7, CONFIRM as i32), Some(CONFIRM));
        // The right player, a different window: still not a deposit.
        assert_eq!(confirm_slot(uuid, 8, CONFIRM as i32), None);
        // The right window, a non-confirm slot: not a deposit.
        assert_eq!(confirm_slot(uuid, 7, 0), None);
        assert_eq!(confirm_slot(uuid, 7, -1), None);

        // A closed window stops counting, so a reused id is not this mod's.
        forget(uuid, 8);
        assert_eq!(confirm_slot(uuid, 7, CONFIRM as i32), Some(CONFIRM));
        forget(uuid, 7);
        assert_eq!(confirm_slot(uuid, 7, CONFIRM as i32), None);
    }

    #[test]
    fn the_layout_keeps_the_parts_apart() {
        // The inputs, the displays and the controls must not share a slot, or
        // a click meant for one would be read as another.
        assert_eq!(INPUT_LAST, DISPLAY_FIRST);
        assert_eq!(DISPLAY_LAST, CONFIRM);
        assert!(CONFIRM < SIZE && STATUS < SIZE && BALANCE < SIZE && KNOWN < SIZE);
        assert_eq!(SIZE, 27);
        for slot in [CONFIRM, STATUS, BALANCE, KNOWN] {
            assert!(!is_input(slot));
        }
    }

    #[test]
    fn the_balance_indicator_is_a_legal_stack_size() {
        // A stack's count is 1..=64 in the icon; a huge balance must not
        // overflow it.
        assert_eq!(count_shown(0), 1);
        assert_eq!(count_shown(1), 1);
        assert_eq!(count_shown(64), 64);
        assert_eq!(count_shown(65), 64);
        assert_eq!(count_shown(usize::MAX), 64);
        let icon = balance_stack(i64::MAX);
        assert!((1..=64).contains(&icon.count));
        assert_eq!(
            balance_stack(0).count,
            1,
            "a zero balance still shows an icon"
        );
    }
}
