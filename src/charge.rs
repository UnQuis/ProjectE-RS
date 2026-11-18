//! Charge model for ProjectE weapons and relics.
//!
//! The reference keeps two independent values on a stack: `CHARGE`, a free
//! integer level changed one step per CHARGE keybind press (sneak = −1), and
//! `STORED_EMC`, the EMC reservoir used to pay for abilities. Charging never
//! costs EMC; EMC is only consumed when an ability fires, drawing first from
//! the stack's reservoir and then from the player's balance/Klein Stars.
//! Both are typed item components, not Forge Energy and not attachments.

use mod_api::{Component, Tag};

/// Data component name for the integer charge level.
pub(crate) const CHARGE_COMPONENT: &str = "projecte:charge";

/// The fractional EMC carry (ProjectE `UNPROCESSED_EMC`, a double).
pub(crate) const UNPROCESSED_EMC_COMPONENT: &str = "projecte:unprocessed_emc";

pub(crate) struct Charge;

impl Component for Charge {
    type Value = i32;

    fn id(&self) -> &str {
        CHARGE_COMPONENT
    }

    fn decode(&self, value: &Tag) -> Option<Self::Value> {
        match value {
            Tag::Int(value) => Some(*value),
            Tag::Long(value) => i32::try_from(*value).ok(),
            _ => None,
        }
    }

    fn encode(&self, value: Self::Value) -> Tag {
        Tag::Int(value)
    }
}

pub(crate) const CHARGE: Charge = Charge;

pub(crate) struct UnprocessedEmc;

impl Component for UnprocessedEmc {
    type Value = f64;

    fn id(&self) -> &str {
        UNPROCESSED_EMC_COMPONENT
    }

    fn decode(&self, value: &Tag) -> Option<Self::Value> {
        match value {
            Tag::Double(bits) => Some(f64::from_bits(*bits)),
            Tag::Float(bits) => Some(f64::from(f32::from_bits(*bits))),
            _ => None,
        }
    }

    fn encode(&self, value: Self::Value) -> Tag {
        Tag::Double(value.to_bits())
    }
}

pub(crate) const UNPROCESSED_EMC: UnprocessedEmc = UnprocessedEmc;

/// A chargeable item and its reference charge ceiling (`PEItems`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChargeSpec {
    pub id: &'static str,
    pub maximum: i32,
}

pub(crate) const CHARGEABLE: &[ChargeSpec] = &[
    ChargeSpec { id: "projecte:dm_pick", maximum: 2 },
    ChargeSpec { id: "projecte:dm_axe", maximum: 2 },
    ChargeSpec { id: "projecte:dm_shovel", maximum: 2 },
    ChargeSpec { id: "projecte:dm_sword", maximum: 2 },
    ChargeSpec { id: "projecte:dm_hoe", maximum: 2 },
    ChargeSpec { id: "projecte:dm_shears", maximum: 2 },
    ChargeSpec { id: "projecte:dm_hammer", maximum: 2 },
    ChargeSpec { id: "projecte:dark_matter_trident", maximum: 2 },
    ChargeSpec { id: "projecte:rm_pick", maximum: 3 },
    ChargeSpec { id: "projecte:rm_axe", maximum: 3 },
    ChargeSpec { id: "projecte:rm_shovel", maximum: 3 },
    ChargeSpec { id: "projecte:rm_sword", maximum: 3 },
    ChargeSpec { id: "projecte:rm_hoe", maximum: 3 },
    ChargeSpec { id: "projecte:rm_shears", maximum: 3 },
    ChargeSpec { id: "projecte:rm_hammer", maximum: 3 },
    ChargeSpec { id: "projecte:red_matter_trident", maximum: 3 },
    ChargeSpec { id: "projecte:rm_katar", maximum: 4 },
    ChargeSpec { id: "projecte:rm_morning_star", maximum: 4 },
    ChargeSpec { id: "projecte:destruction_catalyst", maximum: 3 },
    ChargeSpec { id: "projecte:hyperkinetic_lens", maximum: 3 },
    ChargeSpec { id: "projecte:mercurial_eye", maximum: 4 },
];

