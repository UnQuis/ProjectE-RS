//! ProjectE-RS: Equivalent Exchange for Horizon.
//!
//! Native mod against the generated `mod_api` (API 0.2). The crate root holds
//! the one `#[horizon_mod] impl Hooks`; per-object moments live in Parts.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use mod_api::*;

/// Set by `data_maps_updated`; consumed on the next `server_tick` when the
/// game has left the Reloading phase and `host.players()` is available.
static PRUNE_PENDING: AtomicBool = AtomicBool::new(false);

mod alchemical;
pub mod condenser;
mod config;
mod data;
mod deposit;
mod deposit_menu;
mod emc;
mod energy;
mod machines;
mod menus;
mod progression;
mod recipes;
mod transaction;
mod transmutation;
#[allow(dead_code)] // The UI is a deliberate shell until its gameplay hooks are wired.
mod ui;

const TABLE: &str = recipes::TABLE;
const PHILOSOPHERS_STONE: &str = alchemical::PHILOSOPHERS_STONE;
const PROJECTE_MENU_SLOTS: usize = ui::table::TABLE_SLOTS;
const TABLE_BLOCK_ENTITY: &str = "projecte:transmutation_table";


#[derive(Default)]
struct ProjectE;

#[mod_api::horizon_mod]
impl Hooks for ProjectE {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        // The table and every machine share one Part type; the game dispatches
        // a Part only for the block entities it is attached to.
        let table = r.block_entity_type("transmutation_table").blocks(&[TABLE]).build();
        r.block_entity_part(&table, ProjectEBlockEntity)
            .expect("table block entity part");

        let table_block = r.block_key(TABLE);
        r.block_container(&table_block, table_container());

        for id in machines::COLLECTORS
            .into_iter()
            .chain(machines::RELAYS)
            .chain([
                condenser::ENERGY_CONDENSER,
                condenser::ENERGY_CONDENSER_MK2,
                machines::ALCHEMICAL_CHEST,
            ])
        {
            let block = r.block_key(id);
            if let Some(container) = machines::container(id) {
                r.block_container(&block, container);
            }
            if let Some(buffer) = machines::energy_buffer(id) {
                r.block_energy(&block, buffer);
            }
            let kind = r
                .block_entity_type(id.rsplit(':').next().unwrap_or(id))
                .blocks(&[id])
                .build();
            r.block_entity_part(&kind, ProjectEBlockEntity)
                .expect("machine block entity part");
        }

