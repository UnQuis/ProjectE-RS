//! The deposit menu: the server-side half of deposit-and-learn.
//!
//! The menu is opened with the SDK's `MenusServerHost::open_menu`; its slots are
//! real server-owned slots. The confirm click is delivered by `menu_clicked`,
//! and `apply` never trusts its own write: it clears the input slots, rewrites
//! them with `menu_set_items`, reads the menu back through `MenuRef`, and
//! credits only what verifiably left ([`deposit::settle`]).

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use mod_api::*;

use crate::{data, deposit};

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
    #[cfg(test)]
    pub(crate) const DISPLAY_FIRST: usize = 9;
    #[cfg(test)]
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
static OPEN: Mutex<BTreeMap<u128, i32>> = Mutex::new(BTreeMap::new());

fn lock() -> MutexGuard<'static, BTreeMap<u128, i32>> {
    OPEN.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn confirm_slot(uuid: u128, window: i32, slot: i32) -> Option<usize> {
    let slot = usize::try_from(slot).ok()?;
    if slot != CONFIRM {
        return None;
    }
    (lock().get(&uuid).copied() == Some(window)).then_some(slot)
}

pub(crate) fn forget(uuid: u128, window: i32) {
    let mut open = lock();
    if open.get(&uuid).copied() == Some(window) {
        open.remove(&uuid);
    }
}

fn icon(host: &dyn Host, id: &str) -> Option<ItemStack> {
    Identifier::parse_namespaced(id)
        .and_then(|id| host.item_key(&id))
        .and_then(|key| ItemStack::new(key, 1))
}

/// Opens the deposit menu for `uuid`, returning the window id or `None`.
pub(crate) fn open(host: &mut dyn ServerHost, uuid: u128) -> Option<i32> {
    let mut items = vec![None; SIZE];
    items[CONFIRM] = icon(host, CONFIRM_ITEM);
    items[STATUS] = icon(host, STATUS_ITEM);
    items[BALANCE] = icon(host, BALANCE_ITEM);
    items[KNOWN] = icon(host, KNOWN_ITEM);
    let window = host
        .open_menu(
            uuid,
            ROWS,
            Text::translate("container.projecte.deposit", Vec::new()),
            items,
            MenuReplace::CloseFirst,
        )
        .ok()
        .flatten()?;
    lock().insert(uuid, window);
    Some(window)
}

/// The stacks the player has put into the nine input slots.
pub(crate) fn offered(host: &dyn Host, contents: &[Option<ItemStack>]) -> Vec<deposit::Offered> {
    let mut out = Vec::new();
    for (slot, stack) in contents.iter().enumerate() {
        if !is_input(slot) {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        if stack.count() == 0 {
            continue;
        }
        out.push(deposit::Offered {
            slot,
            item: stack.item().id(host),
            count: stack.count(),
        });
    }
    out
}

/// What happened, so the caller can tell the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Deposited {
    Ok { emc: i64, learned: usize },
    Empty,
    Refused(String),
    NotApplied,
}

/// Reads all slots of an open menu made by `open_menu`.
///
/// SDK 0.2 has no `menu_contents(uuid)` helper; the menu handle carries the
/// per-slot reads.
pub(crate) fn contents(host: &mut dyn ServerHost, uuid: u128) -> Vec<Option<ItemStack>> {
    let Some(window) = lock().get(&uuid).copied() else {
        return Vec::new();
    };
    let Some(menu) = host.menu(uuid, window) else {
        return Vec::new();
    };
    (0..SIZE)
        .map(|slot| menu.slot(host, slot).flatten())
        .collect()
}

