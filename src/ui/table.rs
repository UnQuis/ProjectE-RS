//! The Transmutation Table's slot contract and its pure display/interaction
//! model. No Horizon types live here: the server hooks in `lib.rs` apply the
//! plans, so knowledge, EMC and lock state stay server-authoritative.
//!
//! Slot layout (27 container slots, vanilla `minecraft:generic_9x3`):
//! `0..8` input, `8` lock, `9` consume, `10` unlearn, `11..27` output.

use std::collections::BTreeSet;

pub(crate) const TABLE_SLOTS: usize = 27;
pub(crate) const INPUT_FIRST: usize = 0;
pub(crate) const INPUT_LAST: usize = 8;
pub(crate) const LOCK_SLOT: usize = 8;
pub(crate) const CONSUME_SLOT: usize = 9;
pub(crate) const UNLEARN_SLOT: usize = 10;
pub(crate) const OUTPUT_FIRST: usize = 11;
pub(crate) const OUTPUT_LAST: usize = 27;
pub(crate) const OUTPUT_COUNT: usize = OUTPUT_LAST - OUTPUT_FIRST;

/// The matter/fuel split ProjectE uses: 12 matter outputs then 4 fuel outputs.
pub(crate) const MATTER_DISPLAY: usize = 12;
pub(crate) const FUEL_DISPLAY: usize = 4;

/// Menu button ids sent by the table screen's pager (`MenuButtonContext.id`).
pub(crate) const BUTTON_PAGE_PREV: i32 = 0;
pub(crate) const BUTTON_PAGE_NEXT: i32 = 1;

/// Advance a page index without wrapping or underflowing.
pub(crate) fn advance_page(page: usize, forward: bool, last_page: usize) -> usize {
    if forward {
        page.saturating_add(1).min(last_page)
    } else {
        page.saturating_sub(1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotRole {
    Input,
    Lock,
    Consume,
    Unlearn,
    Output(usize),
    Player,
}

pub(crate) fn slot_role(index: usize) -> SlotRole {
    match index {
        INPUT_FIRST..INPUT_LAST => SlotRole::Input,
        LOCK_SLOT => SlotRole::Lock,
        CONSUME_SLOT => SlotRole::Consume,
        UNLEARN_SLOT => SlotRole::Unlearn,
        OUTPUT_FIRST..OUTPUT_LAST => SlotRole::Output(index - OUTPUT_FIRST),
        27..63 => SlotRole::Player,
        _ => SlotRole::Player,
    }
}

/// The two output sections, in ProjectE's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    Matter,
    Fuel,
}

/// The visible slice of each section for one page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DisplayPage {
    pub matter: Vec<String>,
    pub fuel: Vec<String>,
    pub has_next: bool,
    pub has_previous: bool,
}

/// Build the client-side display grid from server-synced knowledge.
///
/// Items are EMC-descending (ProjectE's only sort). A lock caps displayed EMC
/// unless it is itself known and affordable; a known lock is pinned to the top
/// of its section. Two independent sections paginate: 12 matter and 4 fuel
/// slots. This is pure so the page arithmetic is covered by tests.
pub(crate) fn display_page(
    known: &BTreeSet<String>,
    value: impl Fn(&str) -> u64,
    is_fuel: impl Fn(&str) -> bool,
    available_emc: i64,
    lock: Option<&str>,
    matches: impl Fn(&str) -> bool,
    page: usize,
) -> DisplayPage {
    let mut matter: Vec<String> = Vec::new();
    let mut fuel: Vec<String> = Vec::new();
    let mut pinned: Option<String> = None;
    // A lock with EMC caps the displayed grid; an unaffordable lock is ignored
    // (ProjectE's `updateClientTargets`).
    let lock_cap = lock
        .filter(|id| value(id) > 0)
        .filter(|id| i64::try_from(value(id)).map_or(false, |emc| emc <= available_emc))
        .map(|id| value(id));

    for item in known {
        let emc = value(item);
        if emc == 0 || i64::try_from(emc).map_or(true, |value| value > available_emc) {
            continue;
        }
        if lock_cap.is_some_and(|cap| emc > cap) {
            continue;
        }
        if !matches(item.as_str()) {
            continue;
        }
        // A known, affordable lock is pinned to the top of its section.
        if let Some(lock_id) = lock
            && item == lock_id
            && known.contains(lock_id)
        {
            pinned = Some(item.clone());
            continue;
        }
        if is_fuel(item) {
            fuel.push(item.clone());
        } else {
            matter.push(item.clone());
        }
    }
    // EMC descending, id as a deterministic tie-breaker.
    let sort = |a: &String, b: &String| value(b).cmp(&value(a)).then_with(|| a.cmp(b));
    fuel.sort_by(sort);
    matter.sort_by(sort);
    if let Some(item) = pinned {
        if is_fuel(&item) {
            fuel.insert(0, item);
        } else {
            matter.insert(0, item);
        }
    }

    let matter_start = page * MATTER_DISPLAY;
    let fuel_start = page * FUEL_DISPLAY;
    let matter_page: Vec<String> = matter
        .iter()
        .skip(matter_start)
        .take(MATTER_DISPLAY)
        .cloned()
        .collect();
    let fuel_page: Vec<String> = fuel
        .iter()
        .skip(fuel_start)
        .take(FUEL_DISPLAY)
        .cloned()
        .collect();
    DisplayPage {
        has_next: matter.len() > matter_start + MATTER_DISPLAY
            || fuel.len() > fuel_start + FUEL_DISPLAY,
        has_previous: page > 0,
        matter: matter_page,
        fuel: fuel_page,
    }
}

/// A stack offered to the consume slot: learn it and credit its sell value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConsumePlan {
    pub item: String,
    pub credited_emc: i64,
}

