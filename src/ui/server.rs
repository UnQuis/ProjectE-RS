//! Server-side helpers shared by the table/menu hooks in `lib.rs`.
//!
//! The display model in `super::table` is pure; this is the thin, fallible
//! bridge that reads the authoritative player data and writes block-menu slots
//! inside a transaction. No reward is ever granted here.

use mod_api::{HostTransaction, MenuSlotBacking, ServerHost};

use super::table;
use crate::{data, emc};

/// Clear a slot of the table's **block** menu inside a transaction.
///
/// `menu_set_items` only rewrites menus opened with `open_menu`, so a table
/// slot cannot be cleared that way. The menu handle names the backing block
/// entity, whose item slot is written transactionally instead. `false` means
/// nothing was changed — the caller must not credit EMC in that case.
pub(crate) fn clear_menu_slot(
    host: &mut dyn ServerHost,
    uuid: u128,
    window: i32,
    slot: usize,
) -> bool {
    let Some(menu) = host.menu(uuid, window) else {
        return false;
    };
    let Some(MenuSlotBacking::BlockEntity { level, pos, slot }) = menu.slot_backing(host, slot) else {
        return false;
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return false;
    };
    let result: Result<bool, mod_api::BlockEntityItemError> =
        host.transaction(|host, tx| entity.set_item_slot(host, tx, slot as u32, None));
    result.unwrap_or(false)
}

/// The item shown in a table output slot for a given page, from the server's
/// own knowledge/EMC/lock data.
///
/// The client's page lives in the table block entity's data slot 0 (synced to
/// the menu); pass it here so both sides name the same item for a click.
pub(crate) fn item_at_slot(
    host: &mut dyn ServerHost,
    uuid: u128,
    page: usize,
    slot: usize,
) -> Option<String> {
    let player = data::get(host, uuid).ok()?;
    let value = |id: &str| match emc::value_mapped(host, id) {
        Ok(value) => value,
        Err(error) => {
            // The static table is a deliberate, diagnosed fallback — never a
            // silent substitution for a broken data map.
            eprintln!("[projecte] EMC data map lookup failed for {id}: {error:?}; using static value");
            emc::value(id)
        }
    };
    // ProjectE's fuel section is decided by `COLLECTOR_FUEL`; until that tag
    // is read here, the item id is the deterministic approximation.
    let is_fuel = |id: &str| id.contains("fuel");
    let display = table::display_page(
        &player.learned,
        value,
        is_fuel,
        player.emc,
        player.lock.as_deref(),
        |_| true,
        page,
    );
    if slot >= table::OUTPUT_FIRST && slot < table::OUTPUT_FIRST + table::MATTER_DISPLAY {
        display.matter.get(slot - table::OUTPUT_FIRST).cloned()
    } else {
        display
            .fuel
            .get(slot - table::OUTPUT_FIRST - table::MATTER_DISPLAY)
            .cloned()
    }
}
