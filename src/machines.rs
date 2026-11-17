//! EMC-producing and EMC-routing ProjectE machines.
//!
//! EMC is deliberately *not* mapped onto the SDK's `i32` Forge-Energy buffer:
//! ProjectE's storage is a server-authoritative `i64` with its own
//! force-insert/force-extract semantics. Machine buffers therefore live in the
//! block entity's saved compound, read and written through the generated SDK.

use mod_api::*;

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

pub(crate) const BLOCK_ENTITY: &str = "projecte:machine";
const EMC_KEY: &str = "emc";
const MICRO_KEY: &str = "micro";
const BONUS_KEY: &str = "bonus";
const MICRO_PER_EMC: i64 = 1_000_000;

const COLLECTOR_CAPACITY: [i64; 3] = [10_000, 30_000, 60_000];
const COLLECTOR_RATE_PER_SECOND: [i64; 3] = [4, 12, 40];
const RELAY_CAPACITY: [i64; 3] = [100_000, 1_000_000, 10_000_000];
const TRANSFER_LIMIT: [i64; 3] = [64, 192, 640];
const RELAY_BONUS_PER_TICK_MILLI: [i64; 3] = [50, 150, 500];

/// ProjectE's covalence loss: a sell-value multiplier in `0.1..=1.0`.
/// The server config override is not wired yet; these are the reference
/// defaults, kept as pure functions so the arithmetic is testable.
pub(crate) const COVALENCE_DEFAULT: f64 = 1.0;
pub(crate) const COVALENCE_MIN: f64 = 0.1;
pub(crate) const COVALENCE_MAX: f64 = 1.0;
pub(crate) const COVALENCE_ROUND_DEFAULT: bool = true;

pub(crate) fn covalence_loss(configured: f64) -> f64 {
    if configured.is_nan() {
        COVALENCE_DEFAULT
    } else {
        configured.clamp(COVALENCE_MIN, COVALENCE_MAX)
    }
}