        let stone = r.item_key(PHILOSOPHERS_STONE);
        r.item_part(stone, StonePart);
        r.menu_part(ui::atlas::MENU, TableMenuPart);
    }

    /// The EMC data map was rebuilt on a server reload. Defer pruning to the
    /// next server tick: `host.players()` is not available during the Reloading
    /// phase in which this hook runs, and calling it panics the server.
    fn data_maps_updated(&self, _host: HostSide<'_>, event: &mut DataMapsUpdatedEvent<'_>) {
        if event.registry != "minecraft:item" || event.cause != DataMapsUpdateCause::ServerReload {
            return;
        }
        PRUNE_PENDING.store(true, Ordering::Release);
    }

    fn server_tick(&self, host: &mut dyn ServerHost, _event: &mut ServerTickEvent<'_>) {
        if !PRUNE_PENDING.swap(false, Ordering::AcqRel) {
            return;
        }
        for player in host.players().server_players() {
            let uuid = player.uuid();
            let Ok(mut data) = data::get(host, uuid) else {
                eprintln!("[projecte] could not read player data for {uuid} while pruning knowledge");
                continue;
            };
            let (retained, removed) = data::prune_known(&data.learned, |id| emc_value(host, id) > 0);
            if removed.is_empty() {
                continue;
            }
            let count = removed.len();
            data.learned = retained;
            if let Err(error) = data::set(host, uuid, data) {
                eprintln!("[projecte] could not save pruned knowledge for {uuid}: {error:?}");
                continue;
            }
            say(host, uuid, Text::literal(format!(
                "§7ProjectE: removed §e{count}§7 item(s) with no EMC value."
            )));
        }
    }

    fn register_configs(&self, _host: HostSide<'_>, event: &mut RegisterConfigsEvent<'_>) {
        config::register(event);
    }

    fn config_loading(&self, _host: HostSide<'_>, event: &mut ConfigLoadingEvent<'_>) {
        config::loading(event.config);
    }

    fn config_reloading(&self, _host: HostSide<'_>, event: &mut ConfigReloadingEvent<'_>) {
        config::loading(event.config);
    }

    fn data_map_types(&self, _host: &mut dyn ServerHost, event: &mut DataMapTypesEvent<'_>) {
        if !event
            .types
            .iter()
            .any(|kind| kind.id == emc::EMC_MAP_ID && kind.registry == "minecraft:item")
        {
            event.types.push(emc::emc_data_map_type());
        }
    }

    fn blocks(&self, _context: &RegistrationContext, event: &mut BlocksEvent<'_>) {
        block(event, TABLE, "minecraft:crafting_table");
        for id in machines::COLLECTORS
            .into_iter()
            .chain(machines::RELAYS)
            .chain([condenser::ENERGY_CONDENSER, condenser::ENERGY_CONDENSER_MK2])
        {
            block(event, id, "minecraft:furnace");
        }
        block(event, machines::ALCHEMICAL_CHEST, "minecraft:chest");
        for (_, id) in alchemical::FUEL_BLOCKS {
            block(event, id, "minecraft:coal_block");
        }
    }

    fn items(&self, _context: &RegistrationContext, event: &mut ItemsEvent<'_>) {
        item(event, PHILOSOPHERS_STONE, "minecraft:diamond", 1);
        for id in energy::KLEIN_STARS {
            item(event, id, "minecraft:nether_star", 1);
        }
        item(event, "projecte:dark_matter", "minecraft:coal", 64);
        item(event, "projecte:red_matter", "minecraft:redstone", 64);
        item(event, "projecte:low_covalence_dust", "minecraft:sugar", 64);
        item(event, "projecte:medium_covalence_dust", "minecraft:redstone", 64);
        item(event, "projecte:high_covalence_dust", "minecraft:glowstone_dust", 64);
        item(event, data::TOME_ID, "minecraft:book", 1);
        let fuel_templates = [
            "minecraft:coal",
            "minecraft:blaze_powder",
            "minecraft:glowstone_dust",
        ];
        for (fuel, template) in alchemical::FUEL_TIERS.iter().skip(1).zip(fuel_templates) {
            item(event, fuel, template, 64);
        }
    }

    fn component_types(&self, _context: &RegistrationContext, event: &mut ComponentTypesEvent<'_>) {
        if !event.types.iter().any(|id| id == energy::STORED_EMC_COMPONENT) {
            event.types.push(energy::STORED_EMC_COMPONENT.to_string());
        }
    }

    fn data_pack_files(&self, _host: &mut dyn ServerHost, event: &mut DataPackFilesEvent<'_>) {
        event.files.extend(recipes::files());
        event.files.extend(emc::data_pack_files());
        event.files.extend(transmutation::data_pack_files());
        event.files.extend(menus::data_pack_files());
    }

    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::new("projecte emc [player:player]"));
        event.commands.push(Command::op(
            "projecte emc set <player:player> <amount:integer>",
            2,
        ));
        event.commands.push(Command::new("projecte learn [item:id]"));
        event.commands.push(Command::new(
            "projecte transmute <item:id> [count:integer]",
        ));
        event.commands.push(Command::new("projecte table"));
        event.commands.push(Command::new("projecte value <item:id>"));
        event.commands.push(Command::new("projecte reset"));
        event.commands.push(Command::new("projecte roadmap"));
        event.commands.push(Command::new("projecte deposit"));
        event.commands.push(Command::op("projecte knowledge all", 2));
        event.commands.push(Command::op("projecte knowledge clear", 2));
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        let player = match event.source {
            CommandSource::Player { player, .. } => Some(*player),
            _ => None,
        };
        if command(host, player, event.command) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        let pos = event.click.pos;
        let Some(level) = level_of(host, event.click.player) else {
            return Flow::Continue;
        };
        let Some(block) = level.block(host, pos) else {
            return Flow::Continue;
        };
        let id = block.id(host);
        if id == TABLE {
            return Flow::Continue;
        }
        if machines::is_collectable(&id) {
            let Some(acting) = host.players().acting_player() else {
                return Flow::Continue;
            };
            if !is_sneaking(host, acting.uuid()) {
                return Flow::Continue;
            }
            machines::collect(host, pos, acting.uuid());
            return Flow::Cancel;
        }
        Flow::Continue
    }

    fn item_crafted(&self, host: &mut dyn ServerHost, event: &mut ItemCraftedEvent<'_>) {
        if !recipes::is_catalyst_recipe(event.recipe) {
            return;
        }
        if let (Some(player), Some(stone)) =
            (host.player(event.player), stack(host, PHILOSOPHERS_STONE, 1))
        {
            if let Err(refusal) = player.give_stack(host, stone) {
                say(host, event.player, Text::literal(format!(
                    "§cYour Philosopher's Stone could not be returned: {refusal:?}."
                )));
            }
        }
    }

    fn menu_closed(&self, _host: &mut dyn ServerHost, event: &mut MenuClosedEvent) {
        deposit_menu::forget(event.uuid, event.window);
    }

    fn menu_clicked(&self, host: &mut dyn ServerHost, event: &mut MenuClickedEvent) {
        if deposit_menu::confirm_slot(event.uuid, event.window, event.slot).is_some() {
            let contents = deposit_menu::contents(host, event.uuid);
            let result = deposit_menu::apply(host, event.uuid, contents);
            deposit_menu::report(host, event.uuid, result);
            return;
        }
        if !(0..PROJECTE_MENU_SLOTS as i32).contains(&event.slot) {
            return;
        }
        let Ok(mut data) = data::get(host, event.uuid) else {
            say(host, event.uuid, Text::literal("§cYour ProjectE data could not be read."));
            return;
        };
        let item = match ui::table::slot_role(event.slot as usize) {
            ui::table::SlotRole::Input | ui::table::SlotRole::Player => return,
            ui::table::SlotRole::Lock => {
                let item = ui::atlas::menu_item(host, event.uuid, event.window, ui::table::LOCK_SLOT)
                    .map(|(id, _)| id);
                let next = match ui::table::plan_lock(
                    item.as_deref(),
                    &data,
                    |id: &str| emc_value(host, id),
                ) {
                    Ok(next) => next,
                    Err(message) => {
                        say(host, event.uuid, Text::literal(message));
                        return;
                    }
                };
                data.lock = next;
                if let Err(error) = data::set(host, event.uuid, data) {
                    say(host, event.uuid, Text::literal(format!("§c{error:?}")));
                }
                return;
            }
            ui::table::SlotRole::Consume => {
                let Some((item, count)) = ui::atlas::menu_item(
                    host, event.uuid, event.window, ui::table::CONSUME_SLOT,
                ) else {
                    say(host, event.uuid, Text::literal("§cThere is nothing in the consume slot."));
                    return;
                };
                let plan = match ui::table::plan_consume(&item, count, |id: &str| emc_value(host, id)) {
                    Ok(plan) => plan,
                    Err(message) => {
                        say(host, event.uuid, Text::literal(message));
                        return;
                    }
                };
                // Clear the slot before crediting: a failed removal must never
                // pay out for an item the player still holds.
                if !ui::server::clear_menu_slot(host, event.uuid, event.window, ui::table::CONSUME_SLOT) {
                    say(host, event.uuid, Text::literal("§cThe consume slot could not be cleared; nothing was credited."));
                    return;
                }
                if let Err(error) = data::add_emc(host, event.uuid, plan.credited_emc) {
                    say(host, event.uuid, Text::literal(format!("§c{error:?}")));
                    return;
                }
                if !ui::table::learn_for_consume(&mut data, &item, |id| emc_value(host, id) > 0) {
                    say(host, event.uuid, Text::literal(format!(
                        "§7{} was already known or cannot be learned.",
                        emc::display_name(&item)
                    )));
                }
                if let Err(error) = data::set(host, event.uuid, data) {
                    say(host, event.uuid, Text::literal(format!("§c{error:?}")));
                    return;
                }
                say(host, event.uuid, Text::literal(format!(
                    "§aConsumed 1x {} for §e{} EMC§a.",
                    emc::display_name(&item), plan.credited_emc
                )));
                return;
            }
            ui::table::SlotRole::Unlearn => {
                let Some((item, _)) = ui::atlas::menu_item(
                    host, event.uuid, event.window, ui::table::UNLEARN_SLOT,
                ) else {
                    say(host, event.uuid, Text::literal("§cThere is nothing in the unlearn slot."));
                    return;
                };
                if let Err(message) = ui::table::plan_unlearn(
                    &item,
                    &data,
                    |id: &str| emc_value(host, id),
                ) {
                    say(host, event.uuid, Text::literal(message));
                    return;
                }
                if !ui::server::clear_menu_slot(host, event.uuid, event.window, ui::table::UNLEARN_SLOT) {
                    say(host, event.uuid, Text::literal("§cThe unlearn slot could not be cleared; nothing was changed."));
                    return;
                }
                data.learned.remove(&item);
                if let Err(error) = data::set(host, event.uuid, data) {
                    say(host, event.uuid, Text::literal(format!("§c{error:?}")));
                }
                say(host, event.uuid, Text::literal(format!(
                    "§aUnlearned {}§a.", emc::display_name(&item)
                )));
                return;
            }
            ui::table::SlotRole::Output(_) => {
                let page = table_page(host, event.uuid, event.window);
                let Some(item) = ui::server::item_at_slot(host, event.uuid, page, event.slot as usize) else {
                    return;
                };
                item
            }
        };
        let item = item.as_str();
        let value = emc_value(host, item);
        if value == 0 {
            return;
        }
        let count: u32 = if event.mode == 1 {
            64
        } else if event.button == 1 {
            16
        } else {
            1
        };
        let plan = match transaction::plan_purchase(
            value,
            count,
            data.emc as u64,
            inventory_room(host, event.uuid, item),
        ) {
            Ok(plan) => plan,
            Err(error) => {
                say(host, event.uuid, Text::literal(format!("§c{}", error.message())));
                return;
            }
        };
        let reserved = plan.maximum_cost();
        if let Err(error) = data::take_emc(host, event.uuid, reserved as i64) {
            say(host, event.uuid, Text::literal(format!("§c{error:?}")));
            return;
        }
        let given = give_and_count(host, event.uuid, item, count);
        let settlement = match transaction::settle_purchase(plan, given, data.emc as u64) {
            Ok(settlement) => settlement,
            Err(error) => {
                credit(host, event.uuid, reserved as i64);
                say(host, event.uuid, Text::literal(format!("§c{}", error.message())));
                return;
            }
        };
        if settlement.spent < reserved {
            credit(host, event.uuid, reserved - settlement.spent);
        }
        say(host, event.uuid, Text::literal(format!(
            "§aBought §e{}x {}§a for §e{} EMC§a.",
            settlement.granted,
            emc::display_name(item),
            settlement.spent
        )));
    }

    fn item_tooltip(&self, host: &dyn ClientHost, event: &mut ItemTooltipEvent<'_>) {
        tooltip_lines(host, event.stack, event.lines);
    }

    fn screen_opening(&self, _host: &dyn ClientHost, event: &mut ScreenOpeningEvent<'_>) -> Option<ScreenOpening> {
        ui::screen::screen_opening(event.new)
    }

    fn screen_closing(&self, _host: &dyn ClientHost, event: &mut ScreenClosingEvent<'_>) {
        ui::screen::screen_closing(event.screen);
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        _host: &dyn ClientHost,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        ui::screen::mouse_pressed(event.screen, event.button, event.modifiers)
    }

    fn screen_mouse_button_released_pre(
        &self,
        _host: &dyn ClientHost,
        event: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        ui::screen::mouse_released(event.screen, event.button, event.modifiers)
    }

    fn screen_char_typed_pre(
        &self,
        _host: &dyn ClientHost,
        event: &mut ScreenCharTypedPreEvent<'_>,
    ) -> Flow {
        ui::screen::char_typed(event.screen, event.c, event.modifiers)
    }

    fn screen_key_pressed_pre(
        &self,
        _host: &dyn ClientHost,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        ui::screen::key_pressed(event.screen, event.key, event.repeat, event.modifiers)
    }
}

