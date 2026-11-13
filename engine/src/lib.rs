//! ProjectE-RS: Equivalent Exchange for Horizon.
//!
//! This is a native trait mod. It declares its blocks and items in code, so
//! the repository contains no textures, models or other binary assets; every
//! declaration borrows a vanilla template's look.

use std::sync::Arc;

use mod_api::*;

mod alchemical;
pub mod condenser;
mod data;
mod deposit;
mod deposit_menu;
mod emc;
mod energy;
mod machines;
mod progression;
mod recipes;
mod transaction;
#[allow(dead_code)] // The UI is a deliberate shell until its gameplay hooks are wired.
mod ui;

const TABLE: &str = recipes::TABLE;
const PHILOSOPHERS_STONE: &str = alchemical::PHILOSOPHERS_STONE;

/// The `/projecte` browsing page's slots, which this mod draws as a menu of its
/// own. Not the Transmutation Table's menu: that one is
/// `ui::atlas::MENU_SLOTS` (nine), and the two never share a screen.
const PROJECTE_MENU_SLOTS: usize = 54;

/// The table block entity's saved-data id. Its id is the table's, so a later
/// revision can store the table's own state without touching another block's.
const TABLE_BLOCK_ENTITY: &str = "projecte:transmutation_table";

#[derive(Default)]
struct ProjectE;

#[mod_api::horizon_mod]
impl Hooks for ProjectE {
    fn blocks(&self, event: &mut BlocksEvent<'_>) {
        event.blocks.push(Default::default());
        let table = event.blocks.last_mut().unwrap();
        table.id = TABLE.to_string();
        table.template = "minecraft:crafting_table".to_string();
        table.item = true;

        for id in machines::COLLECTORS
            .into_iter()
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
        let fuel_templates = [
            "minecraft:coal",
            "minecraft:blaze_powder",
            "minecraft:glowstone_dust",
        ];
        for (fuel, template) in alchemical::FUEL_TIERS.iter().skip(1).zip(fuel_templates) {
            item(event, fuel, template, 64);
        }
    }

    fn component_types(&self, event: &mut ComponentTypesEvent<'_>) {
        if !event
            .types
            .iter()
            .any(|id| id == energy::STORED_EMC_COMPONENT)
        {
            event.types.push(energy::STORED_EMC_COMPONENT.to_string());
        }
    }

    fn block_entity(
        &self,
        _host: &mut dyn Host,
        event: &mut BlockEntityEvent<'_>,
    ) -> Option<(String, Compound)> {
        if event.block == TABLE {
            // The table keeps the screen's slots, which are its menu's. Its
            // block entity is its own id, so its saved data is its own; the
            // screen stores nothing in it today.
            return Some((TABLE_BLOCK_ENTITY.to_owned(), Compound::new()));
        }
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
        if event.block == TABLE {
            return Some(table_container());
        }
        machines::container(event.block)
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
        machines::tick(host, event);
    }

