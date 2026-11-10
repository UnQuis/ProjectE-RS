//! ProjectE-RS: Equivalent Exchange for Horizon.
//!
//! This is a native trait mod. It declares its blocks and items in code, so
//! the repository contains no textures, models or other binary assets; every
//! declaration borrows a vanilla template's look.

use std::sync::Arc;

use horizon_client::mod_api::*;

pub mod condenser;
mod alchemical;
mod data;
mod emc;
mod energy;
mod machines;
mod progression;
mod recipes;
mod transaction;

const TABLE: &str = recipes::TABLE;
const PHILOSOPHERS_STONE: &str = alchemical::PHILOSOPHERS_STONE;

const MENU_SLOTS: usize = 54;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(ProjectE)
}

struct ProjectE;

impl Hooks for ProjectE {
    fn blocks(&self, event: &mut BlocksEvent<'_>) {
        event.blocks.push(Default::default());
        let table = event.blocks.last_mut().unwrap();
        table.id = TABLE.to_string();
        table.template = "minecraft:crafting_table".to_string();
        table.item = true;

        for id in machines::COLLECTORS.into_iter()
            .chain(machines::RELAYS)
            .chain([condenser::ENERGY_CONDENSER])
        {
            event.blocks.push(Default::default());
            let machine = event.blocks.last_mut().unwrap();
            machine.id = id.to_string();
            machine.template = "minecraft:furnace".to_string();
            machine.item = true;
        }

        event.blocks.push(Default::default());
        let chest = event.blocks.last_mut().unwrap();
        chest.id = machines::ALCHEMICAL_CHEST.to_string();
        chest.template = "minecraft:chest".to_string();
        chest.item = true;

        // Fuel storage blocks. They borrow a vanilla template look until the
        // project's own models and textures exist.
        for (_, block) in alchemical::FUEL_BLOCKS {
            event.blocks.push(Default::default());
            let fuel = event.blocks.last_mut().unwrap();
            fuel.id = block.to_string();
            fuel.template = "minecraft:coal_block".to_string();
            fuel.item = true;
        }
    }

    fn items(&self, event: &mut ItemsEvent<'_>) {
        item(event, PHILOSOPHERS_STONE, "minecraft:diamond", 1);
        for id in energy::KLEIN_STARS {
            item(event, id, "minecraft:nether_star", 1);
        }
        item(event, "projecte:dark_matter", "minecraft:coal", 64);
        item(event, "projecte:red_matter", "minecraft:redstone", 64);
        item(event, "projecte:alchemical_coal", "minecraft:coal", 64);
        item(event, "projecte:mobius_fuel", "minecraft:blaze_powder", 64);
        item(event, "projecte:aeternalis_fuel", "minecraft:glowstone_dust", 64);
    }

    fn component_types(&self, event: &mut ComponentTypesEvent<'_>) {
        if !event.types.iter().any(|id| id == energy::STORED_EMC_COMPONENT) {
            event.types.push(energy::STORED_EMC_COMPONENT.to_string());
        }
    }

    fn block_entity(
        &self,
        _host: &mut dyn Host,
        event: &mut BlockEntityEvent<'_>,
    ) -> Option<(String, Compound)> {
        machines::block_entity(event.block)
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = machines::has_tick(event.block);
    }

    fn block_entity_container(
        &self,
        _host: &dyn Host,
        event: &mut BlockEntityContainerEvent<'_>,
    ) -> Option<ModContainer> {
        machines::container(event.block)
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
        machines::tick(host, event);
    }