/// The per-block behaviour for the table and every machine.
struct ProjectEBlockEntity;

#[mod_api::horizon_part]
impl BlockPart for ProjectEBlockEntity {}

#[mod_api::horizon_part]
impl BlockEntityPart for ProjectEBlockEntity {
    fn block_entity(&self, _level: LevelSide, event: &mut BlockEntityEvent<'_>) -> Option<Compound> {
        if event.block == TABLE {
            return Some(Compound::new());
        }
        machines::block_entity(event.block).map(|(_, data)| data)
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = machines::has_tick(event.block);
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        machines::tick(host, event);
    }

    fn menu_button(&self, host: &mut dyn ServerHost, event: &mut MenuButtonEvent<'_>) {
        let forward = match event.context.id {
            ui::table::BUTTON_PAGE_PREV => false,
            ui::table::BUTTON_PAGE_NEXT => true,
            _ => return,
        };
        if event.entity.id() != TABLE_BLOCK_ENTITY {
            return;
        }
        let Ok(data) = data::get(host, event.context.player) else {
            return;
        };
        let current = event
            .entity
            .data
            .first()
            .copied()
            .unwrap_or(0)
            .max(0) as usize;
        let last = data.learned.len().div_ceil(ui::table::MATTER_DISPLAY).max(1);
        let next = ui::table::advance_page(current, forward, last);
        if next != current {
            if let Some(slot) = event.entity.data.first_mut() {
                *slot = next as i32;
                host.block_entity_sync(event.context.pos);
            } else {
                say(host, event.context.player, Text::literal(
                    "§cThe table has no data slot for its page.",
                ));
            }
        }
    }