pub(crate) fn maximum_charge(id: &str) -> Option<i32> {
    CHARGEABLE
        .iter()
        .find(|spec| spec.id == id)
        .map(|spec| spec.maximum)
}

/// EMC spent by one activation of an ability, from the MIT reference.
///
/// Sword/katar area attacks are free — charge scales radius and damage, not
/// price. The paid relics are listed with their exact costs.
pub(crate) fn ability_cost(id: &str, charge: i32) -> i64 {
    match id {
        // PESword / RedMatterSword / PEKatar AOE: `emcCost = 0`.
        "projecte:dm_sword" | "projecte:rm_sword" | "projecte:rm_katar" => 0,
        // DestructionCatalyst: 8 EMC per destroyed block (one per activation).
        "projecte:destruction_catalyst" => 8,
        // HyperkineticLens: UNCHARGED / SINGLE / DOUBLE / MAX.
        "projecte:hyperkinetic_lens" => match charge.clamp(0, 3) {
            0 => 384,
            1 => 768,
            2 => 1_536,
            _ => 2_304,
        },
        // VolcaniteAmulet places lava for 32 EMC.
        "projecte:volcanite_amulet" => 32,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChargeError {
    Overflow,
    AlreadyAtBound { requested: i32, bound: i32 },
    NotEnoughEmc { required: i64, available: i64 },
}

/// What a charge-key press would do: `sneak` decrements, otherwise increments.
///
/// `None` means the reference's `changeCharge` returned false — the item was
/// already at the bound, so no sound should play and no component is written.
pub(crate) fn plan_change_charge(current: i32, maximum: i32, sneak: bool) -> Result<i32, ChargeError> {
    let current = current.clamp(0, maximum.max(0));
    if sneak {
        if current == 0 {
            return Err(ChargeError::AlreadyAtBound {
                requested: -1,
                bound: 0,
            });
        }
        Ok(current - 1)
    } else if current >= maximum {
        Err(ChargeError::AlreadyAtBound {
            requested: 1,
            bound: maximum,
        })
    } else {
        current.checked_add(1).ok_or(ChargeError::Overflow)
    }
}

/// EMC drawn for one ability activation: the stack's reservoir first, the
/// remainder from the player's authoritative balance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FuelPlan {
    pub from_stack: i64,
    pub from_player: i64,
    pub new_stack_stored: i64,
}

/// Plan `ItemPE.consumeFuel`: spend `cost` EMC, preferring the stack's own
/// reservoir. Refuse the whole activation if the player cannot cover the gap.
pub(crate) fn plan_consume_fuel(
    stack_stored: i64,
    balance: i64,
    cost: i64,
) -> Result<FuelPlan, ChargeError> {
    if cost <= 0 {
        return Ok(FuelPlan {
            from_stack: 0,
            from_player: 0,
            new_stack_stored: stack_stored.max(0),
        });
    }
    let from_stack = stack_stored.max(0).min(cost);
    let needed = cost - from_stack;
    if needed > balance {
        return Err(ChargeError::NotEnoughEmc {
            required: needed,
            available: balance,
        });
    }
    let new_stack_stored = stack_stored
        .max(0)
        .checked_sub(from_stack)
        .ok_or(ChargeError::Overflow)?;
    Ok(FuelPlan {
        from_stack,
        from_player: needed,
        new_stack_stored,
    })
}

/// EMC sold when a stack is deposited: `floor(value * count)`, with the
/// fractional part kept in `UNPROCESSED_EMC` for the next deposit.
pub(crate) fn split_fractional(value: f64, carried: f64) -> (i64, f64) {
    let total = value.max(0.0) + carried.max(0.0);
    let whole = total.floor();
    (
        if whole >= i64::MAX as f64 {
            i64::MAX
        } else {
            whole as i64
        },
        (total - whole).clamp(0.0, 1.0),
    )
}

/// Result of a fractional EMC drain (rings drain 0.32 EMC per tick).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FractionalPlan {
    pub whole: i64,
    pub next_carried: f64,
    pub new_stack_stored: i64,
    pub from_player: i64,
}