    fn right_click_block(
        &self,
        host: &mut dyn Host,
        event: &mut RightClickBlockEvent,
    ) -> Flow {
        let block = host.block(event.pos);
        if block == TABLE {
            if let Some(player) = host.acting_player() {
                open_table(host, player.uuid);
                return Flow::Cancel;
            }
            return Flow::Continue;
        }
        if machines::is_collectable(&block) {
            if !host.sneaking() {
                return Flow::Continue;
            }
            let Some(player) = host.acting_player() else {
                return Flow::Continue;
            };
            machines::collect(host, event.pos, player.uuid);
            return Flow::Cancel;
        }
        Flow::Continue
    }

    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        if &*event.stack.item != PHILOSOPHERS_STONE {
            return Flow::Continue;
        }
        let Some(player) = host.acting_player() else {
            return Flow::Continue;
        };
        open_table(host, player.uuid);
        Flow::Cancel
    }

    /// World transmutation: the stone turns the block it is used on into a
    /// deterministic partner block, free of charge and without being used up.
    ///
    /// `edit_blocks` writes block states by name on the level being ticked,
    /// and its filter makes the write conditional: a block that changed since
    /// the click is left alone instead of overwritten. The block is free, as
    /// public ProjectE behavior describes; the cost of the stone is its recipe.
    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        if &*event.stack.item != PHILOSOPHERS_STONE {
            return Flow::Continue;
        }
        // Client-side copies of the world have no acting player, and the world
        // edit helper refuses a level that is not the server's ticking one.
        let Some(player) = host.acting_player() else {
            return Flow::Continue;
        };
        let from = host.block(event.pos);
        if !alchemical::is_transformable(&from) {
            // Leave the click alone: the stone has nothing to say about this
            // block, so it must not claim the use either.
            return Flow::Continue;
        }
        let Some(to) = alchemical::world_swap(&from, player.sneaking) else {
            // Transformable, but this block has a single form and the player
            // asked for the second one.
            host.action_bar(
                player.uuid,
                Text::literal(format!(
                    "§7The stone has no second form for {}.",
                    emc::display_name(&from)
                )),
            );
            return Flow::Cancel;
        };
        let edit = NativeBlockEdit {
            palette: vec![to.to_string()],
            filters: vec![from.clone()],
            runs: vec![NativeBlockRun {
                x: event.pos[0],
                z: event.pos[2],
                y: event.pos[1],
                height: 1,
                surface: None,
                block: 0,
                only: Some(0),
            }],
        };
        match host.edit_blocks(&player.dimension, &edit) {
            Ok(1) => {
                host.action_bar(
                    player.uuid,
                    Text::literal(format!(
                        "§dTransmuted {} into §e{}§d.",
                        emc::display_name(&from),
                        emc::display_name(to)
                    )),
                );
            }
            Ok(0) => {
                // The chunk was not loaded, or the block stopped being what
                // was clicked. Nothing was changed, and the player is told so
                // rather than left to think the click worked.
                host.action_bar(
                    player.uuid,
                    Text::literal(format!(
                        "§7Nothing changed: the block is no longer {}.",
                        emc::display_name(&from)
                    )),
                );
            }
            Ok(changed) => {
                // The single-run edit can only report one block; more than
                // that means the Host applied something else, which is worth
                // surfacing instead of hiding.
                host.tell(
                    player.uuid,
                    Text::literal(format!(
                        "§cThe stone applied {changed} block changes instead of one."
                    )),
                );
            }
            Err(reason) => {
                // A refused edit is reported, never swallowed: the player (or
                // the log reader, through the chat line) must know why.
                host.tell(
                    player.uuid,
                    Text::literal(format!("§cThe stone could not change the block: {reason}")),
                );
            }
        }
        Flow::Cancel
    }

    /// Data-pack recipes cannot keep an ingredient, so every recipe that needs
    /// the Philosopher's Stone lists it as an ordinary one; this hook gives one
    /// stone back for each such craft. See `API_WORKAROUNDS.md`.
    fn item_crafted(&self, host: &mut dyn Host, event: &mut ItemCraftedEvent<'_>) {
        if !recipes::is_catalyst_recipe(event.recipe) {
            return;
        }
        let leftover = host.give_stack(event.player, stack(PHILOSOPHERS_STONE, 1));
        if let Some(leftover) = leftover {
            host.tell(
                event.player,
                Text::literal(format!(
                    "§cYour Philosopher's Stone could not be returned: {} did not fit in your inventory.",
                    leftover.count
                )),
            );
        }
    }

    fn menu_clicked(&self, host: &mut dyn Host, event: &mut MenuClickedEvent) {
        if !(0..MENU_SLOTS as i32).contains(&event.slot) {
            return;
        }
        let data = data::get(host, event.uuid);
        let learned = learned_items(&data);
        let Some(item) = learned.get(event.slot as usize) else {
            return;
        };
        let value = emc::value(item);
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
            data.emc,
            inventory_room(host, event.uuid, item),
        ) {
            Ok(plan) => plan,
            Err(error) => {
                host.tell(event.uuid, Text::literal(format!("§c{}", error.message())));
                return;
            }
        };
        let leftover = host.give_stack(event.uuid, stack(item, count));
        let leftover_count = leftover.as_ref().map_or(0, |s| s.count);
        let Some(given) = count.checked_sub(leftover_count) else {
            host.tell(
                event.uuid,
                Text::literal("§cThe inventory returned an invalid item count."),
            );
            return;
        };
        let settlement = match transaction::settle_purchase(plan, given, data.emc) {
            Ok(settlement) => settlement,
            Err(error) => {
                host.tell(event.uuid, Text::literal(format!("§c{}", error.message())));
                return;
            }
        };
        let mut data = data;
        data.emc = settlement.remaining_emc;
        data::set(host, event.uuid, data.clone());
        let spent = settlement.spent;
        let given = settlement.granted;
        host.tell(
            event.uuid,
            Text::literal(format!(
                "§aBought §e{}x {}§a for §e{} EMC§a. Balance: §e{}§a.",
                given,
                emc::display_name(item),
                spent,
                data.emc
            )),
        );
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(NativeCommand::new("projecte emc [player:player]"));
        event.commands.push(
            NativeCommand::op("projecte emc set <player:player> <amount:integer>", 2),
        );
        event
            .commands
            .push(NativeCommand::new("projecte learn [item:word]"));
        event.commands.push(NativeCommand::new(
            "projecte transmute <item:word> [count:integer]",
        ));
        event.commands.push(NativeCommand::new("projecte table"));
        event
            .commands
            .push(NativeCommand::new("projecte value <item:word>"));
        event
            .commands
            .push(NativeCommand::new("projecte reset"));
        event
            .commands
            .push(NativeCommand::new("projecte roadmap"));
    }

    fn perform_command(
        &self,
        host: &mut dyn Host,
        event: &mut PerformCommandEvent<'_>,
    ) -> Flow {
        let parts: Vec<&str> = event.command.split_whitespace().collect();
        if parts.first().copied() != Some("projecte") {
            return Flow::Continue;
        }
        let result = match parts.get(1).copied() {
            Some("emc") => command_emc(host, event, &parts[2..]),
            Some("learn") => command_learn(host, event, &parts[2..]),
            Some("transmute") => command_transmute(host, event, &parts[2..]),
            Some("table") => {
                open_table(host, event.player.uuid);
                true
            }
            Some("value") => {
                let Some(id) = parts.get(2) else {
                    host.feedback(Text::literal("§cUsage: /projecte value <item>"));
                    return Flow::Cancel;
                };
                let id = normalize_item(id);
                let value = emc::value(&id);
                if value == 0 {
                    host.feedback(Text::literal(format!(
                        "§7{} has no EMC value.",
                        emc::display_name(&id)
                    )));
                } else {
                    host.feedback(Text::literal(format!(
                        "§e{}§7: §e{} EMC",
                        emc::display_name(&id),
                        value
                    )));
                }
                true
            }
            Some("roadmap") => {
                command_roadmap(host);
                true
            }
            Some("reset") => {
                data::set(host, event.player.uuid, data::PlayerData::default());
                host.feedback(Text::literal("§aProjectE data reset."));
                true
            }
            _ => false,
        };
        if result {
            Flow::Cancel
        } else {
            host.feedback(Text::literal(
                "§cUsage: /projecte <emc|learn|transmute|table|value|reset|roadmap>",
            ));
            Flow::Cancel
        }
    }

    fn tooltip(&self, host: &dyn Host, event: &mut TooltipEvent<'_>) {
        if let Some(capacity) = energy::capacity(&event.stack.item) {
            let stored = energy::stored_emc(host, &event.stack).unwrap_or(0);
            event.lines.push((
                Text::literal(format!("§bStored EMC: §f{stored} / {capacity}")),
                u32::MAX,
            ));
        }
        let value = emc::value(&event.stack.item);
        if value > 0 {
            event.lines.push((
                Text::literal(format!("§7EMC: §e{}", value)),
                u32::MAX,
            ));
        }
    }

    fn data_pack_files(&self, _host: &dyn Host, event: &mut DataPackFilesEvent<'_>) {
        event.files.extend(recipes::files());
    }
}

