//! Collector and relay behavior.
//!
//! Machine EMC is stored in the block entity's saved NBT compound. This is a
//! deliberate compatibility workaround, documented in API_WORKAROUNDS.md.

use horizon_client::items::ItemStack;
use horizon_client::mod_api::{Compound, Holder, Host, ModBlockEntity, ModContainer, Tag, Text};

use crate::{data, emc, energy};

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
    is_machine(block)
}

pub(crate) fn container(block: &str) -> Option<ModContainer> {
    if !is_machine(block) {
        return None;
    }
    Some(ModContainer {
        slots: 3,
        faces: [
            vec![2, 1],
            vec![0],
            vec![1],
            vec![1],
            vec![1],
            vec![1],
        ],
        inputs: vec![0, 1],
        outputs: vec![2],
        menu: Some("minecraft:furnace".to_string()),
    })
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

fn take_one(slot: &mut Option<ItemStack>) {
    if let Some(stack) = slot.as_mut() {
        stack.count = stack.count.saturating_sub(1);
        if stack.count == 0 {
            *slot = None;
        }
    }
}

fn move_full_star(entity: &mut ModBlockEntity, input_slot: usize) {
    if input_slot >= entity.items.len() || entity.items[input_slot].is_none() {
        return;
    }
    if entity.items.get(2).is_some_and(Option::is_some) {
        return;
    }
    entity.items[2] = entity.items[input_slot].take();
}

/// Collector generates EMC, charges a star in slot 0, and feeds adjacent
/// Relays. Slot 1 is reserved for a future fuel-lock/upgrade feature.
fn collector_tick(
    host: &mut dyn Host,
    event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
) {
    let mut stored = next_buffer(
        read_emc(&event.entity.nbt),
        COLLECTOR_RATE,
        COLLECTOR_CAPACITY,
    );

    if let Some(mut star) = event.entity.items[0].clone() {
        let moved = energy::charge(host, &mut star, stored.min(RELAY_TRANSFER));
        stored -= moved;
        event.entity.items[0] = Some(star);
        let full = event.entity.items[0].as_ref().is_some_and(|s| {
            energy::stored_emc(host, s) >= energy::capacity(&s.item).unwrap_or(i64::MAX)
        });
        if full {
            move_full_star(event.entity, 0);
        }
    }

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
    event.entity.data = vec![stored as i32, COLLECTOR_CAPACITY as i32, 0, 1];
}

/// Relay accepts EMC-bearing items or Klein Stars in slot 0 and charges a
/// Klein Star in slot 1. A full star moves to output slot 2.
fn relay_tick(
    host: &mut dyn Host,
    event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
) {
    let mut stored = read_emc(&event.entity.nbt).min(RELAY_CAPACITY);

    if let Some(mut input) = event.entity.items[0].clone() {
        if energy::capacity(&input.item).is_some() {
            let moved = energy::discharge(
                host,
                &mut input,
                (RELAY_CAPACITY - stored).min(RELAY_TRANSFER),
            );
            stored += moved;
            event.entity.items[0] = Some(input);
        } else {
            let value = emc::value(&input.item);
            if value > 0 && value <= (RELAY_CAPACITY - stored) as u64 {
                stored += value as i64;
                take_one(&mut event.entity.items[0]);
            }
        }
    }

    if let Some(mut output) = event.entity.items[1].clone() {
        if energy::capacity(&output.item).is_some() {
            let moved = energy::charge(host, &mut output, stored.min(RELAY_TRANSFER));
            stored -= moved;
            event.entity.items[1] = Some(output);
            let full = event.entity.items[1].as_ref().is_some_and(|s| {
                energy::stored_emc(host, s) >= energy::capacity(&s.item).unwrap_or(i64::MAX)
            });
            if full {
                move_full_star(event.entity, 1);
            }
        }
    }

    write_emc(&mut event.entity.nbt, stored);
    event.entity.data = vec![stored as i32, RELAY_CAPACITY as i32, 0, 1];
}

pub(crate) fn tick(
    host: &mut dyn Host,
    event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
) {
    if event.entity.id != BLOCK_ENTITY {
        return;
    }
    let block = host.block(event.pos);
    if event.entity.nbt.get(&EMC_KEY.into()).is_none() {
        let legacy = data::BUFFER
            .get(host, Holder::BlockEntity(event.pos))
            .unwrap_or(0)
            .clamp(0, capacity(&block).unwrap_or(0));
        write_emc(&mut event.entity.nbt, legacy);
        let _ = data::BUFFER.set(host, Holder::BlockEntity(event.pos), 0);
    }
    if event.entity.items.len() < 3 {
        event.entity.items.resize(3, None);
    }
    match block.as_str() {
        COLLECTOR => collector_tick(host, event),
        RELAY => relay_tick(host, event),
        _ => {}
    }
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
        assert!(has_tick(RELAY));
        assert!(is_machine(COLLECTOR));
        assert!(is_machine(RELAY));
        assert!(!is_machine("minecraft:furnace"));
        assert_eq!(container(COLLECTOR).unwrap().slots, 3);
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