    fn screen(&self, _host: &dyn Host, event: &mut ScreenEvent<'_>) -> Option<Screen> {
        ui::screen::screen(event.block)
    }

    fn screen_draw(&self, host: &dyn ClientHost, event: &mut ScreenDrawEvent<'_>) {
        ui::screen::screen_draw(host, event.draw, event.block, event.data, event.origin);
    }

    fn screen_click(&self, _host: &dyn ClientHost, event: &mut ScreenClickEvent<'_>) -> Option<i32> {
        ui::screen::screen_click(event.block, event.at)
    }

    fn screen_tooltip(&self, _host: &dyn ClientHost, event: &mut ScreenTooltipEvent<'_>) {
        ui::screen::screen_tooltip(event.block, event.data, event.at, event.lines);
    }
}

/// Slot rules for the table's own menu: the 16 output cells are display-only.
struct TableMenuPart;

#[mod_api::horizon_part]
impl MenuPart for TableMenuPart {
    fn menu_slot_rules(&self, _host: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        match ui::table::slot_role(event.slot) {
            ui::table::SlotRole::Output(_) => event.may_place = false,
            ui::table::SlotRole::Lock | ui::table::SlotRole::Consume | ui::table::SlotRole::Unlearn => {
                event.max_stack = 1;
            }
            ui::table::SlotRole::Input | ui::table::SlotRole::Player => {}
        }
    }
}

/// The Philosopher's Stone's own item behaviour: world transmutation.
struct StonePart;

#[mod_api::horizon_part]
impl ItemPart for StonePart {
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        let Some(level) = level_of(host, event.player) else {
            return None;
        };
        let from = level.block(host, event.pos)?.id(host);
        if !alchemical::is_transformable(&from) {
            return None;
        }
        let sneaking = is_sneaking(host, event.player);
        let Some(to) = transmutation::resolve(&from, sneaking) else {
            say(host, event.player, Text::literal(format!(
                "§7The stone has no second form for {}.",
                emc::display_name(&from)
            )));
            return Some(InteractionResult::Fail);
        };
        let state = host.block_state_id(to)?;
        match level.set_block(
            host,
            event.pos,
            state,
            BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
        ) {
            Ok(true) => say(host, event.player, Text::literal(format!(
                "§dTransmuted {} into §e{}§d.",
                emc::display_name(&from),
                emc::display_name(to)
            ))),
            Ok(false) => say(host, event.player, Text::literal(
                "§7Nothing changed: the block is no longer what you clicked.",
            )),
            Err(reason) => say(host, event.player, Text::literal(format!(
                "§cThe stone could not change the block: {reason:?}"
            ))),
        }
        Some(InteractionResult::Consume)
    }
}