fn item(event: &mut ItemsEvent<'_>, id: &str, template: &str, max: u8) {
    event.items.push(Default::default());
    let item = event.items.last_mut().unwrap();
    item.id = id.to_string();
    item.template = template.to_string();
    item.max_stack_size = Some(max);
}

fn stack(id: &str, count: u32) -> ItemStack {
    ItemStack {
        item: Arc::from(id),
        count,
        damage: 0,
        extra: None,
    }
}

fn learned_items(data: &data::PlayerData) -> Vec<String> {
    let mut items: Vec<String> = data
        .learned
        .iter()
        .filter(|id| emc::value(id) > 0)
        .cloned()
        .collect();
    items.sort_by(|a, b| {
        emc::value(b)
            .cmp(&emc::value(a))
            .then_with(|| a.cmp(b))
    });
    items
}

fn open_table(host: &mut dyn Host, uuid: u128) {
    let data = data::get(host, uuid);
    let learned = learned_items(&data);
    if learned.is_empty() {
        host.tell(
            uuid,
            Text::literal("§7No learned items yet. Use §e/projecte learn§7 while holding one."),
        );
        return;
    }
    let mut items = vec![None; MENU_SLOTS];
    for (slot, id) in learned.iter().take(MENU_SLOTS).enumerate() {
        items[slot] = Some(stack(id, 1));
    }
    let title = format!("Transmutation Table  -  {} EMC", data.emc);
    host.open_menu(uuid, 6, Text::literal(title), items);
}

