//! Pure planning and settlement for EMC transmutation transactions.
//!
//! This module owns the invariants shared by the table and command paths:
//! validate the full request before granting anything, then charge only for
//! the amount the inventory actually accepted.

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
    balance: i64,
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
    if balance < maximum_cost {
        return Err(PurchaseError::InsufficientEmc {
            required: maximum_cost,
            available: balance,
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
    balance: i64,
) -> Result<PurchaseSettlement, PurchaseError> {
    if granted == 0 {
        return Err(PurchaseError::NothingGranted);
    }
    if granted > plan.requested {
        return Err(PurchaseError::InvalidGrant);
    }

    let spent = crate::emc::checked_cost(plan.value, granted).ok_or(PurchaseError::CostOverflow)?;
    if spent > plan.maximum_cost || spent > balance {
        return Err(PurchaseError::InvalidGrant);
    }

    Ok(PurchaseSettlement {
        granted,
        spent,
        remaining_emc: balance - spent,
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
            plan_purchase(u64::MAX, 2, i64::MAX, u32::MAX),
            Err(PurchaseError::CostOverflow)
        );
        assert_eq!(
            plan_purchase(i64::MAX as u64 + 1, 1, i64::MAX, 1),
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
        let plan = plan_purchase(i64::MAX as u64, 1, i64::MAX, 1).unwrap();
        assert_eq!(
            settle_purchase(plan, 1, i64::MAX),
            Ok(PurchaseSettlement {
                granted: 1,
                spent: i64::MAX,
                remaining_emc: 0,
            })
        );
    }
}