/// Sell value after covalence: `floor(value * loss)`, raised to 1 when a
/// non-zero item would otherwise round down to nothing.
pub(crate) fn sell_value(base: u64, configured: f64, rounding: bool) -> u64 {
    if base == 0 {
        return 0;
    }
    let scaled = (base as f64 * covalence_loss(configured)).floor();
    if scaled < 1.0 && rounding {
        1
    } else {
        scaled as u64
    }
}

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
        if block == condenser::ENERGY_CONDENSER_MK2 {
            return Some(Self::Condenser(condenser::MK2_SPEC));
        }
        if block == condenser::ENERGY_CONDENSER_MK3 {
            return Some(Self::Condenser(condenser::MK3_SPEC));
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

    fn feeds(self, target: Self, _source_fill: f64, _target_fill: f64) -> bool {
        match (self, target) {
            (Self::Collector(_), Self::Relay(_) | Self::Condenser(_)) => true,
            // ProjectE suppresses relay-to-relay pushes entirely to prevent
            // two relays from trading the same EMC back and forth forever.
            (Self::Relay(_), Self::Relay(_)) => false,
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

fn slots(first: usize, last: usize) -> Vec<usize> {
    (first..last).collect()
}

fn faces(horizontal: Vec<usize>, vertical: Vec<usize>) -> [Vec<usize>; 6] {
    // Direction order: down, up, north, south, west, east.
    [
        vertical.clone(),
        vertical,
        horizontal.clone(),
        horizontal.clone(),
        horizontal.clone(),
        horizontal,
    ]
}

fn container_of(
    slots_count: usize,
    inputs: Vec<usize>,
    outputs: Vec<usize>,
    face_sets: [Vec<usize>; 6],
    menu: &str,
) -> ModContainer {
    ModContainer {
        slots: slots_count,
        faces: face_sets,
        inputs,
        outputs,
        menu: Some(menu.to_owned()),
    }
}

pub(crate) fn container(block: &str) -> Option<ModContainer> {
    if block == ALCHEMICAL_CHEST {
        // ProjectE: a 104-slot (8 x 13) chest, inventory written at NBT root.
        let all = slots(0, 104);
        return Some(container_of(104, all.clone(), all.clone(), faces(all, Vec::new()), "projecte:alchemical_chest"));
    }
    let machine = Machine::of(block)?;
    match machine {
        Machine::Condenser(spec) if spec.capacity == condenser::MK3_SPEC.capacity => {
            // MK3's reference has two menus: 91 input slots and a separate
            // 180-slot output (the SDK caps one menu at 256 slots). The block's
            // primary container is the input side; the output menu id is
            // exported for a face-specific open in `lib.rs`.
            let input = slots(0, condenser::MK3_INPUT_SLOTS);
            Some(container_of(
                condenser::MK3_INPUT_SLOTS,
                input.clone(),
                Vec::new(),
                faces(input, Vec::new()),
                condenser::CONDENSER_MK3_INPUT_MENU,
            ))
        }
        Machine::Condenser(spec) if spec.capacity == condenser::MK2_SPEC.capacity => {
            // MK2: 42 input + 42 output slots; the output side is take-only.
            let input = slots(0, 42);
            let output = slots(42, 84);
            Some(container_of(
                84,
                input.clone(),
                output.clone(),
                faces(input, output),
                condenser::ENERGY_CONDENSER_MK2,
            ))
        }
        Machine::Condenser(_) => {
            // MK1 uses one 91-slot handler for both input and output.
            let all = slots(0, 91);
            Some(container_of(91, all.clone(), all.clone(), faces(all.clone(), all), condenser::ENERGY_CONDENSER))
        }
        Machine::Collector(tier) => {
            let (count, input_count) = [(11, 8), (15, 12), (19, 16)][tier];
            let input = slots(0, input_count);
            // Aux slots: upgrading (input_count), upgrade result, ghost lock.
            let result = vec![input_count + 1];
            Some(container_of(
                count,
                input.clone(),
                result.clone(),
                faces(input, result),
                match tier {
                    0 => "projecte:collector_mk1",
                    1 => "projecte:collector_mk2",
                    _ => "projecte:collector_mk3",
                },
            ))
        }
        Machine::Relay(tier) => {
            let (count, input_count) = [(8, 7), (14, 13), (22, 21)][tier];
            let input = slots(0, input_count);
            let output = slots(input_count, count);
            Some(container_of(
                count,
                input.clone(),
                output.clone(),
                faces(input, output),
                match tier {
                    0 => "projecte:relay_mk1",
                    1 => "projecte:relay_mk2",
                    _ => "projecte:relay_mk3",
                },
            ))
        }
    }
}

/// This is intentionally `None`: EMC is an `i64` with custom semantics, not
/// the SDK's `i32` Forge-Energy buffer. Kept as a registration hook so the
/// central `Hooks::register` can stay uniform across all machine blocks.
pub(crate) fn energy_buffer(_block: &str) -> Option<ModEnergyBuffer> {
    None
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

fn migrate_legacy_buffer(host: &mut dyn ServerHost, pos: [i32; 3], nbt: &mut Compound, max: i64) -> bool {
    if nbt.get(&EMC_KEY.into()).is_some() {
        return true;
    }
    let legacy = match data::BUFFER.get(host, Holder::BlockEntity(pos)) {
        Ok(value) => value.clamp(0, max),
        Err(_) => return false,
    };
    write_emc(nbt, legacy);
    if data::BUFFER.set(host, Holder::BlockEntity(pos), 0).is_err() {
        return false;
    }
    true
}

fn collector_sun_level(sky_open: bool, daytime: bool, light: u8) -> u8 {
    if sky_open && daytime {
        16
    } else {
        light.min(15).saturating_add(1).min(16)
    }
}

/// The level that actually contains `pos`. A tick's event has no level handle,
/// so ask the game which of its open levels answers for this position.
fn level_at(host: &dyn Host, pos: [i32; 3]) -> Option<LevelRef> {
    let level = host.level("overworld")?;
    level.block(host, pos).is_some().then_some(level)
}

fn block_id(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> String {
    level
        .block(host, pos)
        .map(|key| key.id(host))
        .unwrap_or_default()
}

fn item_id(_host: &dyn Host, stack: &ItemStack) -> String {
    stack.item().id(_host)
}

fn collector_sun(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> u8 {
    let above = [pos[0], pos[1] + 1, pos[2]];
    let block_light = level
        .brightness(host, LightLayer::Block, above)
        .unwrap_or(0)
        .clamp(0, 15) as u8;
    let sky_light = level
        .brightness(host, LightLayer::Sky, above)
        .unwrap_or(0)
        .clamp(0, 15) as u8;
    let time = level.day_time(host).unwrap_or(0).rem_euclid(24_000);
    let raining = level.weather(host).map(|weather| weather.0 >= 0.2).unwrap_or(false);
    let daytime = !(12_300..=23_850).contains(&time) && !raining;
    let sky_open = level.can_see_sky(host, above).unwrap_or(false);
    collector_sun_level(
        sky_open,
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

/// Split an EMC offer evenly across the eligible neighbours.
///
/// ProjectE's `EmcBlockEntity.sendToAllAcceptors` gives every acceptor
/// `emc / targetCount` and leaves the remainder in the source; it does not
/// redistribute unused capacity. Keeping that observable behaviour matters
/// for parity even though a fairer split is tempting.
fn share(offer: i64, room: &[i64]) -> Vec<i64> {
    let active: Vec<usize> = room
        .iter()
        .enumerate()
        .filter_map(|(index, available)| ((*available).max(0) > 0).then_some(index))
        .collect();
    if active.is_empty() {
        return vec![0; room.len()];
    }
    let each = offer.max(0) / active.len() as i64;
    let mut allocations = vec![0; room.len()];
    for index in active {
        allocations[index] = each.min(room[index].max(0));
    }
    allocations
}

fn push_emc(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    source: Machine,
    stored: i64,
    offer: i64,
) -> i64 {
    let source_fill = source.fill(stored);
    let mut targets = Vec::new();
    let mut rooms = Vec::new();

    for face in FACES {
        let target_pos = [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]];
        let target_id = block_id(host, level, target_pos);
        let Some(target) = Machine::of(&target_id) else {
            continue;
        };
        let Some(entity) = host.block_entity(level, target_pos) else {
            continue;
        };
        let Some(nbt) = entity.data(host) else {
            continue;
        };
        let target_emc = read_emc(&nbt).min(target.capacity());
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
        let Some(entity) = host.block_entity(level, target_pos) else {
            continue;
        };
        let Some(nbt) = entity.data(host) else {
            continue;
        };
        let current = read_emc(&nbt).min(target.capacity());
        let moved = allocation.min(target.capacity() - current).max(0);
        if moved > 0 {
            let mut patch = Compound::new();
            write_emc(&mut patch, current + moved);
            if entity.merge_data(host, patch).unwrap_or(false) {
                total += moved;
            }
        }
    }
    total
}

fn take_one(slot: &mut Option<ItemStack>) {
    if let Some(stack) = slot.as_mut() {
        let left = stack.count().saturating_sub(1);
        if left == 0 {
            *slot = None;
        } else if stack.set_count(left).is_ok() {
            // the same stack now carries the reduced count
        } else {
            *slot = None;
        }
    }
}

fn move_full_star(entity: &mut mod_api::ModBlockEntityView<'_>, input_slot: usize) {
    if input_slot >= entity.items.len() || entity.items[input_slot].is_none() {
        return;
    }
    if entity.items.get(2).is_some_and(Option::is_some) {
        return;
    }
    entity.items[2] = entity.items[input_slot].take();
}

fn star_fill(host: &dyn Host, stack: Option<&ItemStack>) -> i32 {
    let Some(stack) = stack else {
        return 0;
    };
    let Some(capacity) = energy::capacity(&item_id(host, stack)) else {
        return 0;
    };
    scaled(energy::stored_emc(stack).unwrap_or(0), capacity)
}

fn scaled(current: i64, maximum: i64) -> i32 {
    if maximum <= 0 {
        0
    } else {
        (current.clamp(0, maximum) as i128 * 1000 / maximum as i128) as i32
    }
}

/// Collector data slots: EMC fill, sky/sun level, Klein Star charge, fuel.
fn collector_data(stored: i64, capacity: i64, sun: u8, star: i32, fuel: i32) -> Vec<i32> {
    vec![scaled(stored, capacity), i32::from(sun), star, fuel]
}

/// Relay data slots: EMC fill, Klein Star charge, burn-slot progress.
fn relay_data(stored: i64, capacity: i64, star: i32, burn: i32) -> Vec<i32> {
    vec![scaled(stored, capacity), star, burn]
}

/// Condenser data slots: display EMC and required EMC, both relative to the buffer.
fn condenser_data(stored: i64, required: i64, capacity: i64) -> Vec<i32> {
    vec![scaled(stored, capacity), scaled(required, capacity)]
}

fn collector_tick(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    event: &mut mod_api::BlockEntityTickEvent<'_>,
    tier: usize,
) {
    let machine = Machine::Collector(tier);
    let capacity = machine.capacity();
    let mut stored = read_emc(&event.entity.nbt).min(capacity);
    let micro = read_long(&event.entity.nbt, MICRO_KEY);
    let sun = collector_sun(host, level, pos);
    let (generated, next_micro) = collector_generation(COLLECTOR_RATE_PER_SECOND[tier], sun, micro);
    stored = stored.saturating_add(generated).min(capacity);

    for slot in 0..2 {
        let Some(mut star) = event.entity.items[slot].clone() else {
            continue;
        };
        if energy::capacity(&item_id(host, &star)).is_none() {
            continue;
        }
        let star_capacity = energy::capacity(&item_id(host, &star)).unwrap_or(0);
        stored -= energy::charge(&mut star, stored.min(machine.transfer()), star_capacity);
        let full = energy::stored_emc(&star).unwrap_or(0) >= star_capacity;
        if full && event.entity.items[2].is_none() {
            event.entity.items[2] = Some(star);
            event.entity.items[slot] = None;
        } else {
            event.entity.items[slot] = Some(star);
        }
    }

    if stored > 0 {
        stored -= push_emc(host, level, pos, machine, stored, stored.min(machine.transfer()));
    }
    write_emc(&mut event.entity.nbt, stored);
    write_long(&mut event.entity.nbt, MICRO_KEY, next_micro);
    event.entity.data = collector_data(
        stored,
        capacity,
        sun,
        star_fill(host, event.entity.items.first().and_then(Option::as_ref)),
        0,
    );
}

fn relay_tick(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    event: &mut mod_api::BlockEntityTickEvent<'_>,
    tier: usize,
) {
    let machine = Machine::Relay(tier);
    let capacity = machine.capacity();
    let mut stored = read_emc(&event.entity.nbt).min(capacity);

    if let Some(mut input) = event.entity.items[0].clone() {
        if energy::capacity(&item_id(host, &input)).is_some() {
            let moved = energy::discharge(&mut input, (capacity - stored).min(machine.transfer()));
            stored += moved;
            event.entity.items[0] = Some(input);
        } else {
            let value = sell_value(
                emc::value(&item_id(host, &input)),
                COVALENCE_DEFAULT,
                COVALENCE_ROUND_DEFAULT,
            );
            if value > 0 && value <= (capacity - stored) as u64 {
                stored += value as i64;
                take_one(&mut event.entity.items[0]);
            }
        }
    }

    if let Some(mut output) = event.entity.items[1].clone()
        && energy::capacity(&item_id(host, &output)).is_some()
    {
        let output_capacity = energy::capacity(&item_id(host, &output)).unwrap_or(0);
        stored -= energy::charge(&mut output, stored.min(machine.transfer()), output_capacity);
        event.entity.items[1] = Some(output);
        let full = event.entity.items[1].as_ref().is_some_and(|stack| {
            energy::stored_emc(stack).unwrap_or(0)
                >= energy::capacity(&item_id(host, stack)).unwrap_or(i64::MAX)
        });
        if full && event.entity.items[2].is_none() {
            move_full_star(&mut event.entity, 1);
        }
    }

    if stored > 0 {
        let sent = push_emc(host, level, pos, machine, stored, stored.min(machine.transfer()));
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
    event.entity.data = relay_data(
        stored,
        capacity,
        star_fill(host, event.entity.items.get(1).and_then(Option::as_ref)),
        scaled(read_long(&event.entity.nbt, BONUS_KEY).clamp(0, 999), 1000),
    );
}

fn same_item_payload(_host: &dyn Host, left: &ItemStack, right: &ItemStack) -> bool {
    left.item() == right.item()
        && left.damage() == right.damage()
        && left.components() == right.components()
}

/// One server-authoritative Condenser step. Slot 1 is the non-consumed target
/// template, slot 2 is output, and slots 0/3..8 accept EMC-valued input.
fn condenser_tick(
    host: &mut dyn ServerHost,
    event: &mut mod_api::BlockEntityTickEvent<'_>,
    spec: condenser::CondenserSpec,
) {
    let capacity = i64::try_from(spec.capacity).unwrap_or(i64::MAX);
    let stored = read_emc(&event.entity.nbt).clamp(0, capacity);
    let mut state = condenser::CondenserState {
        stored_emc: stored as u64,
    };

    let Some(target) = event.entity.items.get(1).and_then(Option::as_ref).cloned() else {
        write_emc(&mut event.entity.nbt, stored);
        event.entity.data = condenser_data(stored, 0, capacity);
        return;
    };
    let target_id = item_id(host, &target);
    let target_value = emc::value(&target_id);
    if target_value == 0 {
        write_emc(&mut event.entity.nbt, stored);
        event.entity.data = condenser_data(stored, 0, capacity);
        return;
    }

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
        let input_value = sell_value(
            emc::value(&item_id(host, &input)),
            COVALENCE_DEFAULT,
            COVALENCE_ROUND_DEFAULT,
        );
        if input_value == 0 {
            continue;
        }
        let Ok(plan) = condenser::plan_input_conversion(state, spec, input_value, input.count()) else {
            continue;
        };
        let consumed = plan.consumed_items;
        if consumed == 0 {
            continue;
        }
        let Ok(settled) = condenser::settle_input_conversion(plan, consumed) else {
            continue;
        };
        let remaining = input.count().saturating_sub(consumed);
        event.entity.items[slot] = if remaining > 0 {
            let mut rest = input;
            if rest.set_count(remaining).is_ok() { Some(rest) } else { None }
        } else {
            None
        };
        state = settled.next_state;
        break;
    }

    let max_stack = host.max_stack_size(target.item()).unwrap_or(64).max(1);
    let output_room = match event.entity.items.get(2).and_then(Option::as_ref) {
        None => max_stack,
        Some(output) if same_item_payload(host, output, &target) => {
            max_stack.saturating_sub(output.count())
        }
        Some(_) => 0,
    };
    if let Ok(plan) = condenser::plan_output_production(state, spec, target_value, output_room) {
        let requested = plan.produced_items;
        if requested > 0 {
            let inserted = match event.entity.items.get(2).and_then(Option::as_ref).cloned() {
                None => {
                    let mut output = target.clone();
                    if output.set_count(requested).is_ok() {
                        event.entity.items[2] = Some(output);
                        requested
                    } else {
                        0
                    }
                }
                Some(mut output) if same_item_payload(host, &output, &target) => {
                    if let Some(count) = output.count().checked_add(requested)
                        && output.set_count(count).is_ok()
                    {
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
    event.entity.data = condenser_data(state.stored_emc as i64, target_value as i64, capacity);
}

pub(crate) fn tick(host: &mut dyn ServerHost, event: &mut mod_api::BlockEntityTickEvent<'_>) {
    if event.entity.id() != BLOCK_ENTITY {
        return;
    }
    let Some(level) = level_at(host, event.pos) else {
        return;
    };
    let Some(machine) = Machine::of(&block_id(host, level, event.pos)) else {
        return;
    };
    if !matches!(machine, Machine::Condenser(_))
        && !migrate_legacy_buffer(host, event.pos, &mut event.entity.nbt, machine.capacity())
    {
        return;
    }
    let required_slots = if matches!(machine, Machine::Condenser(_)) { 9 } else { 3 };
    if event.entity.items.len() < required_slots {
        event.entity.items.resize(required_slots, None);
    }
    match machine {
        Machine::Collector(tier) => collector_tick(host, level, event.pos, event, tier),
        Machine::Relay(tier) => relay_tick(host, level, event.pos, event, tier),
        Machine::Condenser(spec) => condenser_tick(host, event, spec),
    }
}

pub(crate) fn collect(host: &mut dyn ServerHost, pos: [i32; 3], player: u128) {
    let Some(level) = level_at(host, pos) else {
        return;
    };
    let Some(machine) = Machine::of(&block_id(host, level, pos)) else {
        return;
    };
    if matches!(machine, Machine::Condenser(_)) {
        return;
    }
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    let Some(nbt) = entity.data(host) else {
        return;
    };
    let collected = read_emc(&nbt).min(machine.capacity());
    if collected == 0 {
        if let Some(subject) = host.player(player) {
            let _ = subject.tell(host, Text::literal("§7This machine has not collected any EMC yet."));
        }
        return;
    }
    match data::try_add_emc(host, player, collected) {
        Ok(total) => {
            let mut patch = Compound::new();
            write_emc(&mut patch, 0);
            if entity.merge_data(host, patch).unwrap_or(false) {
                if let Some(subject) = host.player(player) {
                    let _ = subject.tell(host, Text::literal(format!(
                        "§dCollected §e{collected} EMC§d. Balance: §e{total}§d."
                    )));
                }
            }
        }
        Err(_) => {
            if let Some(subject) = host.player(player) {
                let _ = subject.tell(host, Text::literal(
                    "§cCould not save your EMC balance; machine buffer was kept.",
                ));
            }
        }
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
    fn relay_to_relay_is_suppressed_to_prevent_emc_thrashing() {
        let relay = Machine::of(RELAYS[0]).unwrap();
        assert!(!relay.feeds(relay, 0.8, 0.2));
        assert!(!relay.feeds(relay, 0.2, 0.8));
    }

    #[test]
    fn covalence_loss_is_clamped_and_sell_values_never_round_a_valued_item_to_zero() {
        assert_eq!(covalence_loss(f64::NAN), COVALENCE_DEFAULT);
        assert_eq!(covalence_loss(0.0), COVALENCE_MIN);
        assert_eq!(covalence_loss(2.0), COVALENCE_MAX);
        assert_eq!(covalence_loss(0.5), 0.5);
        assert_eq!(sell_value(8192, 1.0, true), 8192);
        assert_eq!(sell_value(8192, 0.5, true), 4096);
        assert_eq!(sell_value(1, 0.1, true), 1);
        assert_eq!(sell_value(1, 0.1, false), 0);
        assert_eq!(sell_value(0, 1.0, true), 0);
    }

    #[test]
    fn machine_containers_match_the_reference_slot_counts() {
        for (block, count, menu, outputs) in [
            (COLLECTORS[0], 11, "projecte:collector_mk1", vec![9]),
            (COLLECTORS[1], 15, "projecte:collector_mk2", vec![13]),
            (COLLECTORS[2], 19, "projecte:collector_mk3", vec![17]),
            (RELAYS[0], 8, "projecte:relay_mk1", vec![7]),
            (RELAYS[1], 14, "projecte:relay_mk2", vec![13]),
            (RELAYS[2], 22, "projecte:relay_mk3", vec![21]),
            (condenser::ENERGY_CONDENSER, 91, "projecte:energy_condenser", (0..91).collect()),
            (condenser::ENERGY_CONDENSER_MK2, 84, condenser::ENERGY_CONDENSER_MK2, (42..84).collect()),
            (
                condenser::ENERGY_CONDENSER_MK3,
                condenser::MK3_INPUT_SLOTS,
                condenser::CONDENSER_MK3_INPUT_MENU,
                Vec::new(),
            ),
            (ALCHEMICAL_CHEST, 104, "projecte:alchemical_chest", (0..104).collect()),
        ] {
            let container = container(block).unwrap_or_else(|| panic!("no container for {block}"));
            assert_eq!(container.slots, count, "{block}");
            assert_eq!(container.menu.as_deref(), Some(menu), "{block}");
            assert_eq!(container.outputs, outputs, "{block}");
            assert_eq!(container.faces.len(), 6);
        }
    }

    #[test]
    fn condenser_mk1_shares_one_handler_between_input_and_output() {
        let mk1 = container(condenser::ENERGY_CONDENSER).unwrap();
        assert_eq!(mk1.inputs, mk1.outputs);
        let mk2 = container(condenser::ENERGY_CONDENSER_MK2).unwrap();
        assert_eq!(mk2.inputs, (0..42).collect::<Vec<_>>());
        assert_eq!(mk2.outputs, (42..84).collect::<Vec<_>>());
    }

    #[test]
    fn share_splits_evenly_and_leaves_the_remainder_in_the_source() {
        assert_eq!(share(100, &[20, 80]), vec![20, 50]);
        assert_eq!(share(10, &[]), Vec::<i64>::new());
        assert_eq!(share(1, &[20, 80]), vec![0, 0]);
        assert_eq!(share(200, &[20, 30]), vec![20, 30]);
        assert_eq!(share(90, &[10, 50, 50]), vec![10, 30, 30]);
    }
}
