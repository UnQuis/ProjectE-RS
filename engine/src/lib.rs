//! ProjectE-RS: Equivalent Exchange for Horizon.
//!
//! This is a native trait mod. It declares its blocks and items in code, so
//! the repository contains no textures, models or other binary assets; every
//! declaration borrows a vanilla template's look.

use std::sync::Arc;

use horizon_client::mod_api::*;

mod data;
mod emc;
mod transaction;

const MOD_ID: &str = "projecte";
const TABLE: &str = "projecte:transmutation_table";
const COLLECTOR: &str = "projecte:energy_collector";
const RELAY: &str = "projecte:antimatter_relay";
const MACHINE_BE: &str = "projecte:machine";
const PHILOSOPHERS_STONE: &str = "projecte:philosophers_stone";

const MENU_SLOTS: usize = 54;
const COLLECTOR_CAP: i64 = 100_000;
const RELAY_CAP: i64 = 1_000_000;

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

        event.blocks.push(Default::default());
        let collector = event.blocks.last_mut().unwrap();
        collector.id = COLLECTOR.to_string();
        collector.template = "minecraft:stone".to_string();
        collector.item = true;

        event.blocks.push(Default::default());
        let relay = event.blocks.last_mut().unwrap();
        relay.id = RELAY.to_string();
        relay.template = "minecraft:stone".to_string();
        relay.item = true;
    }

    fn items(&self, event: &mut ItemsEvent<'_>) {
        item(event, PHILOSOPHERS_STONE, "minecraft:diamond", 1);
        item(event, "projecte:klein_star_ein", "minecraft:nether_star", 1);
        item(event, "projecte:klein_star_zwei", "minecraft:nether_star", 1);
        item(event, "projecte:klein_star_drei", "minecraft:nether_star", 1);
        item(event, "projecte:klein_star_vier", "minecraft:nether_star", 1);
        item(event, "projecte:dark_matter", "minecraft:coal", 64);
        item(event, "projecte:red_matter", "minecraft:redstone", 64);
        item(event, "projecte:alchemical_coal", "minecraft:coal", 64);
        item(event, "projecte:mobius_fuel", "minecraft:blaze_powder", 64);
        item(event, "projecte:aeternalis_fuel", "minecraft:glowstone_dust", 64);
    }

    fn block_entity(
        &self,
        _host: &mut dyn Host,
        event: &mut BlockEntityEvent<'_>,
    ) -> Option<(String, Compound)> {
        if event.block == COLLECTOR || event.block == RELAY {
            Some((MACHINE_BE.to_string(), Compound::new()))
        } else {
            None
        }
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        if event.block == COLLECTOR || event.block == RELAY {
            event.ticks = true;
        }
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
        if event.entity.id != MACHINE_BE {
            return;
        }
        let block = host.block(event.pos);
        let (rate, cap) = if block == RELAY {
            (4, RELAY_CAP)
        } else if block == COLLECTOR {
            (1, COLLECTOR_CAP)
        } else {
            return;
        };
        let holder = Holder::BlockEntity(event.pos);
        let current = data::BUFFER.get(host, holder).unwrap_or(0);
        if current < cap {
            let _ = data::BUFFER.set(host, holder, (current + rate).min(cap));
        }
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
        if block == COLLECTOR || block == RELAY {
            let Some(player) = host.acting_player() else {
                return Flow::Continue;
            };
            let holder = Holder::BlockEntity(event.pos);
            let stored = data::BUFFER.get(host, holder).unwrap_or(0);
            if stored > 0 {
                let total = data::add_emc(host, player.uuid, stored);
                let _ = data::BUFFER.set(host, holder, 0);
                host.tell(
                    player.uuid,
                    Text::literal(format!(
                        "§dCollected §e{} EMC§d. Stored: §e{}§d.",
                        stored, total
                    )),
                );
            } else {
                host.tell(
                    player.uuid,
                    Text::literal("§7This machine has not collected any EMC yet."),
                );
            }
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
                "§cUsage: /projecte <emc|learn|transmute|table|value|reset>",
            ));
            Flow::Cancel
        }
    }

    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent<'_>) {
        let value = emc::value(&event.stack.item);
        if value > 0 {
            event.lines.push((
                Text::literal(format!("§7EMC: §e{}", value)),
                u32::MAX,
            ));
        }
    }

    fn data_pack_files(&self, _host: &dyn Host, event: &mut DataPackFilesEvent<'_>) {
        for (name, ingredients, result, count) in RECIPES {
            let list = ingredients
                .iter()
                .map(|id| format!("\"{id}\""))
                .collect::<Vec<_>>()
                .join(",");
            let json = format!(
                "{{\"type\":\"minecraft:crafting_shapeless\",\"category\":\"misc\",\"ingredients\":[{list}],\"result\":{{\"id\":\"{result}\",\"count\":{count}}}}}"
            );
            event.files.push((
                format!("recipe/{MOD_ID}:{name}"),
                json.into_bytes(),
            ));
        }
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

const RECIPES: &[(&str, &[&str], &str, u32)] = &[
    (
        "philosophers_stone",
        &[
            "minecraft:glowstone_dust",
            "minecraft:glowstone_dust",
            "minecraft:glowstone_dust",
            "minecraft:glowstone_dust",
            "minecraft:diamond",
            "minecraft:redstone",
        ],
        PHILOSOPHERS_STONE,
        1,
    ),
    (
        "transmutation_table",
        &[
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            PHILOSOPHERS_STONE,
        ],
        TABLE,
        1,
    ),
    (
        "energy_collector",
        &[
            "minecraft:glowstone",
            "minecraft:glowstone",
            "minecraft:glowstone",
            "minecraft:glowstone",
            "minecraft:glowstone",
            "minecraft:glowstone",
            "minecraft:glowstone",
            "minecraft:glowstone",
            "minecraft:diamond",
        ],
        COLLECTOR,
        1,
    ),
    (
        "antimatter_relay",
        &[
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            "minecraft:obsidian",
            COLLECTOR,
        ],
        RELAY,
        1,
    ),
    (
        "alchemical_coal",
        &[
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:redstone",
        ],
        "projecte:alchemical_coal",
        4,
    ),
    (
        "mobius_fuel",
        &[
            "projecte:alchemical_coal",
            "projecte:alchemical_coal",
            "projecte:alchemical_coal",
            "projecte:alchemical_coal",
            "minecraft:glowstone_dust",
        ],
        "projecte:mobius_fuel",
        4,
    ),
    (
        "aeternalis_fuel",
        &[
            "projecte:mobius_fuel",
            "projecte:mobius_fuel",
            "projecte:mobius_fuel",
            "projecte:mobius_fuel",
            "minecraft:glowstone_dust",
        ],
        "projecte:aeternalis_fuel",
        4,
    ),
    (
        "dark_matter",
        &[
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:coal",
            "minecraft:diamond",
        ],
        "projecte:dark_matter",
        1,
    ),
    (
        "red_matter",
        &[
            "projecte:dark_matter",
            "projecte:dark_matter",
            "projecte:dark_matter",
            "projecte:dark_matter",
            "projecte:dark_matter",
            "projecte:dark_matter",
            "projecte:dark_matter",
            "projecte:dark_matter",
            "minecraft:diamond_block",
        ],
        "projecte:red_matter",
        1,
    ),
    (
        "klein_star_ein",
        &[
            "minecraft:diamond",
            "minecraft:diamond",
            "minecraft:diamond",
            "minecraft:diamond",
            "minecraft:diamond",
            "minecraft:diamond",
            "minecraft:diamond",
            "minecraft:diamond",
            "minecraft:glowstone",
        ],
        "projecte:klein_star_ein",
        1,
    ),
    (
        "klein_star_zwei",
        &[
            "projecte:klein_star_ein",
            "projecte:klein_star_ein",
            "projecte:klein_star_ein",
            "projecte:klein_star_ein",
            "minecraft:nether_star",
        ],
        "projecte:klein_star_zwei",
        1,
    ),
    (
        "klein_star_drei",
        &[
            "projecte:klein_star_zwei",
            "projecte:klein_star_zwei",
            "projecte:klein_star_zwei",
            "projecte:klein_star_zwei",
            "minecraft:nether_star",
        ],
        "projecte:klein_star_drei",
        1,
    ),
    (
        "klein_star_vier",
        &[
            "projecte:klein_star_drei",
            "projecte:klein_star_drei",
            "projecte:klein_star_drei",
            "projecte:klein_star_drei",
            "minecraft:nether_star",
        ],
        "projecte:klein_star_vier",
        1,
    ),
];