fn block(event: &mut BlocksEvent<'_>, id: &str, template: &str) {
    let mut declaration = Block::default();
    declaration.id = id.to_string();
    declaration.template = template.to_string();
    declaration.item = true;
    event.blocks.push(declaration);
}

fn item(event: &mut ItemsEvent<'_>, id: &str, template: &str, max: u8) {
    let mut declaration = Item::default();
    declaration.id = id.to_string();
    declaration.template = template.to_string();
    declaration.max_stack_size = Some(max);
    event.items.push(declaration);
}

fn level_of(host: &dyn Host, uuid: u128) -> Option<LevelRef> {
    let player = host.player(uuid)?;
    let dimension = player.dimension(host)?;
    let id = format!("{}:{}", dimension.namespace(), dimension.path());
    host.level(&id)
}

fn is_sneaking(host: &dyn Host, uuid: u128) -> bool {
    host.player(uuid)
        .and_then(|player| player.is_secondary_use_active(host))
        .unwrap_or(false)
}

/// Resolve EMC through the merged `projecte:emc` data map. A lookup failure is
/// reported explicitly before the static table is used; it is never swallowed.
fn emc_value(host: &dyn Host, id: &str) -> u64 {
    match emc::value_mapped(host, id) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("[projecte] EMC data map lookup failed for {id}: {error:?}; using the static table");
            emc::value(id)
        }
    }
}

/// The value of a concrete stack: the data-map/static base adjusted by the
/// component processors (durability, stored EMC, enchantments, exact override).
fn emc_value_stack(host: &dyn Host, stack: &ItemStack) -> u64 {
    let id = stack.item().id(host);
    let base = emc_value(host, &id);
    let maximum = host.max_damage(stack.item()).unwrap_or(0);
    let damage = stack.damage();
    let durability = (maximum > 0 && damage > 0).then_some(emc::Durability {
        remaining: maximum.saturating_sub(damage),
        maximum,
    });
    let patch = stack.components().map(|components| components.as_nbt());
    let enchantments = match patch.and_then(|p| p.get(&"minecraft:enchantments".into())) {
        Some(Tag::Compound(levels)) => levels
            .iter()
            .filter_map(|(id, tag)| {
                let level = match tag {
                    Tag::Int(value) if *value > 0 => *value as u32,
                    Tag::Long(value) if *value > 0 => u32::try_from(*value).ok()?,
                    _ => return None,
                };
                Some((id.to_string_lossy(), level, 1))
            })
            .collect(),
        _ => Vec::new(),
    };
    let exact_value = match patch.and_then(|p| p.get(&"projecte:exact_value".into())) {
        Some(Tag::Long(value)) if *value >= 0 => u64::try_from(*value).ok(),
        Some(Tag::Int(value)) if *value >= 0 => Some(*value as u64),
        _ => None,
    };
    let components = emc::ItemComponents {
        durability,
        enchantments,
        stored_emc: stack.get(&energy::STORED_EMC).unwrap_or(0),
        exact_value,
    };
    emc::value_with_components(base, &components, emc::ComponentConfig::default())
}

fn item_key_of(host: &dyn Host, id: &str) -> Option<ItemKey> {
    host.registry_entry("minecraft:item", id)?.as_item_key(host)
}

fn stack(host: &dyn Host, id: &str, count: u32) -> Option<ItemStack> {
    ItemStack::new(item_key_of(host, id)?, count)
}

fn say(host: &mut dyn ServerHost, uuid: u128, line: Text) {
    if let Some(player) = host.player(uuid) {
        if let Err(error) = player.tell(host, line) {
            let _ = host.messages().broadcast(Text::literal(format!(
                "[projecte] failed to deliver a message: {error:?}"
            )));
        }
    }
}

/// The table's current page lives in the backing block entity's data slot 0,
/// which the menu syncs to the client. An unavailable value means page 0.
fn table_page(host: &mut dyn ServerHost, uuid: u128, window: i32) -> usize {
    let Some(menu) = host.menu(uuid, window) else {
        return 0;
    };
    let Some(MenuSlotBacking::BlockEntity { level, pos, .. }) =
        menu.slot_backing(host, ui::table::OUTPUT_FIRST)
    else {
        return 0;
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return 0;
    };
    entity
        .data_slots(host)
        .and_then(|slots| slots.first().copied())
        .unwrap_or(0)
        .max(0) as usize
}

fn feedback(host: &mut dyn ServerHost, line: Text) {
    if let Err(error) = host.commands().feedback(line) {
        let _ = host.messages().broadcast(Text::literal(format!(
            "[projecte] command feedback failed: {error:?}"
        )));
    }
}

