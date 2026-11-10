//! Collector and relay behavior, kept separate from hook orchestration.
//!
//! The current storage backend is a ProjectE attachment. Keeping the machine
//! rules behind this module gives us one place to migrate to Horizon's native
//! block-entity energy capability when its Host access is available.

use horizon_client::mod_api::{Compound, Holder, Host, Text};

use crate::data;

pub(crate) const COLLECTOR: &str = "projecte:energy_collector";
pub(crate) const RELAY: &str = "projecte:antimatter_relay";
const BLOCK_ENTITY: &str = "projecte:machine";

const COLLECTOR_CAPACITY: i64 = 100_000;
const RELAY_CAPACITY: i64 = 1_000_000;
const COLLECTOR_RATE: i64 = 1;
const RELAY_RATE: i64 = 4;

pub(crate) fn is_machine(block: &str) -> bool {
    matches!(block, COLLECTOR | RELAY)
}

pub(crate) fn block_entity(block: &str) -> Option<(String, Compound)> {
    is_machine(block).then(|| (BLOCK_ENTITY.to_owned(), Compound::new()))
}

pub(crate) fn has_tick(block: &str) -> bool {
    is_machine(block)
}

fn specification(block: &str) -> Option<(i64, i64)> {
    match block {
        COLLECTOR => Some((COLLECTOR_RATE, COLLECTOR_CAPACITY)),
        RELAY => Some((RELAY_RATE, RELAY_CAPACITY)),
        _ => None,
    }
}

fn next_buffer(current: i64, rate: i64, capacity: i64) -> i64 {
    current
        .clamp(0, capacity)
        .checked_add(rate)
        .unwrap_or(capacity)
        .min(capacity)
}

pub(crate) fn tick(
    host: &mut dyn Host,
    event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
) {
    if event.entity.id != BLOCK_ENTITY {
        return;
    }
    let block = host.block(event.pos);
    let Some((rate, capacity)) = specification(&block) else {
        return;
    };
    let holder = Holder::BlockEntity(event.pos);
    let current = data::BUFFER.get(host, holder).unwrap_or(0);
    let next = next_buffer(current, rate, capacity);
    if next != current {
        let _ = data::BUFFER.set(host, holder, next);
    }
}

pub(crate) fn collect(host: &mut dyn Host, pos: [i32; 3], player: u128) {
    let holder = Holder::BlockEntity(pos);
    let stored = data::BUFFER.get(host, holder).unwrap_or(0).max(0);
    if stored > 0 {
        let total = data::add_emc(host, player, stored);
        let _ = data::BUFFER.set(host, holder, 0);
        host.tell(
            player,
            Text::literal(format!(
                "§dCollected §e{} EMC§d. Stored: §e{}§d.",
                stored, total
            )),
        );
    } else {
        host.tell(
            player,
            Text::literal("§7This machine has not collected any EMC yet."),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_ids_have_expected_specs() {
        assert_eq!(specification(COLLECTOR), Some((1, 100_000)));
        assert_eq!(specification(RELAY), Some((4, 1_000_000)));
        assert_eq!(specification("minecraft:furnace"), None);
        assert!(is_machine(COLLECTOR));
        assert!(is_machine(RELAY));
        assert!(!is_machine("minecraft:furnace"));
    }

    #[test]
    fn accumulation_respects_capacity_and_rejects_negative_state() {
        assert_eq!(next_buffer(0, COLLECTOR_RATE, COLLECTOR_CAPACITY), 1);
        assert_eq!(next_buffer(99_999, 1, COLLECTOR_CAPACITY), 100_000);
        assert_eq!(next_buffer(100_000, 1, COLLECTOR_CAPACITY), 100_000);
        assert_eq!(next_buffer(-50, 4, RELAY_CAPACITY), 4);
        assert_eq!(next_buffer(1_000_000, 4, RELAY_CAPACITY), 1_000_000);
        assert_eq!(next_buffer(i64::MAX, 4, RELAY_CAPACITY), 1_000_000);
    }
}
