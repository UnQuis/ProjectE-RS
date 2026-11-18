//! Pure planning and settlement for EMC transmutation transactions.
//!
//! This module owns the invariants shared by the table and command paths:
//! validate the full request before granting anything, then charge only for
//! the amount the inventory actually accepted.

/// ProjectE's covalence loss, stored as a fixed-point numerator over
/// [`COVALENCE_SCALE`]. Keeping it integral makes sell prices exact and
/// repeatable — no floating-point drift in a server-authoritative calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Covalence {
    numerator: u64,
    minimum_one: bool,
}

const COVALENCE_SCALE: u128 = 1_000_000;

impl Covalence {
    /// ProjectE's default: sell for full value, with a floor of one EMC.
    pub const DEFAULT: Self = Self {
        numerator: COVALENCE_SCALE as u64,
        minimum_one: true,
    };

    /// Validate the configured loss (0.1 ..= 1.0) before it reaches any price.
    /// This is the only way to build a non-default covalence; invalid values
    /// are rejected rather than clamped.
    pub fn new(loss: f64, minimum_one: bool) -> Option<Self> {
        if !loss.is_finite() || !(0.1..=1.0).contains(&loss) {
            return None;
        }
        let scaled = (loss * COVALENCE_SCALE as f64).round();
        // The range check bounds this well below u64::MAX; the explicit guard
        // keeps a future scale change from becoming a lossy cast.
        if !scaled.is_finite() || scaled <= 0.0 || scaled > COVALENCE_SCALE as f64 {
            return None;
        }
        Some(Self {
            numerator: scaled as u64,
            minimum_one,
        })
    }

    pub fn loss(self) -> f64 {
        self.numerator as f64 / COVALENCE_SCALE as f64
    }