/// Credit EMC back after a failed or partial settlement. A failure here is
/// reported, never silently dropped: it leaves the player's balance short.
fn credit(host: &mut dyn ServerHost, uuid: u128, amount: i64) {
    if let Err(error) = data::add_emc(host, uuid, amount) {
        let line = format!("[projecte] could not refund {amount} EMC to {uuid}: {error:?}");
        // A refund failure must be visible even when the player has left.
        if let Some(player) = host.player(uuid) {
            if player.tell(host, Text::literal(line.clone())).is_err() {
                eprintln!("{line}");
            }
        } else {
            eprintln!("{line}");
        }
    }
}

const PLAYER_INVENTORY_SLOTS: u32 = 36;

fn inventory_room(host: &dyn Host, uuid: u128, item: &str) -> u32 {
    let Some(key) = item_key_of(host, item) else {
        return 0;
    };
    let max = host.max_stack_size(key).unwrap_or(64).max(1);
    let Some(player) = host.player(uuid) else {
        return 0;
    };
    let Some(stacks) = player.inventory(host) else {
        return 0;
    };
    let mut room = 0u32;
    for slot in stacks.iter() {
        if slot.item() == key && slot.damage() == 0 {
            room = room.saturating_add(max.saturating_sub(slot.count()));
        }
    }
    let empty = PLAYER_INVENTORY_SLOTS.saturating_sub(stacks.len() as u32);
    room.saturating_add(max.saturating_mul(empty))
}

/// Give a stack and report how many items the inventory actually accepted.
fn give_and_count(host: &mut dyn ServerHost, uuid: u128, item: &str, count: u32) -> u32 {
    let Some(key) = item_key_of(host, item) else {
        return 0;
    };
    let Some(player) = host.player(uuid) else {
        return 0;
    };
    let before = player
        .inventory(host)
        .map(|slots| count_item(&slots, key))
        .unwrap_or(0);
    let Some(given) = stack(host, item, count) else {
        feedback(host, Text::literal("§cThe item could not be resolved."));
        return 0;
    };
    if let Err(refusal) = player.give_stack(host, given) {
        feedback(host, Text::literal(format!("§c{refusal:?}")));
        return 0;
    }
    let after = player
        .inventory(host)
        .map(|slots| count_item(&slots, key))
        .unwrap_or(before);
    after.saturating_sub(before)
}

fn count_item(slots: &[ItemStack], key: ItemKey) -> u32 {
    slots
        .iter()
        .filter(|slot| slot.item() == key && slot.damage() == 0)
        .map(|slot| slot.count())
        .fold(0u32, |total, count| total.saturating_add(count))
}

fn table_container() -> ModContainer {
    ui::atlas::table_container()
}

fn normalize_item(raw: &str) -> String {
    if raw.contains(':') {
        raw.to_string()
    } else {
        format!("minecraft:{raw}")
    }
}

fn tooltip_lines(host: &dyn Host, stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    let item = stack.item().id(host);
    let item = item.as_str();
    if let Some(capacity) = energy::capacity(item) {
        let stored = energy::stored_emc(stack).unwrap_or(0);
        lines.push((
            Text::literal(format!("§bStored EMC: §f{stored} / {capacity}")),
            u32::MAX,
        ));
    }
    let value = emc_value_stack(host, stack);
    if value > 0 {
        lines.push((Text::literal(format!("§7EMC: §e{value}")), u32::MAX));
    }
    if stack.count() > 1 && let Some(total) = alchemical::total_value(value, stack.count()) {
        lines.push((Text::literal(format!("§7Stack EMC: §e{total}")), u32::MAX));
    }
    if let Some(tier) = alchemical::tier(item) {
        let burn = alchemical::burn_ticks(item).unwrap_or(0);
        let previous = alchemical::previous_tier(item).unwrap_or("none");
        let next = alchemical::next_tier(item).unwrap_or("none");
        let block = alchemical::fuel_block(item).unwrap_or("none");
        lines.push((
            Text::literal(format!(
                "§6Fuel tier {} · {} reference burn ticks · prev: {} · next: {} · block: {}",
                tier + 1, burn, previous, next, block
            )),
            u32::MAX,
        ));
    }
    if let Some(fuel) = alchemical::fuel_of_block(item) {
        lines.push((Text::literal(format!("§6Packed fuel: {fuel}")), u32::MAX));
    }
    if item == PHILOSOPHERS_STONE {
        let example = alchemical::STONE_CONVERSIONS
            .first()
            .map(|c| format!("{}x{} -> {}x{}", c.from_count, c.from, c.to_count, c.to))
            .unwrap_or_else(|| "none".to_string());
        lines.push((
            Text::literal(format!(
                "§7World swaps and {} reversible crafting conversions (e.g. {example})",
                alchemical::STONE_CONVERSIONS.len()
            )),
            u32::MAX,
        ));
    }
}

