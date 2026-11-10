//! EMC-producing and EMC-routing ProjectE machines.
//!
//! Machine buffers live in block-entity NBT because the pinned trait-mod Host
//! surface cannot call Horizon's native energy read/write helpers. See
//! API_WORKAROUNDS.md for the workaround and migration path.

use horizon_client::items::ItemStack;
use horizon_client::mod_api::{Compound, Holder, Host, ModBlockEntity, ModContainer, Tag, Text};

use crate::{condenser, data, emc, energy};

pub(crate) const COLLECTORS: [&str; 3] = [
    "projecte:energy_collector",
    "projecte:energy_collector_mk2",
    "projecte:energy_collector_mk3",
];
pub(crate) const RELAYS: [&str; 3] = [
    "projecte:antimatter_relay",
    "projecte:antimatter_relay_mk2",
    "projecte:antimatter_relay_mk3",
];
pub(crate) const ALCHEMICAL_CHEST: &str = "projecte:alchemical_chest";
const ALCHEMICAL_CHEST_BLOCK_ENTITY: &str = "projecte:alchemical_chest";

const BLOCK_ENTITY: &str = "projecte:machine";
const EMC_KEY: &str = "emc";
const MICRO_KEY: &str = "micro";
const BONUS_KEY: &str = "bonus";
const MICRO_PER_EMC: i64 = 1_000_000;

const COLLECTOR_CAPACITY: [i64; 3] = [10_000, 30_000, 60_000];
const COLLECTOR_RATE_PER_SECOND: [i64; 3] = [4, 12, 40];
const RELAY_CAPACITY: [i64; 3] = [100_000, 1_000_000, 10_000_000];
const TRANSFER_LIMIT: [i64; 3] = [64, 192, 640];
const RELAY_BONUS_PER_TICK_MILLI: [i64; 3] = [50, 150, 500];

const FACES: [[i32; 3]; 6] = [
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, 0, 0],
    [1, 0, 0],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Machine {
    Collector(usize),
    Relay(usize),
    Condenser(condenser::CondenserSpec),
}

impl Machine {
    fn of(block: &str) -> Option<Self> {
        if block == condenser::ENERGY_CONDENSER {
            return Some(Self::Condenser(condenser::MK1_SPEC));
        }
        if let Some(tier) = COLLECTORS.iter().position(|id| *id == block) {
            return Some(Self::Collector(tier));
        }
        RELAYS.iter().position(|id| *id == block).map(Self::Relay)
    }

    fn capacity(self) -> i64 {
        match self {
            Self::Collector(tier) => COLLECTOR_CAPACITY[tier],
            Self::Relay(tier) => RELAY_CAPACITY[tier],
            Self::Condenser(spec) => i64::try_from(spec.capacity).unwrap_or(i64::MAX),
        }
    }

    fn transfer(self) -> i64 {
        match self {
            Self::Collector(tier) | Self::Relay(tier) => TRANSFER_LIMIT[tier],
            Self::Condenser(_) => 0,
        }
    }

    fn fill(self, stored: i64) -> f64 {
        stored.clamp(0, self.capacity()) as f64 / self.capacity().max(1) as f64
    }

    fn feeds(self, target: Self, source_fill: f64, target_fill: f64) -> bool {
        match (self, target) {
            (Self::Collector(_), Self::Relay(_) | Self::Condenser(_)) => true,
            (Self::Relay(_), Self::Relay(_)) => target_fill + 1e-9 < source_fill,
            (Self::Relay(_), Self::Condenser(_)) => true,
            (Self::Condenser(_), _) => false,
            _ => false,
        }
    }
}

pub(crate) fn is_machine(block: &str) -> bool {
    Machine::of(block).is_some()
}

pub(crate) fn is_collectable(block: &str) -> bool {
    matches!(
        Machine::of(block),
        Some(Machine::Collector(_) | Machine::Relay(_))
    )
}