/// Plan a consume-slot operation. `value` is per item; the count is multiplied
/// with checked arithmetic and the whole operation is refused on overflow.
pub(crate) fn plan_consume(
    item: &str,
    count: u32,
    value: impl Fn(&str) -> u64,
) -> Result<ConsumePlan, &'static str> {
    if count == 0 {
        return Err("§cThere is nothing in the consume slot.");
    }
    let unit = value(item);
    if unit == 0 {
        return Err("§cThat item has no EMC value and cannot be consumed.");
    }
    // Selling an item applies ProjectE's covalence loss; buying uses the full
    // value. Both are server-authoritative — this planner only computes.
    let sell = crate::config::covalence()
        .sell_value(unit)
        .ok_or("§cThat stack is too valuable to count.")?;
    let total = crate::emc::checked_cost(sell, count).ok_or("§cThat stack is too valuable to count.")?;
    Ok(ConsumePlan {
        item: item.to_owned(),
        credited_emc: total,
    })
}

/// Plan an unlearn-slot operation: an item with EMC can be forgotten.
pub(crate) fn plan_unlearn(
    item: &str,
    player: &crate::data::PlayerData,
    value: impl Fn(&str) -> u64,
) -> Result<String, &'static str> {
    if value(item) == 0 {
        return Err("§cThat item has no EMC value and cannot be unlearned.");
    }
    if !player.has_knowledge(item) {
        return Err("§cYou have not learned that item yet.");
    }
    Ok(item.to_owned())
}

/// Learn an item with the server's validation (blacklist, EMC value, Tome).
pub(crate) fn learn_for_consume(
    player: &mut crate::data::PlayerData,
    item: &str,
    has_value: impl Fn(&str) -> bool,
) -> bool {
    player.learn(item, &BTreeSet::new(), has_value)
}