fn command_tokens(invocation: &CommandInvocation, host: &dyn Host) -> Vec<String> {
    let mut tokens = invocation.literals.clone();
    for arg in &invocation.arguments {
        let value = match &arg.value {
            CommandArgumentValue::String(value) => value.clone(),
            CommandArgumentValue::Integer(value) => value.to_string(),
            CommandArgumentValue::Number(value) => value.to_string(),
            CommandArgumentValue::Boolean(value) => value.to_string(),
            CommandArgumentValue::Player(player) => player.name(host).unwrap_or_default(),
            _ => continue,
        };
        tokens.push(value);
    }
    tokens
}

fn command(host: &mut dyn ServerHost, player: Option<PlayerRef>, invocation: &CommandInvocation) -> bool {
    let tokens = command_tokens(invocation, host);
    let Some(player) = player else {
        feedback(host, Text::literal("§cThis command needs a player."));
        return true;
    };
    let uuid = player.uuid();
    let Some(sub) = tokens.get(1).map(String::as_str) else {
        feedback(host, Text::literal(
            "§cUsage: /projecte <emc|learn|transmute|table|value|reset|roadmap|deposit|knowledge>",
        ));
        return true;
    };
    match sub {
        "emc" => command_emc(host, uuid, &tokens[2..]),
        "knowledge" => command_knowledge(host, uuid, &tokens[2..]),
        "learn" => command_learn(host, uuid, &tokens[2..]),
        "transmute" => command_transmute(host, uuid, &tokens[2..]),
        "table" => {
            feedback(host, Text::literal("§7Use the Transmutation Table block to open its screen."));
            true
        }
        "deposit" => {
            match deposit_menu::open(host, uuid) {
                Some(_) => feedback(host, Text::literal(
                    "§aPut items in the top nine slots, then click the green slot to deposit them.",
                )),
                None => feedback(host, Text::literal("§cThe deposit menu could not be opened for you.")),
            }
            true
        }
        "value" => {
            let Some(raw) = tokens.get(2) else {
                feedback(host, Text::literal("§cUsage: /projecte value <item>"));
                return true;
            };
            let id = normalize_item(raw);
            let value = emc_value(host, &id);
            if value == 0 {
                feedback(host, Text::literal(format!("§7{} has no EMC value.", emc::display_name(&id))));
            } else {
                feedback(host, Text::literal(format!("§e{}§7: §e{} EMC", emc::display_name(&id), value)));
            }
            true
        }
        "roadmap" => {
            for node in progression::NODES {
                feedback(host, Text::literal(format!(
                    "§7[S{}:{}] [{}] §e{}§7 - {} | needs: {} | API: {}",
                    node.stage, node.id, node.status.label(), node.name,
                    node.unlocks,
                    if node.prerequisites.is_empty() { "none".to_string() } else { node.prerequisites.join(", ") },
                    node.horizon
                )));
            }
            true
        }
        "reset" => {
            match data::get(host, uuid) {
                Ok(mut data) => {
                    data.clear_knowledge();
                    data.emc = 0;
                    match data::set(host, uuid, data) {
                        Ok(()) => feedback(host, Text::literal("§aProjectE data reset.")),
                        Err(error) => feedback(host, Text::literal(format!("§c{error:?}"))),
                    }
                }
                Err(error) => feedback(host, Text::literal(format!("§c{error:?}"))),
            }
            true
        }
        _ => {
            feedback(host, Text::literal(
                "§cUsage: /projecte <emc|learn|transmute|table|value|reset|roadmap|deposit|knowledge>",
            ));
            true
        }
    }
}

fn command_knowledge(host: &mut dyn ServerHost, uuid: u128, args: &[String]) -> bool {
    let Ok(mut data) = data::get(host, uuid) else {
        feedback(host, Text::literal("§cYour ProjectE data could not be read."));
        return true;
    };
    match args.first().map(String::as_str) {
        Some("all") => {
            if data.grant_full_knowledge() {
                match data::set(host, uuid, data) {
                    Ok(()) => feedback(host, Text::literal("§aFull knowledge granted.")),
                    Err(error) => feedback(host, Text::literal(format!("§c{error:?}"))),
                }
            } else {
                feedback(host, Text::literal("§7You already have full knowledge."));
            }
        }
        Some("clear") => {
            data.clear_knowledge();
            match data::set(host, uuid, data) {
                Ok(()) => feedback(host, Text::literal("§aAll knowledge forgotten.")),
                Err(error) => feedback(host, Text::literal(format!("§c{error:?}"))),
            }
        }
        _ => feedback(host, Text::literal(
            "§cUsage: /projecte knowledge <all|clear>",
        )),
    }
    true
}