fn inventory_room(host: &dyn Host, uuid: u128, item: &str) -> u32 {
    let max = host.max_stack_size(item).unwrap_or(64).max(1);
    let Some(stacks) = host.player_stacks(uuid) else {
        return 0;
    };
    let mut room = 0u32;
    for slot in stacks.iter().flatten() {
        if &*slot.item == item && slot.damage == 0 && slot.extra.is_none() {
            room = room.saturating_add(max.saturating_sub(slot.count));
        }
    }
    room = room.saturating_add(max.saturating_mul(stacks.iter().filter(|s| s.is_none()).count() as u32));
    room
}

fn normalize_item(raw: &str) -> String {
    if raw.contains(':') {
        raw.to_string()
    } else {
        format!("minecraft:{raw}")
    }
}

fn find_player(host: &dyn Host, name: &str) -> Option<NativeServerPlayer> {
    host.server_players()
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

/// Prints the documented progression and what of it exists today. The data
/// lives in `progression.rs`, so this cannot drift from the tests.
fn command_roadmap(host: &mut dyn Host) {
    host.feedback(Text::literal(
        "§dProjectE-RS progression (status - node - what it unlocks):",
    ));
    for node in progression::NODES {
        host.feedback(Text::literal(format!(
            "§7[{}] §e{}§7 - {}",
            node.status.label(),
            node.name,
            node.unlocks
        )));
    }
}

fn command_emc(host: &mut dyn Host, event: &PerformCommandEvent<'_>, args: &[&str]) -> bool {
    if args.first().copied() == Some("set") {
        if args.len() < 3 {
            host.feedback(Text::literal(
                "§cUsage: /projecte emc set <player> <amount>",
            ));
            return true;
        }
        let Some(target) = find_player(host, args[1]) else {
            host.feedback(Text::literal("§cPlayer not found."));
            return true;
        };
        let Ok(amount) = args[2].parse::<i64>() else {
            host.feedback(Text::literal("§cAmount must be a whole number."));
            return true;
        };
        let mut data = data::get(host, target.uuid);
        data.emc = amount.max(0);
        data::set(host, target.uuid, data.clone());
        host.feedback(Text::literal(format!(
            "§aSet {}'s EMC to §e{}§a.",
            target.name, data.emc
        )));
        return true;
    }
    let target = match args.first() {
        Some(name) => match find_player(host, name) {
            Some(player) => player,
            None => {
                host.feedback(Text::literal("§cPlayer not found."));
                return true;
            }
        },
        None => event.player.clone(),
    };
    let data = data::get(host, target.uuid);
    host.feedback(Text::literal(format!(
        "§e{}§7 has §e{} EMC§7 and §e{}§7 learned items.",
        target.name,
        data.emc,
        data.learned.len()
    )));
    true
}

fn command_learn(host: &mut dyn Host, event: &PerformCommandEvent<'_>, args: &[&str]) -> bool {
    let mut learn = Vec::new();
    if args.first().copied() == Some("all") {
        learn.extend(host.carried_items().into_iter().map(|(id, _)| normalize_item(&id)));
    } else if let Some(raw) = args.first() {
        learn.push(normalize_item(raw));
    } else if let Some((id, _, _)) = host.held() {
        learn.push(id);
    } else {
        host.feedback(Text::literal(
            "§cHold an item, or use /projecte learn all.",
        ));
        return true;
    }

    let mut data = data::get(host, event.player.uuid);
    let mut added = 0;
    let mut no_value = 0;
    for id in learn {
        if emc::value(&id) == 0 {
            no_value += 1;
            continue;
        }
        if data.learned.insert(id) {
            added += 1;
        }
    }
    let total = data.learned.len();
    data::set(host, event.player.uuid, data);
    host.feedback(Text::literal(format!(
        "§aLearned §e{}§a new item(s); §e{}§a known.{}",
        added,
        total,
        if no_value > 0 {
            format!(" §7({no_value} item(s) have no EMC value.)")
        } else {
            String::new()
        }
    )));
    true
}

fn command_transmute(host: &mut dyn Host, event: &PerformCommandEvent<'_>, args: &[&str]) -> bool {
    let Some(raw) = args.first() else {
        host.feedback(Text::literal(
            "§cUsage: /projecte transmute <item> [count]",
        ));
        return true;
    };
    let id = normalize_item(raw);
    let count = args
        .get(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, 4096);
    let value = emc::value(&id);
    if value == 0 {
        host.feedback(Text::literal(format!(
            "§c{} has no EMC value.",
            emc::display_name(&id)
        )));
        return true;
    }
    let data = data::get(host, event.player.uuid);
    if !data.learned.contains(&id) {
        host.feedback(Text::literal(format!(
            "§cYou have not learned {}. Use /projecte learn while holding it.",
            emc::display_name(&id)
        )));
        return true;
    }
    let plan = match transaction::plan_purchase(
        value,
        count,
        data.emc,
        inventory_room(host, event.player.uuid, &id),
    ) {
        Ok(plan) => plan,
        Err(error) => {
            host.feedback(Text::literal(format!("§c{}", error.message())));
            return true;
        }
    };
    let leftover = host.give_stack(event.player.uuid, stack(&id, count));
    let leftover_count = leftover.as_ref().map_or(0, |s| s.count);
    let Some(given) = count.checked_sub(leftover_count) else {
        host.feedback(Text::literal("§cThe inventory returned an invalid item count."));
        return true;
    };
    let settlement = match transaction::settle_purchase(plan, given, data.emc) {
        Ok(settlement) => settlement,
        Err(error) => {
            host.feedback(Text::literal(format!("§c{}", error.message())));
            return true;
        }
    };
    let mut data = data;
    data.emc = settlement.remaining_emc;
    data::set(host, event.player.uuid, data.clone());
    let spent = settlement.spent;
    let given = settlement.granted;
    host.feedback(Text::literal(format!(
        "§aTransmuted §e{}x {}§a for §e{} EMC§a. Balance: §e{}§a.",
        given,
        emc::display_name(&id),
        spent,
        data.emc
    )));
    true
}