pub(crate) fn block_entity(block: &str) -> Option<(String, Compound)> {
    if block == ALCHEMICAL_CHEST {
        return Some((ALCHEMICAL_CHEST_BLOCK_ENTITY.to_owned(), Compound::new()));
    }
    is_machine(block).then(|| (BLOCK_ENTITY.to_owned(), Compound::new()))
}

pub(crate) fn has_tick(block: &str) -> bool {
    is_machine(block)
}

pub(crate) fn container(block: &str) -> Option<ModContainer> {
    if block == ALCHEMICAL_CHEST {
        let all_slots: Vec<usize> = (0..27).collect();
        return Some(ModContainer {
            slots: 27,
            faces: [
                all_slots.clone(),
                all_slots.clone(),
                all_slots.clone(),
                all_slots.clone(),
                all_slots.clone(),
                all_slots.clone(),
            ],
            inputs: all_slots.clone(),
            outputs: all_slots,
            menu: Some("minecraft:generic_9x3".to_string()),
        });
    }
    let machine = Machine::of(block)?;
    if matches!(machine, Machine::Condenser(_)) {
        return Some(ModContainer {
            slots: 9,
            faces: [
                vec![2],
                vec![0, 3, 4, 5, 6, 7, 8],
                vec![0, 3, 4, 5, 6, 7, 8],
                vec![0, 3, 4, 5, 6, 7, 8],
                vec![0, 3, 4, 5, 6, 7, 8],
                vec![0, 3, 4, 5, 6, 7, 8],
            ],
            inputs: vec![0, 3, 4, 5, 6, 7, 8],
            outputs: vec![2],
            menu: Some("minecraft:generic_9x1".to_string()),
        });
    }
    Some(ModContainer {
        slots: 3,
        faces: [vec![2, 1], vec![0], vec![1], vec![1], vec![1], vec![1]],
        inputs: vec![0, 1],
        outputs: vec![2],
        menu: Some("minecraft:furnace".to_string()),
    })
}

fn read_long(nbt: &Compound, key: &str) -> i64 {
    match nbt.get(&key.into()) {
        Some(Tag::Long(value)) => *value,
        Some(Tag::Int(value)) => i64::from(*value),
        _ => 0,
    }
}

fn write_long(nbt: &mut Compound, key: &str, value: i64) {
    nbt.insert(key.into(), Tag::Long(value));
}

fn read_emc(nbt: &Compound) -> i64 {
    read_long(nbt, EMC_KEY).max(0)
}
fn write_emc(nbt: &mut Compound, value: i64) {
    write_long(nbt, EMC_KEY, value.max(0));
}

fn migrate_legacy_buffer(host: &mut dyn Host, pos: [i32; 3], nbt: &mut Compound, max: i64) -> bool {
    if nbt.get(&EMC_KEY.into()).is_some() {
        return true;
    }
    let legacy = match data::BUFFER.get(host, Holder::BlockEntity(pos)) {
        Ok(value) => value.clamp(0, max),
        Err(_) => return false,
    };
    write_emc(nbt, legacy);
    let _ = data::BUFFER.set(host, Holder::BlockEntity(pos), 0);
    true
}

fn collector_sun_level(sky_open: bool, daytime: bool, light: u8) -> u8 {
    if sky_open && daytime {
        16
    } else {
        light.min(15).saturating_add(1).min(16)
    }
}

fn collector_sun(host: &dyn Host, pos: [i32; 3]) -> u8 {
    let above = [pos[0], pos[1] + 1, pos[2]];
    let (block_light, sky_light) = host.light(above);
    let time = host.day_time().rem_euclid(24_000);
    let daytime = !(12_300..=23_850).contains(&time) && host.weather().0 < 0.2;
    collector_sun_level(
        sky_light == 15,
        daytime,
        block_light.max(if daytime { sky_light } else { 0 }),
    )
}