fn command_emc(host: &mut dyn ServerHost, uuid: u128, args: &[String]) -> bool {
    if args.first().map(String::as_str) == Some("set") {
        if args.len() < 3 {
            feedback(host, Text::literal("§cUsage: /projecte emc set <player> <amount>"));
            return true;
        }
        let Some(target) = host.players().player_by_name(&args[1]) else {
            feedback(host, Text::literal("§cPlayer not found."));
            return true;
        };
        let Ok(amount) = args[2].parse::<i64>() else {
            feedback(host, Text::literal("§cAmount must be a whole number."));
            return true;
        };
        if amount < 0 {
            feedback(host, Text::literal("§cA negative EMC balance is not allowed."));
            return true;
        }
        let Ok(mut data) = data::get(host, target.uuid()) else {
            feedback(host, Text::literal("§cPlayer data could not be read."));
            return true;
        };
        data.emc = amount;
        let name = target.name(host).unwrap_or_default();
        match data::set(host, target.uuid(), data.clone()) {
            Ok(()) => feedback(host, Text::literal(format!(
                "§aSet {name}'s EMC to §e{}§a.", data.emc
            ))),
            Err(error) => feedback(host, Text::literal(format!("§c{error:?}"))),
        }
        return true;
    }
    let target = match args.first() {
        Some(name) => match host.players().player_by_name(name) {
            Some(player) => player,
            None => {
                feedback(host, Text::literal("§cPlayer not found."));
                return true;
            }
        },
        None => match host.player(uuid) {
            Some(player) => player,
            None => return true,
        },
    };
    let name = target.name(host).unwrap_or_default();
    match data::get(host, target.uuid()) {
        Ok(data) => feedback(host, Text::literal(format!(
            "§e{name}§7 has §e{} EMC§7 and §e{}§7 learned items.",
            data.emc, data.learned.len()
        ))),
        Err(error) => feedback(host, Text::literal(format!("§c{error:?}"))),
    }
    true
}

fn command_learn(host: &mut dyn ServerHost, uuid: u128, args: &[String]) -> bool {
    let Some(player) = host.player(uuid) else {
        return true;
    };
    let mut learn = Vec::new();
    if args.first().map(String::as_str) == Some("all") {
        let stacks = player.inventory(host).unwrap_or_default();
        learn.extend(stacks.iter().map(|stack| normalize_item(&stack.item().id(host))));
    } else if let Some(raw) = args.first() {
        learn.push(normalize_item(raw));
    } else if let Some(stack) = player.inventory(host).and_then(|slots| slots.first().cloned()) {
        learn.push(normalize_item(&stack.item().id(host)));
    } else {
        feedback(host, Text::literal("§cHold an item, or use /projecte learn all."));
        return true;
    }
    let Ok(mut data) = data::get(host, uuid) else {
        feedback(host, Text::literal("§cPlayer data could not be read."));
        return true;
    };
    let mut added = 0;
    let mut no_value = 0;
    let blacklist = BTreeSet::new();
    for id in learn {
        if data.learn(&id, &blacklist, |id| emc_value(host, id) > 0) {
            added += 1;
        } else {
            no_value += 1;
        }
    }
    let total = data.learned.len();
    match data::set(host, uuid, data) {
        Ok(()) => feedback(host, Text::literal(format!(
            "§aLearned §e{}§a new item(s); §e{}§a known.{}",
            added, total,
            if no_value > 0 { format!(" §7({no_value} item(s) have no EMC value.)") } else { String::new() }
        ))),
        Err(error) => feedback(host, Text::literal(format!("§c{error:?}"))),
    }
    true
}

fn command_transmute(host: &mut dyn ServerHost, uuid: u128, args: &[String]) -> bool {
    let Some(raw) = args.first() else {
        feedback(host, Text::literal("§cUsage: /projecte transmute <item> [count]"));
        return true;
    };
    let id = normalize_item(raw);
    let count = match args.get(1) {
        Some(value) => match value.parse::<u32>() {
            Ok(count) if (1..=4096).contains(&count) => count,
            _ => {
                feedback(host, Text::literal("§cCount must be a whole number from 1 to 4096."));
                return true;
            }
        },
        None => 1,
    };
    let value = emc_value(host, &id);
    if value == 0 {
        feedback(host, Text::literal(format!("§c{} has no EMC value.", emc::display_name(&id))));
        return true;
    }
    let Ok(data) = data::get(host, uuid) else {
        feedback(host, Text::literal("§cPlayer data could not be read."));
        return true;
    };
    if !data.learned.contains(&id) {
        feedback(host, Text::literal(format!(
            "§cYou have not learned {}. Use /projecte learn while holding it.",
            emc::display_name(&id)
        )));
        return true;
    }
    let plan = match transaction::plan_purchase(value, count, data.emc as u64, inventory_room(host, uuid, &id)) {
        Ok(plan) => plan,
        Err(error) => {
            feedback(host, Text::literal(format!("§c{}", error.message())));
            return true;
        }
    };
    let reserved = plan.maximum_cost();
    if let Err(error) = data::take_emc(host, uuid, reserved as i64) {
        feedback(host, Text::literal(format!("§c{error:?}")));
        return true;
    }
    let given = give_and_count(host, uuid, &id, count);
    let settlement = match transaction::settle_purchase(plan, given, data.emc as u64) {
        Ok(settlement) => settlement,
        Err(error) => {
            credit(host, uuid, reserved as i64);
            feedback(host, Text::literal(format!("§c{}", error.message())));
            return true;
        }
    };
    if settlement.spent < reserved {
        credit(host, uuid, reserved - settlement.spent);
    }
    feedback(host, Text::literal(format!(
        "§aTransmuted §e{}x {}§a for §e{} EMC§a.",
        settlement.granted, emc::display_name(&id), settlement.spent
    )));
    true
}