/// A lock slot only accepts a known item with EMC, or clears the lock when empty.
pub(crate) fn plan_lock(
    item: Option<&str>,
    player: &crate::data::PlayerData,
    value: impl Fn(&str) -> u64,
) -> Result<Option<String>, &'static str> {
    match item {
        None => Ok(None),
        Some(id) if value(id) == 0 => Err("§cOnly items with EMC can be used as a lock."),
        Some(id) if !player.has_knowledge(id) => Err("§cYou must learn an item before locking it."),
        Some(id) => Ok(Some(id.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(id: &str) -> u64 {
        match id {
            "a" => 1,
            "b" => 2,
            "c" => 3,
            "d" => 4,
            "e" => 5,
            "cap" => 3,
            "fuel:1" => 10,
            "fuel:2" => 20,
            "fuel:3" => 30,
            "fuel:4" => 40,
            "fuel:5" => 50,
            _ => 0,
        }
    }

    fn is_fuel(id: &str) -> bool {
        id.starts_with("fuel:")
    }

    fn known(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    fn player(ids: &[&str]) -> crate::data::PlayerData {
        crate::data::PlayerData {
            emc: i64::MAX,
            learned: known(ids),
            lock: None,
            full_knowledge: false,
        }
    }

    #[test]
    fn the_pager_buttons_never_leave_the_page_range() {
        assert_eq!(advance_page(0, false, 3), 0);
        assert_eq!(advance_page(0, true, 3), 1);
        assert_eq!(advance_page(2, true, 3), 3);
        assert_eq!(advance_page(3, true, 3), 3);
        assert_eq!(advance_page(3, false, 3), 2);
    }

    #[test]
    fn slot_roles_cover_the_whole_table() {
        assert_eq!(slot_role(0), SlotRole::Input);
        assert_eq!(slot_role(7), SlotRole::Input);
        assert_eq!(slot_role(8), SlotRole::Lock);
        assert_eq!(slot_role(9), SlotRole::Consume);
        assert_eq!(slot_role(10), SlotRole::Unlearn);
        assert_eq!(slot_role(11), SlotRole::Output(0));
        assert_eq!(slot_role(26), SlotRole::Output(15));
        assert_eq!(slot_role(27), SlotRole::Player);
        assert_eq!(slot_role(62), SlotRole::Player);
    }

    #[test]
    fn the_two_sections_paginate_independently() {
        let ids = ["a", "b", "c", "d", "e", "fuel:1", "fuel:2", "fuel:3", "fuel:4", "fuel:5"];
        let set = known(&ids);
        let all = |_id: &str| true;
        let first = display_page(&set, values, is_fuel, i64::MAX, None, all, 0);
        assert_eq!(first.matter, ["e", "d", "c", "b", "a"]);
        assert_eq!(first.fuel, ["fuel:5", "fuel:4", "fuel:3", "fuel:2"]);
        assert!(first.has_next, "fuel:1 overflows the four fuel slots");
        assert!(!first.has_previous);
        let second = display_page(&set, values, is_fuel, i64::MAX, None, all, 1);
        assert!(second.matter.is_empty());
        assert_eq!(second.fuel, ["fuel:1"]);
        assert!(!second.has_next);
        assert!(second.has_previous);
    }

    #[test]
    fn unaffordable_and_unknown_items_stay_out_of_the_display() {
        let set = known(&["a", "b", "c"]);
        let all = |_id: &str| true;
        let page = display_page(&set, values, is_fuel, 2, None, all, 0);
        assert_eq!(page.matter, ["b", "a"]);
        assert!(!page.has_next);
    }

    #[test]
    fn a_known_lock_is_pinned_and_an_unknown_lock_caps_the_grid() {
        let set = known(&["a", "b", "c"]);
        let all = |_id: &str| true;
        let pinned = display_page(&set, values, is_fuel, i64::MAX, Some("a"), all, 0);
        assert_eq!(pinned.matter.first().map(String::as_str), Some("a"));
        // An unknown lock is treated as "no lock" by the reference when its
        // EMC requirement is not affordable; we simply keep the affordable set.
        let unknown = display_page(&set, values, is_fuel, i64::MAX, Some("missing"), all, 0);
        assert_eq!(unknown.matter.len(), 3);
        // A lock with EMC caps the grid inclusively; the lock itself is never
        // duplicated because it is pinned and skipped in the main loop.
        let capped = display_page(&set, values, is_fuel, i64::MAX, Some("cap"), all, 0);
        assert_eq!(capped.matter, ["c", "b", "a"]);
        // An unaffordable lock is ignored.
        let ignored = display_page(&set, values, is_fuel, 2, Some("cap"), all, 0);
        assert_eq!(ignored.matter, ["b", "a"]);
    }

    #[test]
    fn consume_learns_and_credits_only_what_fits() {
        let plan = plan_consume("b", 3, values).unwrap();
        assert_eq!(plan.item, "b");
        assert_eq!(plan.credited_emc, 6);
        assert_eq!(plan_consume("b", 0, values).unwrap_err(), "§cThere is nothing in the consume slot.");
        assert!(plan_consume("z", 1, values).is_err());
        assert!(plan_consume("b", u32::MAX, |_| u64::MAX).is_err());
    }

    #[test]
    fn unlearn_requires_a_known_valued_item() {
        assert_eq!(plan_unlearn("b", &player(&["b"]), values).unwrap(), "b");
        assert!(plan_unlearn("a", &player(&["b"]), values).is_err());
        assert!(plan_unlearn("z", &player(&["b"]), values).is_err());
    }

    #[test]
    fn lock_requires_a_known_valued_item_or_clears() {
        assert_eq!(plan_lock(None, &player(&["b"]), values).unwrap(), None);
        assert_eq!(
            plan_lock(Some("b"), &player(&["b"]), values).unwrap(),
            Some("b".to_owned())
        );
        assert!(plan_lock(Some("a"), &player(&["b"]), values).is_err());
        assert!(plan_lock(Some("z"), &player(&["b"]), values).is_err());
        // A full-knowledge player may lock any valued item.
        let mut all = player(&[]);
        all.full_knowledge = true;
        assert_eq!(
            plan_lock(Some("a"), &all, values).unwrap(),
            Some("a".to_owned())
        );
    }
}