fn collector_generation(rate_per_second: i64, sun: u8, micro: i64) -> (i64, i64) {
    let produced_micro = rate_per_second
        .saturating_mul(i64::from(sun.min(16)))
        .saturating_mul(MICRO_PER_EMC)
        / 320;
    let total = micro
        .clamp(0, MICRO_PER_EMC - 1)
        .saturating_add(produced_micro);
    (total / MICRO_PER_EMC, total % MICRO_PER_EMC)
}

fn share(offer: i64, room: &[i64]) -> Vec<i64> {
    let mut allocations = vec![0; room.len()];
    let mut remaining = offer.max(0);
    while remaining > 0 {
        let active: Vec<usize> = room
            .iter()
            .enumerate()
            .filter_map(|(index, available)| {
                (allocations[index] < (*available).max(0)).then_some(index)
            })
            .collect();
        if active.is_empty() {
            break;
        }
        let each = remaining / active.len() as i64;
        let remainder = remaining % active.len() as i64;
        let mut moved = 0;
        for (rank, index) in active.into_iter().enumerate() {
            let extra = if (rank as i64) < remainder { 1 } else { 0 };
            let amount = (each + extra)
                .max(1)
                .min((room[index] - allocations[index]).max(0))
                .min(remaining - moved);
            allocations[index] += amount;
            moved += amount;
        }
        if moved == 0 {
            break;
        }
        remaining -= moved;
    }
    allocations
}

fn push_emc(host: &mut dyn Host, pos: [i32; 3], source: Machine, stored: i64, offer: i64) -> i64 {
    let source_fill = source.fill(stored);
    let mut targets = Vec::new();
    let mut rooms = Vec::new();

    for face in FACES {
        let target_pos = [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]];
        let Some(target) = Machine::of(&host.block(target_pos)) else {
            continue;
        };
        let Some(block_entity) = host.mod_block_entity(target_pos) else {
            continue;
        };
        if block_entity.id != BLOCK_ENTITY {
            continue;
        }
        let target_emc = read_emc(&block_entity.nbt).min(target.capacity());
        let room = (target.capacity() - target_emc).max(0);
        if room > 0 && source.feeds(target, source_fill, target.fill(target_emc)) {
            targets.push((target_pos, target));
            rooms.push(room);
        }
    }

    let allocations = share(offer, &rooms);
    let mut total = 0;
    for ((target_pos, target), allocation) in targets.into_iter().zip(allocations) {
        if allocation == 0 {
            continue;
        }
        let mut moved = 0;
        host.change_mod_block_entity(target_pos, &mut |entity| {
            if entity.id != BLOCK_ENTITY {
                return;
            }
            let current = read_emc(&entity.nbt).min(target.capacity());
            moved = allocation.min(target.capacity() - current).max(0);
            if moved > 0 {
                write_emc(&mut entity.nbt, current + moved);
            }
        });
        total += moved;
    }
    total
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

fn furnace_data(stored: i64, capacity: i64) -> Vec<i32> {
    let scaled = if capacity <= 0 {
        0
    } else {
        (stored.clamp(0, capacity) as i128 * 1000 / capacity as i128) as i32
    };
    vec![scaled, 1000, 0, 1000]
}

fn collector_tick(
    host: &mut dyn Host,
    pos: [i32; 3],
    event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
    tier: usize,
) {
    let machine = Machine::Collector(tier);
    let capacity = machine.capacity();
    let mut stored = read_emc(&event.entity.nbt).min(capacity);
    let micro = read_long(&event.entity.nbt, MICRO_KEY);
    let sun = collector_sun(host, pos);
    let (generated, next_micro) = collector_generation(COLLECTOR_RATE_PER_SECOND[tier], sun, micro);
    stored = stored.saturating_add(generated).min(capacity);

    for slot in 0..2 {
        let Some(mut star) = event.entity.items[slot].clone() else {
            continue;
        };
        if energy::capacity(&star.item).is_none() {
            continue;
        }
        stored -= energy::charge(host, &mut star, stored.min(machine.transfer()));
        let full = energy::stored_emc(host, &star).unwrap_or(0)
            >= energy::capacity(&star.item).unwrap_or(i64::MAX);
        if full && event.entity.items[2].is_none() {
            event.entity.items[2] = Some(star);
            event.entity.items[slot] = None;
        } else {
            event.entity.items[slot] = Some(star);
        }
    }

    if stored > 0 {
        stored -= push_emc(host, pos, machine, stored, stored.min(machine.transfer()));
    }
    write_emc(&mut event.entity.nbt, stored);
    write_long(&mut event.entity.nbt, MICRO_KEY, next_micro);
    event.entity.data = furnace_data(stored, capacity);
}

