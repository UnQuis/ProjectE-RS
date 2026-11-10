//! Deposit-and-learn: turning items into EMC and knowledge.
//!
//! This module is the **pure** half of the deposit flow. It decides, from the
//! contents of the table's deposit slots and the player's current balance, what
//! a deposit would do — and it decides it before anything is mutated, so the
//! screen path can apply an all-or-nothing plan and the tests can cover the
//! arithmetic without a game.
//!
//! # The rule
//!
//! A deposit takes **whole stacks** out of the table's deposit slots. For each
//! stack it computes `count × value` with checked arithmetic; if every stack is
//! payable the plan names the exact counts, the total EMC and the item ids to
//! learn. If any stack cannot be paid for — no EMC value, or the total
//! overflows `i64` — the whole deposit is refused and **nothing** is spent,
//! nothing is learned and no item is removed. Partial application on a failed
//! plan is the one outcome that would lose items, so it cannot happen here by
//! construction: `plan` is pure, and the caller applies exactly what it returns.
//!
//! # Why the caller must read back after removing
//!
//! The plan says what *should* leave the deposit slots. The caller then writes
//! the emptied slots back and reads the menu again. Only what actually left is
//! credited ([`settle`]), so a write the engine ignored cannot mint EMC or
//! knowledge for items the player still holds.

use std::collections::BTreeSet;

/// One stack offered for deposit: the item's id and how many are in the slot.
///
/// `slot` is the menu slot the stack sits in, so the caller can clear exactly
/// that one and no other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Offered {
    pub slot: usize,
    pub item: String,
    pub count: u32,
}

/// What a deposit would do, if applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DepositPlan {
    /// The stacks to clear, as offered: the caller empties these slots.
    pub taken: Vec<Offered>,
    /// The EMC the whole deposit pays, already checked for overflow.
    pub emc: i64,
    /// The item ids the deposit teaches. Already-known ids are included: the
    /// caller treats an id it already knows as no-op, so a repeat deposit pays
    /// EMC and does not duplicate knowledge.
    pub learned: Vec<String>,
}

/// Why a deposit was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DepositError {
    /// Nothing was in the deposit slots.
    Empty,
    /// At least one item has no EMC value; the payload is its id.
    NoValue(String),
    /// `count × value` or the running total did not fit in `i64`.
    Overflow,
    /// The balance would pass `i64::MAX`; refused rather than wrapping.
    BalanceOverflow { credited: i64, balance: i64 },
}

impl DepositError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::Empty => "§cPut items in the table's slots to deposit them.".to_owned(),
            Self::NoValue(item) => {
                let name = item.rsplit(':').next().unwrap_or(item).replace('_', " ");
                format!("§c{name} has no EMC value, so it cannot be deposited.")
            }
            Self::Overflow => "§cThat deposit is too large to count.".to_owned(),
            Self::BalanceOverflow { credited, balance } => format!(
                "§cThat would take your EMC past its limit \
                 (§e{credited}§c on top of §e{balance}§c)."
            ),
        }
    }
}

/// Plans a deposit from the stacks in the deposit slots and the player's
/// current balance.
///
/// `value` answers an item's EMC value, and is passed in so this stays a pure
/// function of its inputs (the caller passes [`crate::emc::value`]).
///
/// Whole slots are taken, never part of one: a deposit slot is emptied or left
/// alone, which is what makes the removal easy to verify afterwards.
pub(crate) fn plan(
    offered: &[Offered],
    balance: i64,
    value: impl Fn(&str) -> u64,
) -> Result<DepositPlan, DepositError> {
    let mut total: i64 = 0;
    let mut taken = Vec::new();
    let mut learned = BTreeSet::new();

    for stack in offered {
        if stack.count == 0 {
            // An empty stack is not a deposit; skip it rather than refusing,
            // so a stale empty slot never blocks a real one.
            continue;
        }
        let unit = value(&stack.item);
        // A zero value is not a deposit: paying nothing for an item would let
        // a player destroy items and call it a feature.
        if unit == 0 {
            return Err(DepositError::NoValue(stack.item.clone()));
        }
        // `checked_cost` returns `Option<i64>`: it already refuses a product
        // that would not fit, so this only has to refuse the running total.
        let paid = crate::emc::checked_cost(unit, stack.count).ok_or(DepositError::Overflow)?;
        total = total.checked_add(paid).ok_or(DepositError::Overflow)?;
        taken.push(stack.clone());
        learned.insert(stack.item.clone());
    }

    if taken.is_empty() {
        return Err(DepositError::Empty);
    }
    if balance.checked_add(total).is_none() {
        return Err(DepositError::BalanceOverflow {
            credited: total,
            balance,
        });
    }

    Ok(DepositPlan {
        taken,
        emc: total,
        learned: learned.into_iter().collect(),
    })
}

