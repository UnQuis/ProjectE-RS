//! Collector and relay behavior.
//!
//! Machine EMC is stored in the block entity's saved NBT compound. This is a
//! deliberate compatibility workaround, documented in API_WORKAROUNDS.md.

use horizon_client::mod_api::{Compound, Host, Tag, Text};

use crate::data;

pub(crate) const COLLECTOR: &str = "projecte:energy_collector";
pub(crate) const RELAY: &str = "projecte:antimatter_relay";
const BLOCK_ENTITY: &str = "projecte:machine";
const EMC_KEY: &str = "emc";

const COLLECTOR_CAPACITY: i64 = 100_000;
const RELAY_CAPACITY: i64 = 1_000_000;
const COLLECTOR_RATE: i64 = 1;
const RELAY_TRANSFER: i64 = 4;

const FACES: [[i32; 3]; 6] = [
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

pub(crate) fn is_machine(block: &str) -> bool {
    matches!(block, COLLECTOR | RELAY)
}

pub(crate) fn block_entity(block: &str) -> Option<(String, Compound)> {
    is_machine(block).then(|| (BLOCK_ENTITY.to_owned(), Compound::new()))
}

pub(crate) fn has_tick(block: &str) -> bool {
    block == COLLECTOR
}

fn capacity(block: &str) -> Option<i64> {
    match block {
        COLLECTOR => Some(COLLECTOR_CAPACITY),
        RELAY => Some(RELAY_CAPACITY),
        _ => None,
    }
}

fn read_emc(nbt: &Compound) -> i64 {
    match nbt.get(&EMC_KEY.into()) {
        Some(Tag::Long(value)) => (*value).max(0),
        Some(Tag::Int(value)) => i64::from(*value).max(0),
        _ => 0,
    }
}

fn write_emc(nbt: &mut Compound, value: i64) {
    nbt.insert(EMC_KEY.into(), Tag::Long(value.max(0)));
}

fn next_buffer(current: i64, rate: i64, max: i64) -> i64 {
    current
        .clamp(0, max)
        .saturating_add(rate.max(0))
        .min(max)
}

/// The collector generates one EMC per tick and sends up to four EMC per
/// adjacent relay per tick. With the current one-EMC/tick generation this
/// also supports multiple collectors feeding a single relay.
pub(crate) fn tick(host: &mut dyn Host, event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>) {
    if event.entity.id != BLOCK_ENTITY || host.block(event.pos) != COLLECTOR {
        return;
    }

    let mut stored = next_buffer(
        read_emc(&event.entity.nbt),
        COLLECTOR_RATE,
        COLLECTOR_CAPACITY,
    );

    for face in FACES {
        if stored == 0 {
            break;
        }
        let pos = [
            event.pos[0] + face[0],
            event.pos[1] + face[1],
            event.pos[2] + face[2],
        ];
        if host.block(pos) != RELAY {
            continue;
        }

        let mut moved = 0;
        host.change_mod_block_entity(pos, &mut |target| {
            if target.id != BLOCK_ENTITY {
                return;
            }
            let target_emc = read_emc(&target.nbt).min(RELAY_CAPACITY);
            let room = RELAY_CAPACITY - target_emc;
            moved = stored.min(RELAY_TRANSFER).min(room).max(0);
            if moved > 0 {
                write_emc(&mut target.nbt, target_emc + moved);
            }
        });
        stored -= moved;
    }

    write_emc(&mut event.entity.nbt, stored);
}

pub(crate) fn collect(host: &mut dyn Host, pos: [i32; 3], player: u128) {
    let Some(machine_capacity) = capacity(&host.block(pos)) else {
        return;
    };
    let mut collected = 0;
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id != BLOCK_ENTITY {
            return;
        }
        collected = read_emc(&entity.nbt).min(machine_capacity);
        if collected > 0 {
            write_emc(&mut entity.nbt, 0);
        }
    });

    if collected > 0 {
        let total = data::add_emc(host, player, collected);
        host.tell(
            player,
            Text::literal(format!(
                "§dCollected §e{collected} EMC§d. Balance: §e{total}§d."
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
    fn machine_capacities_and_tick_registration() {
        assert_eq!(capacity(COLLECTOR), Some(COLLECTOR_CAPACITY));
        assert_eq!(capacity(RELAY), Some(RELAY_CAPACITY));
        assert_eq!(capacity("minecraft:furnace"), None);
        assert!(has_tick(COLLECTOR));
        assert!(!has_tick(RELAY));
        assert!(is_machine(COLLECTOR));
        assert!(is_machine(RELAY));
        assert!(!is_machine("minecraft:furnace"));
    }

    #[test]
    fn generation_is_bounded_and_recovers_invalid_values() {
        assert_eq!(next_buffer(0, COLLECTOR_RATE, COLLECTOR_CAPACITY), 1);
        assert_eq!(next_buffer(99_999, 1, COLLECTOR_CAPACITY), 100_000);
        assert_eq!(next_buffer(100_000, 1, COLLECTOR_CAPACITY), 100_000);
        assert_eq!(next_buffer(-50, 4, RELAY_CAPACITY), 4);
        assert_eq!(next_buffer(i64::MAX, 4, RELAY_CAPACITY), RELAY_CAPACITY);
    }

    #[test]
    fn stored_emc_is_read_from_block_entity_nbt() {
        let mut nbt = Compound::new();
        write_emc(&mut nbt, 1234);
        assert_eq!(read_emc(&nbt), 1234);
        write_emc(&mut nbt, -100);
        assert_eq!(read_emc(&nbt), 0);
    }
}