/// Applies the deposit the player confirmed.
pub(crate) fn apply(
    host: &mut dyn ServerHost,
    uuid: u128,
    mut items: Vec<Option<ItemStack>>,
) -> Deposited {
    let want = offered(host, &items);
    if want.is_empty() {
        return Deposited::Empty;
    }
    let before = match data::get(host, uuid) {
        Ok(data) => data,
        Err(_) => return Deposited::Refused("§cYour ProjectE data could not be read.".to_owned()),
    };
    let plan = match deposit::plan(&want, before.emc, crate::emc::value) {
        Ok(plan) => plan,
        Err(err) => return Deposited::Refused(err.message()),
    };

    for stack in &plan.taken {
        if let Some(slot) = items.get_mut(stack.slot) {
            *slot = None;
        }
    }
    let Some(window) = lock().get(&uuid).copied() else {
        return Deposited::NotApplied;
    };
    if host.menu_set_items(uuid, window, items).is_err() {
        return Deposited::NotApplied;
    }

    let after = contents(host, uuid);
    let still = |stack: &deposit::Offered| -> u32 {
        match after.get(stack.slot).and_then(|s| s.as_ref()) {
            Some(left) if left.item().id(host) == stack.item => left.count(),
            _ => 0,
        }
    };
    let settlement = deposit::settle(&plan, crate::emc::value, still, &before.learned);
    if settlement.credited == 0 {
        return Deposited::NotApplied;
    }

    let Some(balance) = before.emc.checked_add(settlement.credited) else {
        return Deposited::NotApplied;
    };
    let mut after_data = before.clone();
    after_data.emc = balance;
    let mut added = 0;
    for item in &settlement.newly_learned {
        if after_data.learned.insert(item.clone()) {
            added += 1;
        }
    }
    if data::set(host, uuid, after_data.clone()).is_err() {
        return Deposited::NotApplied;
    }

    let mut redraw = after;
    redraw.resize(SIZE, None);
    redraw[BALANCE] = balance_stack(host, after_data.emc);
    redraw[KNOWN] = icon(host, KNOWN_ITEM).and_then(|mut stack| {
        stack.set_count(count_shown(after_data.learned.len())).ok()?;
        Some(stack)
    });
    let _ = host.menu_set_items(uuid, window, redraw);

    Deposited::Ok {
        emc: settlement.credited,
        learned: added,
    }
}

fn balance_stack(host: &dyn Host, emc: i64) -> Option<ItemStack> {
    let shown = usize::try_from(emc).unwrap_or(usize::MAX);
    let mut stack = icon(host, BALANCE_ITEM)?;
    stack.set_count(count_shown(shown)).ok()?;
    Some(stack)
}

fn count_shown(value: usize) -> u32 {
    u32::try_from(value.clamp(1, 64)).unwrap_or(64)
}

pub(crate) fn report(host: &mut dyn ServerHost, uuid: u128, result: Deposited) {
    let line = match result {
        Deposited::Ok { emc, learned } => {
            let known = data::get(host, uuid).map(|data| data.learned.len()).unwrap_or(0);
            Text::literal(format!(
                "§aDeposited §e+{emc} EMC§a; §e{learned}§a new item(s); §e{known}§a known."
            ))
        }
        Deposited::Empty => Text::literal(
            "§cPut items in the deposit slots first, then click the confirm slot.",
        ),
        Deposited::Refused(message) => Text::literal(message),
        Deposited::NotApplied => Text::literal(
            "§cThe deposit did not go through, so nothing was taken and no EMC was credited. Your items are still in the slots.",
        ),
    };
    let _ = host.commands().feedback(line);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_confirm_slot_is_a_confirm_click() {
        let uuid = 0xface_u128;
        lock().insert(uuid, 1);
        assert_eq!(confirm_slot(uuid, 1, CONFIRM as i32), Some(CONFIRM));
        for other in [0i32, 9, 17, 19, 20, 26, 27, -999] {
            assert_eq!(confirm_slot(uuid, 1, other), None, "slot {other}");
        }
        assert_eq!(confirm_slot(uuid, -1, CONFIRM as i32), None);
        assert_eq!(confirm_slot(uuid, 1, -1), None);
        forget(uuid, 1);
        assert_eq!(confirm_slot(uuid, 1, CONFIRM as i32), None);
    }

    #[test]
    fn the_layout_keeps_the_parts_apart() {
        assert_eq!(INPUT_LAST, DISPLAY_FIRST);
        assert_eq!(DISPLAY_LAST, CONFIRM);
        const {
            assert!(CONFIRM < SIZE && STATUS < SIZE && BALANCE < SIZE && KNOWN < SIZE);
        }
        assert_eq!(SIZE, 27);
        for slot in [CONFIRM, STATUS, BALANCE, KNOWN] {
            assert!(!is_input(slot));
        }
    }

    #[test]
    fn the_balance_indicator_is_a_legal_stack_size() {
        assert_eq!(count_shown(0), 1);
        assert_eq!(count_shown(1), 1);
        assert_eq!(count_shown(64), 64);
        assert_eq!(count_shown(65), 64);
        assert_eq!(count_shown(usize::MAX), 64);
    }
}