fn relay_tick(
    host: &mut dyn Host,
    pos: [i32; 3],
    event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
    tier: usize,
) {
    let machine = Machine::Relay(tier);
    let capacity = machine.capacity();
    let mut stored = read_emc(&event.entity.nbt).min(capacity);

    if let Some(mut input) = event.entity.items[0].clone() {
        if energy::capacity(&input.item).is_some() {
            let moved = energy::discharge(
                host,
                &mut input,
                (capacity - stored).min(machine.transfer()),
            );
            stored += moved;
            event.entity.items[0] = Some(input);
        } else {
            let value = emc::value(&input.item);
            if value > 0 && value <= (capacity - stored) as u64 {
                stored += value as i64;
                take_one(&mut event.entity.items[0]);
            }
        }
    }

    if let Some(mut output) = event.entity.items[1].clone()
        && energy::capacity(&output.item).is_some()
    {
            stored -= energy::charge(host, &mut output, stored.min(machine.transfer()));
            event.entity.items[1] = Some(output);
            let full = event.entity.items[1].as_ref().is_some_and(|stack| {
                energy::stored_emc(host, stack).unwrap_or(0)
                    >= energy::capacity(&stack.item).unwrap_or(i64::MAX)
            });
        if full && event.entity.items[2].is_none() {
            move_full_star(event.entity, 1);
        }
    }

    if stored > 0 {
        let sent = push_emc(host, pos, machine, stored, stored.min(machine.transfer()));
        stored -= sent;
        if sent > 0 {
            let bonus = read_long(&event.entity.nbt, BONUS_KEY)
                .clamp(0, 999)
                .saturating_add(RELAY_BONUS_PER_TICK_MILLI[tier]);
            stored = stored.saturating_add(bonus / 1000).min(capacity);
            write_long(&mut event.entity.nbt, BONUS_KEY, bonus % 1000);
        }
    }
    write_emc(&mut event.entity.nbt, stored);
    event.entity.data = furnace_data(stored, capacity);
}

fn same_item_payload(left: &ItemStack, right: &ItemStack) -> bool {
    left.item == right.item && left.damage == right.damage && left.extra == right.extra
}