    /// The EMC credited when an item is consumed (sold) rather than bought.
    /// Returns `None` only when the scaled value cannot fit the balance type.
    pub fn sell_value(self, value: u64) -> Option<u64> {
        let scaled = (value as u128)
            .checked_mul(self.numerator as u128)?
            .checked_div(COVALENCE_SCALE)?;
        let floored = u64::try_from(scaled).ok()?;
        Some(if floored == 0 && self.minimum_one && value > 0 {
            1
        } else {
            floored
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PurchaseError {
    ZeroValue,
    ZeroCount,
    CostOverflow,
    InsufficientEmc { required: i64, available: i64 },
    InsufficientInventory { required: u32, available: u32 },
    NothingGranted,
    InvalidGrant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PurchasePlan {
    value: u64,
    requested: u32,
    maximum_cost: i64,
}

impl PurchasePlan {
    pub(crate) const fn maximum_cost(self) -> i64 {
        self.maximum_cost
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PurchaseSettlement {
    pub granted: u32,
    pub spent: i64,
    pub remaining_emc: i64,
}

/// Validate a complete purchase before any item or balance mutation.
///
/// The inventory room is checked against the full request. A caller must not
/// call the inventory grant operation unless this succeeds.
pub(crate) fn plan_purchase(
    value: u64,
    requested: u32,
    balance: u64,
    inventory_room: u32,
) -> Result<PurchasePlan, PurchaseError> {
    if value == 0 {
        return Err(PurchaseError::ZeroValue);
    }
    if requested == 0 {
        return Err(PurchaseError::ZeroCount);
    }

    let maximum_cost =
        crate::emc::checked_cost(value, requested).ok_or(PurchaseError::CostOverflow)?;
    // Balances above i64::MAX are far larger than any representable cost, so
    // clamping this comparison bound cannot make an unaffordable purchase pass.
    let comparison_balance = i64::try_from(balance).unwrap_or(i64::MAX);
    if comparison_balance < maximum_cost {
        return Err(PurchaseError::InsufficientEmc {
            required: maximum_cost,
            available: comparison_balance,
        });
    }
    if inventory_room < requested {
        return Err(PurchaseError::InsufficientInventory {
            required: requested,
            available: inventory_room,
        });
    }

    Ok(PurchasePlan {
        value,
        requested,
        maximum_cost,
    })
}

/// Settle a validated purchase after the inventory reports how many items it
/// accepted. Partial grants are charged proportionally; zero or impossible
/// grant counts are rejected.
pub(crate) fn settle_purchase(
    plan: PurchasePlan,
    granted: u32,
    balance: u64,
) -> Result<PurchaseSettlement, PurchaseError> {
    if granted == 0 {
        return Err(PurchaseError::NothingGranted);
    }
    if granted > plan.requested {
        return Err(PurchaseError::InvalidGrant);
    }

    let spent = crate::emc::checked_cost(plan.value, granted).ok_or(PurchaseError::CostOverflow)?;
    if spent > plan.maximum_cost || spent < 0 || (spent as u64) > balance {
        return Err(PurchaseError::InvalidGrant);
    }

    let comparison_balance = i64::try_from(balance).unwrap_or(i64::MAX);
    Ok(PurchaseSettlement {
        granted,
        spent,
        remaining_emc: comparison_balance - spent,
    })
}

impl PurchaseError {
    pub(crate) fn message(self) -> String {
        match self {
            Self::ZeroValue => "This item has no EMC value.".to_owned(),
            Self::ZeroCount => "Cannot transmute zero items.".to_owned(),
            Self::CostOverflow => "That transaction is too expensive.".to_owned(),
            Self::InsufficientEmc {
                required,
                available,
            } => {
                format!("Not enough EMC: need {required}, have {available}.")
            }
            Self::InsufficientInventory {
                required,
                available,
            } => {
                format!("Not enough inventory space: need {required}, have room for {available}.")
            }
            Self::NothingGranted => "Could not give the item.".to_owned(),
            Self::InvalidGrant => "The inventory returned an invalid item count.".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_a_valid_purchase() {
        let plan = plan_purchase(32, 4, 256, 4).unwrap();
        assert_eq!(
            plan,
            PurchasePlan {
                value: 32,
                requested: 4,
                maximum_cost: 128,
            }
        );
    }

    #[test]
    fn rejects_zero_value_and_zero_count() {
        assert_eq!(plan_purchase(0, 1, 100, 64), Err(PurchaseError::ZeroValue));
        assert_eq!(plan_purchase(1, 0, 100, 64), Err(PurchaseError::ZeroCount));
    }

    #[test]
    fn rejects_costs_that_do_not_fit_the_balance_type() {
        assert_eq!(
            plan_purchase(u64::MAX, 2, i64::MAX as u64, u32::MAX),
            Err(PurchaseError::CostOverflow)
        );
        assert_eq!(
            plan_purchase(i64::MAX as u64 + 1, 1, i64::MAX as u64, 1),
            Err(PurchaseError::CostOverflow)
        );
    }

    #[test]
    fn rejects_insufficient_emc_before_granting() {
        assert_eq!(
            plan_purchase(16, 4, 63, 64),
            Err(PurchaseError::InsufficientEmc {
                required: 64,
                available: 63,
            })
        );
    }

    #[test]
    fn rejects_insufficient_inventory_before_granting() {
        assert_eq!(
            plan_purchase(16, 4, 64, 3),
            Err(PurchaseError::InsufficientInventory {
                required: 4,
                available: 3,
            })
        );
    }

    #[test]
    fn charges_only_for_a_partial_grant() {
        let plan = plan_purchase(32, 8, 512, 8).unwrap();
        assert_eq!(
            settle_purchase(plan, 3, 512),
            Ok(PurchaseSettlement {
                granted: 3,
                spent: 96,
                remaining_emc: 416,
            })
        );
    }

    #[test]
    fn rejects_zero_or_impossible_grants() {
        let plan = plan_purchase(32, 8, 512, 8).unwrap();
        assert_eq!(
            settle_purchase(plan, 0, 512),
            Err(PurchaseError::NothingGranted)
        );
        assert_eq!(
            settle_purchase(plan, 9, 512),
            Err(PurchaseError::InvalidGrant)
        );
    }

    #[test]
    fn supports_the_largest_representable_cost() {
        let plan = plan_purchase(i64::MAX as u64, 1, i64::MAX as u64, 1).unwrap();
        assert_eq!(
            settle_purchase(plan, 1, i64::MAX as u64),
            Ok(PurchaseSettlement {
                granted: 1,
                spent: i64::MAX,
                remaining_emc: 0,
            })
        );
    }

    #[test]
    fn refuses_to_settle_when_the_spent_cost_exceeds_the_reported_balance() {
        // The caller may have lost EMC between the plan and the grant. Charging
        // anyway would either underflow the balance or mint value.
        let plan = plan_purchase(32, 4, 128, 4).unwrap();
        assert_eq!(
            settle_purchase(plan, 4, 64),
            Err(PurchaseError::InvalidGrant)
        );
    }

    #[test]
    fn default_covalence_sells_for_full_value() {
        assert_eq!(Covalence::DEFAULT.sell_value(8192), Some(8192));
        assert_eq!(Covalence::DEFAULT.sell_value(1), Some(1));
        assert_eq!(Covalence::DEFAULT.sell_value(0), Some(0));
        assert_eq!(Covalence::DEFAULT.loss(), 1.0);
    }

    #[test]
    fn covalence_loss_truncates_and_floors_to_one() {
        let half = Covalence::new(0.5, true).unwrap();
        assert_eq!(half.sell_value(8192), Some(4096));
        assert_eq!(half.sell_value(5), Some(2));
        assert_eq!(half.sell_value(1), Some(1));
        assert_eq!(half.sell_value(0), Some(0));

        let no_floor = Covalence::new(0.5, false).unwrap();
        assert_eq!(no_floor.sell_value(5), Some(2));
        assert_eq!(no_floor.sell_value(1), Some(0));
        assert_eq!(no_floor.sell_value(0), Some(0));

        let tenth = Covalence::new(0.1, true).unwrap();
        assert_eq!(tenth.sell_value(10), Some(1));
        assert_eq!(tenth.sell_value(9), Some(1));
        assert_eq!(Covalence::new(0.1, false).unwrap().sell_value(9), Some(0));
    }

    #[test]
    fn covalence_rejects_out_of_range_losses() {
        for loss in [0.0, -0.5, 1.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(Covalence::new(loss, true), None, "accepted {loss:?}");
        }
        assert!(Covalence::new(0.1, true).is_some());
        assert!(Covalence::new(1.0, true).is_some());
    }

    #[test]
    fn covalence_handles_the_largest_values_without_wrapping() {
        assert_eq!(Covalence::DEFAULT.sell_value(u64::MAX), Some(u64::MAX));
        let half = Covalence::new(0.5, true).unwrap();
        assert_eq!(half.sell_value(u64::MAX), Some(u64::MAX / 2));
    }

    #[test]
    fn a_partial_grant_is_always_cheaper_than_the_reservation() {
        let plan = plan_purchase(8192, 64, 600_000, 64).unwrap();
        let settlement = settle_purchase(plan, 63, 600_000).unwrap();
        assert_eq!(settlement.spent, 8192 * 63);
        assert!(settlement.spent < plan.maximum_cost());
        assert_eq!(settlement.remaining_emc, 600_000 - settlement.spent);
    }
}