    fn right_click_block(&self, host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
        let block = host.block(event.pos);
        if block == TABLE {
            // The table's menu is its block entity's container (below), which
            // the engine opens on a right-click by itself; the hook leaves the
            // click alone so that path runs. Nothing here opens a menu, so the
            // screen and the container cannot disagree about what is open.
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
        // The stone is used on blocks (world transmutation) and in the
        // crafting grid; it does not open the table. The table's screen is its
        // block's own menu, which the engine opens when the block is used.
        let _ = host.acting_player();
        Flow::Continue
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

    fn menu_closed(&self, _host: &mut dyn Host, event: &mut MenuClosedEvent) {
        // The window is gone, so its slot 18 must stop being a confirm
        // control: a later menu that reuses the id is not this mod's.
        deposit_menu::forget(event.uuid, event.window);
    }

    fn menu_input_closed(&self, _host: &mut dyn Host, event: &mut MenuInputClosedEvent<'_>) {
        deposit_menu::forget(event.uuid, event.window);
        // Nothing is taken out of `event.slots`: every item still in an input
        // slot was never paid for, so the engine must give it back to the
        // player. Keeping it here would be the one way this mod could erase an
        // item without crediting EMC for it.
        let _ = &event.slots;
    }

    fn menu_clicked(&self, host: &mut dyn Host, event: &mut MenuClickedEvent) {
        // The deposit menu first: its confirm slot is the only click in the
        // mod that changes EMC or knowledge.
        if deposit_menu::confirm_slot(event.uuid, event.window, event.slot).is_some() {
            let contents = host.menu_contents(event.uuid).unwrap_or_default();
            let result = deposit_menu::apply(host, event.uuid, contents);
            deposit_menu::report(host, event.uuid, result);
            return;
        }
        if !(0..PROJECTE_MENU_SLOTS as i32).contains(&event.slot) {
            return;
        }
        let data = data::get(host, event.uuid);
        let learned = match learned_items(host, &data) {
            Ok(items) => items,
            Err(error) => {
                host.tell(
                    event.uuid,
                    Text::literal(format!("§cEMC data map is unavailable ({error:?}).")),
                );
                return;
            }
        };
        let Some((item, value)) = learned.get(event.slot as usize) else {
            return;
        };
        let count: u32 = if event.mode == 1 {
            64
        } else if event.button == 1 {
            16
        } else {
            1
        };
        let plan = match transaction::plan_purchase(
            *value,
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
        let reserved = plan.maximum_cost();
        if !data::take_emc(host, event.uuid, reserved) {
            host.tell(
                event.uuid,
                Text::literal("§cYour EMC balance changed; the purchase was cancelled."),
            );
            return;
        }
        let leftover = host.give_stack(event.uuid, stack(item, count));
        let leftover_count = leftover.as_ref().map_or(0, |s| s.count);
        let Some(given) = count.checked_sub(leftover_count) else {
            data::add_emc(host, event.uuid, reserved);
            host.tell(
                event.uuid,
                Text::literal("§cThe inventory returned an invalid item count."),
            );
            return;
        };
        let settlement = match transaction::settle_purchase(plan, given, data.emc) {
            Ok(settlement) => settlement,
            Err(error) => {
                data::add_emc(host, event.uuid, reserved);
                host.tell(event.uuid, Text::literal(format!("§c{}", error.message())));
                return;
            }
        };
        let refund = reserved - settlement.spent;
        let balance = if refund > 0 {
            data::add_emc(host, event.uuid, refund)
        } else {
            data::get(host, event.uuid).emc
        };
        let spent = settlement.spent;
        let given = settlement.granted;
        host.tell(
            event.uuid,
            Text::literal(format!(
                "§aBought §e{}x {}§a for §e{} EMC§a. Balance: §e{}§a.",
                given,
                emc::display_name(item),
                spent,
                balance
            )),
        );
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(NativeCommand::new("projecte emc [player:player]"));
        event.commands.push(NativeCommand::op(
            "projecte emc set <player:player> <amount:integer>",
            2,
        ));
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
        event.commands.push(NativeCommand::new("projecte reset"));
        event.commands.push(NativeCommand::new("projecte roadmap"));
        event.commands.push(NativeCommand::new("projecte deposit"));
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        let parts: Vec<&str> = event.command.split_whitespace().collect();
        if parts.first().copied() != Some("projecte") {
            return Flow::Continue;
        }
        let result = match parts.get(1).copied() {
            Some("emc") => command_emc(host, event, &parts[2..]),
            Some("learn") => command_learn(host, event, &parts[2..]),
            Some("transmute") => command_transmute(host, event, &parts[2..]),
            Some("table") => {
                host.feedback(Text::literal(
                    "§7Use the Transmutation Table block to open its screen.",
                ));
                true
            }
            Some("deposit") => {
                // A deposit needs the engine's own menu: the player fills the
                // input slots by vanilla's click rules and confirms with a
                // real click, so nothing here touches an item itself.
                match deposit_menu::open(host, event.player.uuid) {
                    Some(_window) => {
                        host.feedback(Text::literal(
                            "§aPut items in the top nine slots, then click the green slot to deposit them.",
                        ));
                    }
                    None => host.feedback(Text::literal(
                        "§cThe deposit menu could not be opened for you.",
                    )),
                }
                true
            }
            Some("value") => {
                let Some(id) = parts.get(2) else {
                    host.feedback(Text::literal("§cUsage: /projecte value <item>"));
                    return Flow::Cancel;
                };
                let id = normalize_item(id);
                match emc::value(host, &id) {
                    Ok(Some(value)) if value > 0 => host.feedback(Text::literal(format!(
                        "§e{}§7: §e{} EMC",
                        emc::display_name(&id),
                        value
                    ))),
                    Ok(_) => host.feedback(Text::literal(format!(
                        "§7{} has no EMC value.",
                        emc::display_name(&id)
                    ))),
                    Err(error) => host.feedback(Text::literal(format!(
                        "§cCould not read projecte:emc ({error:?})."
                    ))),
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
            let stored = energy::stored_emc(host, event.stack).unwrap_or(0);
            event.lines.push((
                Text::literal(format!("§bStored EMC: §f{stored} / {capacity}")),
                u32::MAX,
            ));
        }
        // Data maps are not synced to clients; never show stale built-in values.
        if let Ok(Some(value)) = emc::value(host, &event.stack.item) {
            if value > 0 {
                event
                    .lines
                    .push((Text::literal(format!("§7EMC: §e{}", value)), u32::MAX));
            }
            if event.stack.count > 1
                && let Some(total) = alchemical::total_value(value, event.stack.count)
            {
                event
                    .lines
                    .push((Text::literal(format!("§7Stack EMC: §e{total}")), u32::MAX));
            }
        }
        if let Some(tier) = alchemical::tier(&event.stack.item) {
            let burn = alchemical::burn_ticks(&event.stack.item).unwrap_or(0);
            let previous = alchemical::previous_tier(&event.stack.item).unwrap_or("none");
            let next = alchemical::next_tier(&event.stack.item).unwrap_or("none");
            let block = alchemical::fuel_block(&event.stack.item).unwrap_or("none");
            event.lines.push((
                Text::literal(format!(
                    "§6Fuel tier {} · {} reference burn ticks · prev: {} · next: {} · block: {}",
                    tier + 1,
                    burn,
                    previous,
                    next,
                    block
                )),
                u32::MAX,
            ));
        }
        if let Some(fuel) = alchemical::fuel_of_block(&event.stack.item) {
            event
                .lines
                .push((Text::literal(format!("§6Packed fuel: {fuel}")), u32::MAX));
        }
        if &*event.stack.item == PHILOSOPHERS_STONE {
            let example = alchemical::STONE_CONVERSIONS
                .first()
                .map(|conversion| {
                    format!(
                        "{}x{} -> {}x{}",
                        conversion.from_count, conversion.from, conversion.to_count, conversion.to
                    )
                })
                .unwrap_or_else(|| "none".to_string());
            event.lines.push((
                Text::literal(format!(
                    "§7World swaps and {} reversible crafting conversions (e.g. {example})",
                    alchemical::STONE_CONVERSIONS.len()
                )),
                u32::MAX,
            ));
        }
    }

    fn data_map_types(
        &self,
        _host: &mut dyn ServerHost,
        event: &mut DataMapTypesEvent<'_>,
    ) {
        event.types.push(emc::MAP.ty());
    }

    fn data_pack_files(
        &self,
        _host: &mut dyn ServerHost,
        event: &mut DataPackFilesEvent<'_>,
    ) {
        event.files.extend(recipes::files());
        event.files.extend(emc::files());
    }

    // --- the Transmutation Table screen -----------------------------------
    //
    // The table's menu screen, drawn by this mod. Every hook delegates to
    // `ui::screen`, which keeps the one screen state and checks that the open
    // menu is the table's, so these are one line each.

    /// `native_screen` (API 19): lay out the table's nine-slot menu — where its
    /// slots are and where the player's inventory starts. Asked on the client
    /// and on the server, so the slots the client draws and the slots the
    /// server has are the same 45.
    fn screen(&self, _host: &dyn Host, event: &mut ScreenEvent<'_>) -> Option<NativeScreen> {
        // `native_screen`'s output is `&mut Option<NativeScreen>` with the
        // `replace` contract, so the trait returns the output's inner type and
        // the out argument is not an event field. The block names the screen;
        // the menu's title stays the server's.
        ui::screen::screen(event.block)
    }

    /// `native_screen_draw` (API 19): draw the table's screen over its menu.
    fn screen_draw(&self, host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
        ui::screen::screen_draw(host, event.batch, event.block, event.data, event.origin);
    }

    /// `native_screen_tooltip` (API 20): a line for what is under the pointer.
    fn screen_tooltip(&self, host: &dyn Host, event: &mut ScreenTooltipEvent<'_>) {
        ui::screen::screen_tooltip(host, event.block, event.data, event.at, event.lines);
    }

    /// `native_screen_opening` (API 367): the table's menu opened.
    fn screen_opening(&self, host: &dyn Host, event: &mut ScreenOpeningEvent<'_>) {
        ui::screen::screen_opening(host, event.screen);
    }

    /// `native_screen_closing` (API 365): the table's menu closed.
    fn screen_closing(&self, host: &dyn Host, event: &mut ScreenClosingEvent<'_>) {
        ui::screen::screen_closing(host, event.screen);
    }

    /// `native_screen_mouse_button_pressed_pre` (API 80): a click on the
    /// screen. A press on one of the screen's own parts is cancelled, so the
    /// same click does not also reach the menu slot under it.
    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        ui::screen::mouse_pressed(host, event.screen, event.button, event.modifiers)
    }

    /// `native_screen_mouse_button_released_pre` (API 80): the button came up.
    fn screen_mouse_button_released_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        ui::screen::mouse_released(host, event.screen, event.button, event.modifiers)
    }

    /// `native_screen_char_typed_pre` (API 104): a character typed at the
    /// screen, taken by the search field while it has the focus.
    fn screen_char_typed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenCharTypedPreEvent<'_>,
    ) -> Flow {
        ui::screen::char_typed(host, event.screen, event.c, event.modifiers)
    }

    /// `native_screen_key_pressed_pre` (API 104): Backspace, Enter and Escape
    /// belong to the search field while it has the focus.
    fn screen_key_pressed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        ui::screen::key_pressed(host, event.screen, event.key, event.repeat, event.modifiers)
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

fn learned_items(
    host: &dyn Host,
    data: &data::PlayerData,
) -> Result<Vec<(String, u64)>, DataMapError> {
    let mut items = Vec::with_capacity(data.learned.len());
    for id in &data.learned {
        if let Some(value) = emc::value(host, id)? {
            if value > 0 {
                items.push((id.clone(), value));
            }
        }
    }
    items.sort_by(|(left_id, left_value), (right_id, right_value)| {
        right_value
            .cmp(left_value)
            .then_with(|| left_id.cmp(right_id))
    });
    Ok(items)
}

/// The table's container: the slots its screen lays out.
///
/// Its menu is vanilla's six-row chest (`minecraft:generic_9x6`), so a stock
/// client shows the same menu, and Horizon's clients draw the mod's own screen
/// over it (`ui::atlas::native_screen`). The menu opens when the block is used,
/// because the block's template opens a menu, as a chest's does.
///
/// Only the slots the screen draws are listed as inputs: the browser's cells
/// are display-only to hoppers, and the two real input slots are what a hopper
/// may fill. Nothing here runs crafting, learning or transmutation yet.
fn table_container() -> ModContainer {
    ModContainer {
        slots: ui::atlas::MENU_SLOTS,
        // No face reaches any slot: a hopper cannot pull an item out of the
        // table's storage behind the player's back. The two inputs are named
        // as the faces' fill targets for the API's sake; with every face list
        // empty nothing is ever pushed in.
        faces: [
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ],
        inputs: ui::atlas::INPUT_SLOTS.to_vec(),
        outputs: Vec::new(),
        menu: Some(ui::atlas::MENU.to_owned()),
    }
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
    room = room
        .saturating_add(max.saturating_mul(stacks.iter().filter(|s| s.is_none()).count() as u32));
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
        let prerequisites = if node.prerequisites.is_empty() {
            "none".to_string()
        } else {
            node.prerequisites.join(", ")
        };
        host.feedback(Text::literal(format!(
            "§7[S{}:{}] [{}] §e{}§7 - {} | needs: {} | API: {}",
            node.stage,
            node.id,
            node.status.label(),
            node.name,
            node.unlocks,
            prerequisites,
            node.horizon
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
        learn.extend(
            host.carried_items()
                .into_iter()
                .map(|(id, _)| normalize_item(&id)),
        );
    } else if let Some(raw) = args.first() {
        learn.push(normalize_item(raw));
    } else if let Some((id, _, _)) = host.held() {
        learn.push(id);
    } else {
        host.feedback(Text::literal("§cHold an item, or use /projecte learn all."));
        return true;
    }

    let mut data = data::get(host, event.player.uuid);
    let mut added = 0;
    let mut no_value = 0;
    for id in learn {
        match emc::value(host, &id) {
            Ok(Some(value)) if value > 0 => {}
            Ok(_) => {
                no_value += 1;
                continue;
            }
            Err(error) => {
                host.feedback(Text::literal(format!(
                    "§cCould not read projecte:emc ({error:?}); no knowledge was changed."
                )));
                return true;
            }
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
        host.feedback(Text::literal("§cUsage: /projecte transmute <item> [count]"));
        return true;
    };
    let id = normalize_item(raw);
    let count = args
        .get(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, 4096);
    let value = match emc::value(host, &id) {
        Ok(Some(value)) if value > 0 => value,
        Ok(_) => {
            host.feedback(Text::literal(format!(
                "§c{} has no EMC value.",
                emc::display_name(&id)
            )));
            return true;
        }
        Err(error) => {
            host.feedback(Text::literal(format!(
                "§cCould not read projecte:emc ({error:?}); transmutation was cancelled."
            )));
            return true;
        }
    };
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
    let reserved = plan.maximum_cost();
    if !data::take_emc(host, event.player.uuid, reserved) {
        host.feedback(Text::literal(
            "§cYour EMC balance changed; the transmutation was cancelled.",
        ));
        return true;
    }
    let leftover = host.give_stack(event.player.uuid, stack(&id, count));
    let leftover_count = leftover.as_ref().map_or(0, |s| s.count);
    let Some(given) = count.checked_sub(leftover_count) else {
        data::add_emc(host, event.player.uuid, reserved);
        host.feedback(Text::literal(
            "§cThe inventory returned an invalid item count.",
        ));
        return true;
    };
    let settlement = match transaction::settle_purchase(plan, given, data.emc) {
        Ok(settlement) => settlement,
        Err(error) => {
            data::add_emc(host, event.player.uuid, reserved);
            host.feedback(Text::literal(format!("§c{}", error.message())));
            return true;
        }
    };
    let refund = reserved - settlement.spent;
    let balance = if refund > 0 {
        data::add_emc(host, event.player.uuid, refund)
    } else {
        data::get(host, event.player.uuid).emc
    };
    let spent = settlement.spent;
    let given = settlement.granted;
    host.feedback(Text::literal(format!(
        "§aTransmuted §e{}x {}§a for §e{} EMC§a. Balance: §e{}§a.",
        given,
        emc::display_name(&id),
        spent,
        balance
    )));
    true
}