/// One server-authoritative Condenser step. Slot 1 is the non-consumed target
/// template, slot 2 is output, and slots 0/3..8 accept EMC-valued input.
fn condenser_tick(
    host: &dyn Host,
    event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
    spec: condenser::CondenserSpec,
) {
    let capacity = i64::try_from(spec.capacity).unwrap_or(i64::MAX);
    let stored = read_emc(&event.entity.nbt).clamp(0, capacity);
    let mut state = condenser::CondenserState {
        stored_emc: stored as u64,
    };

    let Some(target) = event.entity.items.get(1).and_then(Option::as_ref).cloned() else {
        write_emc(&mut event.entity.nbt, stored);
        event.entity.data = furnace_data(stored, capacity);
        return;
    };
    let target_value = emc::value(&target.item);
    if target_value == 0 {
        write_emc(&mut event.entity.nbt, stored);
        event.entity.data = furnace_data(stored, capacity);
        return;
    }

    // Process at most the tier budget. Invalid/unknown input stacks stay put;
    // the first valid stack that fits is converted.
    for slot in [0usize, 3, 4, 5, 6, 7, 8] {
        let Some(input) = event
            .entity
            .items
            .get(slot)
            .and_then(Option::as_ref)
            .cloned()
        else {
            continue;
        };
        let input_value = emc::value(&input.item);
        if input_value == 0 {
            continue;
        }
        let Ok(plan) = condenser::plan_input_conversion(state, spec, input_value, input.count)
        else {
            continue;
        };
        let consumed = plan.consumed_items;
        if consumed == 0 {
            continue;
        }
        let Ok(settled) = condenser::settle_input_conversion(plan, consumed) else {
            continue;
        };
        let remaining = input.count - consumed;
        event.entity.items[slot] = (remaining > 0).then_some(ItemStack {
            count: remaining,
            ..input
        });
        state = settled.next_state;
        break;
    }

    let max_stack = host.max_stack_size(&target.item).unwrap_or(64).max(1);
    let output_room = match event.entity.items.get(2).and_then(Option::as_ref) {
        None => max_stack,
        Some(output) if same_item_payload(output, &target) => {
            max_stack.saturating_sub(output.count)
        }
        Some(_) => 0,
    };
    if let Ok(plan) = condenser::plan_output_production(state, spec, target_value, output_room) {
        let requested = plan.produced_items;
        if requested > 0 {
            let inserted = match event.entity.items.get(2).and_then(Option::as_ref).cloned() {
                None => {
                    let mut output = target.clone();
                    output.count = requested;
                    event.entity.items[2] = Some(output);
                    requested
                }
                Some(mut output) if same_item_payload(&output, &target) => {
                    if let Some(count) = output.count.checked_add(requested) {
                        output.count = count;
                        event.entity.items[2] = Some(output);
                        requested
                    } else {
                        0
                    }
                }
                Some(_) => 0,
            };
            if let Ok(settled) = condenser::settle_output_production(plan, inserted) {
                state = settled.next_state;
            }
        }
    }

    write_emc(
        &mut event.entity.nbt,
        i64::try_from(state.stored_emc).unwrap_or(i64::MAX),
    );
    event.entity.data = furnace_data(state.stored_emc as i64, capacity);
}

pub(crate) fn tick(
    host: &mut dyn Host,
    event: &mut horizon_client::mod_api::BlockEntityTickEvent<'_>,
) {
    if event.entity.id != BLOCK_ENTITY {
        return;
    }
    let Some(machine) = Machine::of(&host.block(event.pos)) else {
        return;
    };
    if !matches!(machine, Machine::Condenser(_))
        && !migrate_legacy_buffer(host, event.pos, &mut event.entity.nbt, machine.capacity())
    {
        return;
    }
    let required_slots = if matches!(machine, Machine::Condenser(_)) {
        9
    } else {
        3
    };
    if event.entity.items.len() < required_slots {
        event.entity.items.resize(required_slots, None);
    }
    match machine {
        Machine::Collector(tier) => collector_tick(host, event.pos, event, tier),
        Machine::Relay(tier) => relay_tick(host, event.pos, event, tier),
        Machine::Condenser(spec) => condenser_tick(host, event, spec),
    }
}