/// Plan a fractional drain, carrying the remainder in `UNPROCESSED_EMC` so a
/// 0.32/tick ring does not round up to one EMC every tick.
pub(crate) fn plan_fractional_consume(
    stack_stored: i64,
    carried: f64,
    balance: i64,
    cost: f64,
) -> Result<FractionalPlan, ChargeError> {
    let (whole, next_carried) = split_fractional(cost, carried);
    let fuel = plan_consume_fuel(stack_stored, balance, whole)?;
    Ok(FractionalPlan {
        whole,
        next_carried,
        new_stack_stored: fuel.new_stack_stored,
        from_player: fuel.from_player,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_is_a_free_bounded_step() {
        assert_eq!(plan_change_charge(0, 2, false), Ok(1));
        assert_eq!(plan_change_charge(1, 2, false), Ok(2));
        assert_eq!(plan_change_charge(2, 2, false), Err(ChargeError::AlreadyAtBound { requested: 1, bound: 2 }));
        assert_eq!(plan_change_charge(2, 2, true), Ok(1));
        assert_eq!(plan_change_charge(0, 2, true), Err(ChargeError::AlreadyAtBound { requested: -1, bound: 0 }));
    }

    #[test]
    fn a_stored_reservoir_pays_before_the_player_balance() {
        let plan = plan_consume_fuel(30, 100, 50).unwrap();
        assert_eq!(plan.from_stack, 30);
        assert_eq!(plan.from_player, 20);
        assert_eq!(plan.new_stack_stored, 0);
        assert_eq!(plan_consume_fuel(100, 0, 50).unwrap().from_player, 0);
    }

    #[test]
    fn an_unaffordable_ability_is_refused_without_partial_payment() {
        assert_eq!(
            plan_consume_fuel(5, 10, 50),
            Err(ChargeError::NotEnoughEmc {
                required: 45,
                available: 10
            })
        );
    }

    #[test]
    fn fractional_emc_is_carried_between_consumptions() {
        assert_eq!(split_fractional(0.4, 0.0), (0, 0.4));
        let (whole, rest) = split_fractional(0.7, 0.4);
        assert_eq!(whole, 1);
        assert!((rest - 0.1).abs() < 1e-9);
    }

    #[test]
    fn ability_costs_match_the_reference() {
        assert_eq!(ability_cost("projecte:dm_sword", 2), 0);
        assert_eq!(ability_cost("projecte:rm_katar", 4), 0);
        assert_eq!(ability_cost("projecte:destruction_catalyst", 3), 8);
        assert_eq!(ability_cost("projecte:hyperkinetic_lens", 0), 384);
        assert_eq!(ability_cost("projecte:hyperkinetic_lens", 3), 2_304);
        assert_eq!(ability_cost("projecte:volcanite_amulet", 0), 32);
    }

    #[test]
    fn charge_ceilings_match_the_reference_items() {
        assert_eq!(maximum_charge("projecte:dm_sword"), Some(2));
        assert_eq!(maximum_charge("projecte:rm_sword"), Some(3));
        assert_eq!(maximum_charge("projecte:rm_katar"), Some(4));
        assert_eq!(maximum_charge("projecte:rm_morning_star"), Some(4));
        assert_eq!(maximum_charge("projecte:minecraft_stone"), None);
    }

    #[test]
    fn fractional_drains_carry_the_remainder() {
        let first = plan_fractional_consume(10, 0.0, 100, 0.32).unwrap();
        assert_eq!(first.whole, 0);
        assert!((first.next_carried - 0.32).abs() < 1e-9);
        let second = plan_fractional_consume(10, first.next_carried, 100, 0.32).unwrap();
        assert_eq!(second.whole, 0);
        // 0.68 carried + 0.32 cost = exactly one whole EMC.
        let fourth = plan_fractional_consume(10, 0.68, 100, 0.32).unwrap();
        assert_eq!(fourth.whole, 1);
        assert!(fourth.next_carried < 1e-9);
    }

    #[test]
    fn charge_components_round_trip() {
        let charge = Charge;
        assert_eq!(charge.decode(&charge.encode(7)), Some(7));
        assert_eq!(charge.decode(&Tag::String("x".into())), None);
        let unprocessed = UnprocessedEmc;
        assert_eq!(unprocessed.decode(&unprocessed.encode(0.25)), Some(0.25));
    }
}