/// What a deposit actually credited, after the caller reports what left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DepositSettlement {
    /// The EMC credited: only for the items that verifiably left.
    pub credited: i64,
    /// The planned total, which the caller must *not* credit: it is here so a
    /// caller can tell the player how much short of the plan it fell.
    pub planned: i64,
    /// The item ids newly added to the learned set (ids already known are not
    /// repeated here).
    pub newly_learned: Vec<String>,
    /// True when items left but the balance would have overflowed, so the
    /// caller must undo the removal instead of crediting.
    pub refused: bool,
}

/// Settles a planned deposit against what the caller **observed** leave the
/// slots.
///
/// `remaining` answers, per offered slot, how many of the item are still in the
/// slot after the caller cleared it and read the menu back. This is what keeps
/// a failed or ignored write from minting EMC: an item that is still in the
/// slot was never deposited.
///
/// If the observed departure does not match the plan, this credits only what
/// left, so the result is always ≤ the plan. When nothing left, the credit is
/// zero and the caller must tell the player the deposit failed.
pub(crate) fn settle(
    plan: &DepositPlan,
    value: impl Fn(&str) -> u64,
    mut remaining: impl FnMut(&Offered) -> u32,
    known: &BTreeSet<String>,
) -> DepositSettlement {
    let mut credited: i64 = 0;
    let mut newly_learned = Vec::new();

    for stack in &plan.taken {
        let left = remaining(stack).min(stack.count);
        let departed = stack.count - left;
        if departed == 0 {
            continue;
        }
        let unit = value(&stack.item);
        // Counted with the same function as `plan`, which already proved the
        // whole stack fits; a smaller count cannot overflow either.
        let Some(paid) = crate::emc::checked_cost(unit, departed) else {
            continue;
        };
        // Every partial sum was verified in `plan`, and `departed` is at most
        // the planned count, so the credit cannot exceed the plan and cannot
        // overflow. Checked rather than saturating: a stored `i64::MAX` would
        // be a lie, so refuse to add instead of capping.
        let Some(next) = credited.checked_add(paid) else {
            continue;
        };
        credited = next;
        // Wholly departed leads to confident knowledge; a partial departure
        // still teaches the item, because the player did give some of it up.
        if !known.contains(&stack.item) && !newly_learned.contains(&stack.item) {
            newly_learned.push(stack.item.clone());
        }
    }

    DepositSettlement {
        credited,
        planned: plan.emc,
        newly_learned,
        refused: credited == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(id: &str) -> u64 {
        match id {
            "minecraft:diamond" => 8192,
            "minecraft:coal" => 128,
            "minecraft:dirt" => 1,
            _ => 0,
        }
    }

    fn offered(slot: usize, item: &str, count: u32) -> Offered {
        Offered {
            slot,
            item: item.to_owned(),
            count,
        }
    }

    #[test]
    fn a_single_stack_pays_its_whole_count_and_teaches_the_item() {
        let plan = plan(&[offered(0, "minecraft:diamond", 3)], 0, values).unwrap();
        assert_eq!(plan.emc, 24_576);
        assert_eq!(plan.learned, vec!["minecraft:diamond"]);
        assert_eq!(plan.taken.len(), 1);
    }

    #[test]
    fn several_stacks_add_up_and_teach_each_item_once() {
        let plan = plan(
            &[
                offered(0, "minecraft:diamond", 1),
                offered(1, "minecraft:coal", 4),
                offered(2, "minecraft:diamond", 2),
            ],
            0,
            values,
        )
        .unwrap();
        assert_eq!(plan.emc, 8192 + 512 + 16_384);
        // Two diamond slots, one id.
        assert_eq!(plan.learned, vec!["minecraft:coal", "minecraft:diamond"]);
    }

    #[test]
    fn an_item_without_a_value_refuses_the_whole_deposit() {
        let err = plan(
            &[
                offered(0, "minecraft:diamond", 1),
                offered(1, "minecraft:air", 64),
            ],
            0,
            values,
        )
        .unwrap_err();
        assert_eq!(err, DepositError::NoValue("minecraft:air".into()));
    }

    #[test]
    fn an_empty_deposit_is_refused_rather_than_credited() {
        assert_eq!(plan(&[], 0, values).unwrap_err(), DepositError::Empty);
        // A zero-count stack is not a deposit either.
        assert_eq!(
            plan(&[offered(0, "minecraft:diamond", 0)], 0, values).unwrap_err(),
            DepositError::Empty
        );
    }

    #[test]
    fn a_count_whose_cost_overflows_is_refused() {
        // A stack of 8192-value items is nowhere near the limit: 8192 ×
        // u32::MAX is only ~3.5e13, which fits comfortably in i64.
        assert!(plan(&[offered(0, "minecraft:diamond", u32::MAX)], 0, |_| 8192_u64).is_ok());
        // To overflow a single stack the *value* has to be huge too: 2^32
        // times u32::MAX does not fit in i64.
        let err = plan(&[offered(0, "x:y", u32::MAX)], 0, |_| 1_u64 << 32).unwrap_err();
        assert_eq!(err, DepositError::Overflow);
        // A value that does not even fit with one item is refused as well.
        assert_eq!(
            plan(&[offered(0, "x:y", 1)], 0, |_| u64::MAX).unwrap_err(),
            DepositError::Overflow
        );
    }

    #[test]
    fn a_total_that_overflows_is_refused_even_when_each_stack_fits() {
        // Each stack fits; two of them do not.
        let big = u64::try_from(i64::MAX / 2 + 1).unwrap();
        let err = plan(
            &[offered(0, "x:y", 1), offered(1, "x:y", 1)],
            0,
            |_| big,
        )
        .unwrap_err();
        assert_eq!(err, DepositError::Overflow);
    }

    #[test]
    fn a_balance_that_would_pass_the_limit_is_refused() {
        let err = plan(&[offered(0, "minecraft:diamond", 2)], i64::MAX - 1, values)
            .unwrap_err();
        assert!(matches!(err, DepositError::BalanceOverflow { .. }));
    }

    #[test]
    fn settling_credits_exactly_what_left_the_slots() {
        // A clear that failed: everything is still there.
        let plan = plan(&[offered(0, "minecraft:diamond", 4)], 0, values).unwrap();
        let nothing = settle(&plan, values, |s| s.count, &BTreeSet::new());
        assert_eq!(nothing.credited, 0);
        assert!(nothing.newly_learned.is_empty());
        assert!(nothing.refused);

        // The clear that worked.
        let all = settle(&plan, values, |_| 0, &BTreeSet::new());
        assert_eq!(all.credited, 32_768);
        assert_eq!(all.newly_learned, vec!["minecraft:diamond"]);
        assert!(!all.refused);
    }

    #[test]
    fn a_partial_removal_credits_only_the_part_that_left() {
        let plan = plan(&[offered(0, "minecraft:diamond", 4)], 0, values).unwrap();
        // Two of the four left the slot.
        let part = settle(&plan, values, |_| 2, &BTreeSet::new());
        assert_eq!(part.credited, 2 * 8192);
        // The item was given up, so it is learned.
        assert_eq!(part.newly_learned, vec!["minecraft:diamond"]);
    }

    #[test]
    fn a_repeat_deposit_pays_again_and_learns_nothing_new() {
        let mut known = BTreeSet::new();
        known.insert("minecraft:diamond".to_owned());

        let plan = plan(&[offered(0, "minecraft:diamond", 1)], 0, values).unwrap();
        let first = settle(&plan, values, |_| 0, &known);
        assert_eq!(first.credited, 8192);
        assert!(first.newly_learned.is_empty());

        let second = settle(&plan, values, |_| 0, &known);
        assert_eq!(second.credited, 8192, "a repeat deposit still pays");
        assert!(second.newly_learned.is_empty(), "but teaches nothing twice");
    }

    #[test]
    fn settling_never_credits_more_than_the_plan() {
        let plan = plan(&[offered(0, "minecraft:coal", 3)], 5, values).unwrap();
        for left in 0..=6u32 {
            let s = settle(&plan, values, |_| left, &BTreeSet::new());
            assert!(s.credited <= plan.emc, "left={left} credited {}", s.credited);
        }
    }

    #[test]
    fn a_value_is_unsigned_so_a_deposit_can_never_charge() {
        // `value` returns `u64`, so a negative price is not representable: the
        // worst a bogus table entry can do is pay nothing, which is refused.
        let zero: u64 = 0;
        assert!(matches!(
            plan(&[offered(0, "x:y", 1)], 0, move |_| zero).unwrap_err(),
            DepositError::NoValue(_)
        ));
        // And a value of exactly zero for one item is a refusal, not a free
        // removal.
        assert!(matches!(
            plan(&[offered(0, "x:y", 64)], 0, |_| 0).unwrap_err(),
            DepositError::NoValue(_)
        ));
    }
}