pub(crate) fn collect(host: &mut dyn Host, pos: [i32; 3], player: u128) {
    let Some(machine) = Machine::of(&host.block(pos)) else {
        return;
    };
    if matches!(machine, Machine::Condenser(_)) {
        return;
    }
    let mut collected = 0;
    host.change_mod_block_entity(pos, &mut |entity| {
        if entity.id != BLOCK_ENTITY {
            return;
        }
        collected = read_emc(&entity.nbt).min(machine.capacity());
        if collected > 0 {
            write_emc(&mut entity.nbt, 0);
        }
    });
    if collected > 0 {
        match data::try_add_emc(host, player, collected) {
            Ok(total) => {
                host.tell(
                    player,
                    Text::literal(format!(
                        "§dCollected §e{collected} EMC§d. Balance: §e{total}§d."
                    )),
                );
            }
            Err(_) => {
                host.change_mod_block_entity(pos, &mut |entity| {
                    if entity.id == BLOCK_ENTITY {
                        let restored = read_emc(&entity.nbt)
                            .saturating_add(collected)
                            .min(machine.capacity());
                        write_emc(&mut entity.nbt, restored);
                    }
                });
                host.tell(
                    player,
                    Text::literal(
                        "§cCould not save your EMC balance; machine buffer was restored.",
                    ),
                );
            }
        }
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
    fn all_machine_tiers_have_expected_capacities_and_rates() {
        assert_eq!(Machine::of(COLLECTORS[0]), Some(Machine::Collector(0)));
        assert_eq!(Machine::of(COLLECTORS[1]), Some(Machine::Collector(1)));
        assert_eq!(Machine::of(COLLECTORS[2]), Some(Machine::Collector(2)));
        assert_eq!(Machine::of(RELAYS[0]), Some(Machine::Relay(0)));
        assert_eq!(Machine::of(RELAYS[1]), Some(Machine::Relay(1)));
        assert_eq!(Machine::of(RELAYS[2]), Some(Machine::Relay(2)));
        assert_eq!(Machine::Collector(0).capacity(), 10_000);
        assert_eq!(Machine::Collector(2).capacity(), 60_000);
        assert_eq!(Machine::Relay(0).capacity(), 100_000);
        assert_eq!(Machine::Relay(2).capacity(), 10_000_000);
        assert_eq!(Machine::Relay(2).transfer(), 640);
    }

    #[test]
    fn collector_generation_keeps_fractional_emc() {
        let (whole, micro) = collector_generation(4, 16, 0);
        assert_eq!((whole, micro), (0, 200_000));
        let (whole, micro) = collector_generation(4, 16, micro);
        assert_eq!((whole, micro), (0, 400_000));
        assert_eq!(collector_generation(4, 16, 800_000), (1, 0));
    }

    #[test]
    fn sun_level_obeys_open_sky_daylight_and_light_fallback() {
        assert_eq!(collector_sun_level(true, true, 0), 16);
        assert_eq!(collector_sun_level(false, true, 15), 16);
        assert_eq!(collector_sun_level(false, false, 0), 1);
        assert_eq!(collector_sun_level(false, false, 8), 9);
    }

    #[test]
    fn collector_and_relay_feed_condensers_but_condensers_never_forward() {
        let collector = Machine::of(COLLECTORS[0]).unwrap();
        let relay = Machine::of(RELAYS[0]).unwrap();
        let condenser = Machine::of(condenser::ENERGY_CONDENSER).unwrap();
        assert!(collector.feeds(condenser, 0.0, 0.0));
        assert!(relay.feeds(condenser, 0.0, 0.0));
        assert!(!condenser.feeds(relay, 1.0, 0.0));
        assert!(!condenser.feeds(collector, 1.0, 0.0));
        assert!(!condenser.feeds(condenser, 1.0, 0.0));
    }

    #[test]
    fn relay_to_relay_still_requires_a_strictly_less_full_target() {
        let relay = Machine::of(RELAYS[0]).unwrap();
        assert!(relay.feeds(relay, 0.8, 0.2));
        assert!(!relay.feeds(relay, 0.2, 0.8));
        assert!(!relay.feeds(relay, 0.5, 0.5));
    }

    #[test]
    fn share_redistributes_unused_capacity_and_never_exceeds_offer() {
        assert_eq!(share(100, &[20, 80]), vec![20, 80]);
        assert_eq!(share(10, &[]), Vec::<i64>::new());
        assert_eq!(share(1, &[20, 80]), vec![1, 0]);
        assert_eq!(share(200, &[20, 30]), vec![20, 30]);
    }
}
