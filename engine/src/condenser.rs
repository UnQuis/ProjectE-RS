//! Pure EMC accounting for Energy Condensers.
//!
//! This module deliberately contains no Horizon hooks or inventory mutations.
//! It plans bounded operations and lets the integration layer settle them
//! against the amount the live inventory actually accepted or produced.
//! The caller must preserve the complete target ItemStack (including its
//! components) when creating output; this module only accounts for its EMC.

/// Persistent EMC balance held by a Condenser.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CondenserState {
    pub stored_emc: u64,
}

/// Per-tick limits supplied by the machine tier.
///
/// Balance constants are intentionally not selected here: the public gameplay
/// references vary by ProjectE version, so tier values belong in a later,
/// explicitly reviewed machine specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CondenserSpec {
    pub capacity: u64,
    pub max_input_items_per_tick: u32,
    pub max_output_items_per_tick: u32,
}

/// A rejected accounting operation. No state is mutated by any planner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CondenserError {
    ZeroCapacity,
    BufferExceedsCapacity,
    ZeroEmcValue,
    ArithmeticOverflow,
    SettlementExceedsPlan,
}

impl CondenserError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::ZeroCapacity => "Condenser capacity must be greater than zero.",
            Self::BufferExceedsCapacity => "Condenser EMC buffer exceeds its capacity.",
            Self::ZeroEmcValue => "The item has no positive EMC value.",
            Self::ArithmeticOverflow => "The EMC operation exceeds the supported numeric range.",
            Self::SettlementExceedsPlan => "The inventory settled more items than the plan allowed.",
        }
    }
}

/// A plan to convert input items into EMC.
///
/// `consumed_items` is the maximum amount the integration layer may remove.
/// If the live inventory accepts/removes fewer items, call
/// `settle_input_conversion` with the actual count before committing the
/// resulting buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputConversionPlan {
    pub consumed_items: u32,
    pub credited_emc: u64,
    pub next_state: CondenserState,
    initial_state: CondenserState,
    unit_emc: u64,
}

/// A plan to manufacture target items from the EMC buffer.
///
/// `produced_items` is the maximum amount the integration layer may insert.
/// Debit only the amount confirmed as inserted by the live output inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputProductionPlan {
    pub produced_items: u32,
    pub charged_emc: u64,
    pub next_state: CondenserState,
    initial_state: CondenserState,
    target_unit_emc: u64,
}

/// Validate a state against its tier capacity.
pub fn validate_state(
    state: CondenserState,
    spec: CondenserSpec,
) -> Result<(), CondenserError> {
    if spec.capacity == 0 {
        return Err(CondenserError::ZeroCapacity);
    }
    if state.stored_emc > spec.capacity {
        return Err(CondenserError::BufferExceedsCapacity);
    }
    Ok(())
}

/// Plan bounded conversion of positive-EMC input items into the EMC buffer.
///
/// Unknown/zero-value items are rejected, not consumed. The returned count is
/// bounded by available input, the tier's per-tick limit, and free buffer room.
pub fn plan_input_conversion(
    state: CondenserState,
    spec: CondenserSpec,
    unit_emc: u64,
    available_items: u32,
) -> Result<InputConversionPlan, CondenserError> {
    validate_state(state, spec)?;
    if unit_emc == 0 {
        return Err(CondenserError::ZeroEmcValue);
    }

    let room = spec.capacity - state.stored_emc;
    let by_capacity = room / unit_emc;
    let count = available_items
        .min(spec.max_input_items_per_tick)
        .min(by_capacity.min(u64::from(u32::MAX)) as u32);
    let credited_emc = unit_emc
        .checked_mul(u64::from(count))
        .ok_or(CondenserError::ArithmeticOverflow)?;
    let next_balance = state
        .stored_emc
        .checked_add(credited_emc)
        .ok_or(CondenserError::ArithmeticOverflow)?;

    Ok(InputConversionPlan {
        consumed_items: count,
        credited_emc,
        next_state: CondenserState {
            stored_emc: next_balance,
        },
        initial_state: state,
        unit_emc,
    })
}

/// Settle an input plan against the number of items actually removed.
///
/// A smaller count is valid (for example, if a slot changed between snapshot
/// and mutation); a larger count is rejected. The caller must commit the
/// returned state only after the matching item removal succeeds.
pub fn settle_input_conversion(
    plan: InputConversionPlan,
    actually_consumed: u32,
) -> Result<InputConversionPlan, CondenserError> {
    if actually_consumed > plan.consumed_items {
        return Err(CondenserError::SettlementExceedsPlan);
    }
    let credited_emc = plan
        .unit_emc
        .checked_mul(u64::from(actually_consumed))
        .ok_or(CondenserError::ArithmeticOverflow)?;
    let stored_emc = plan
        .initial_state
        .stored_emc
        .checked_add(credited_emc)
        .ok_or(CondenserError::ArithmeticOverflow)?;

    Ok(InputConversionPlan {
        consumed_items: actually_consumed,
        credited_emc,
        next_state: CondenserState { stored_emc },
        ..plan
    })
}

/// Plan bounded production of a target item from stored EMC.
///
/// Output capacity is supplied as a count of items the output inventory can
/// actually accept. The result is bounded by that room, the tier's throughput,
/// and the available EMC.
pub fn plan_output_production(
    state: CondenserState,
    spec: CondenserSpec,
    target_unit_emc: u64,
    output_room: u32,
) -> Result<OutputProductionPlan, CondenserError> {
    validate_state(state, spec)?;
    if target_unit_emc == 0 {
        return Err(CondenserError::ZeroEmcValue);
    }

    let by_emc = state.stored_emc / target_unit_emc;
    let count = output_room
        .min(spec.max_output_items_per_tick)
        .min(by_emc.min(u64::from(u32::MAX)) as u32);
    let charged_emc = target_unit_emc
        .checked_mul(u64::from(count))
        .ok_or(CondenserError::ArithmeticOverflow)?;
    let next_balance = state
        .stored_emc
        .checked_sub(charged_emc)
        .ok_or(CondenserError::ArithmeticOverflow)?;

    Ok(OutputProductionPlan {
        produced_items: count,
        charged_emc,
        next_state: CondenserState {
            stored_emc: next_balance,
        },
        initial_state: state,
        target_unit_emc,
    })
}

/// Settle output production against the count actually inserted.
///
/// The buffer is debited only for successfully inserted items. A failed or
/// partial insertion therefore cannot spend EMC for items that were not made.
pub fn settle_output_production(
    plan: OutputProductionPlan,
    actually_inserted: u32,
) -> Result<OutputProductionPlan, CondenserError> {
    if actually_inserted > plan.produced_items {
        return Err(CondenserError::SettlementExceedsPlan);
    }
    let charged_emc = plan
        .target_unit_emc
        .checked_mul(u64::from(actually_inserted))
        .ok_or(CondenserError::ArithmeticOverflow)?;
    let stored_emc = plan
        .initial_state
        .stored_emc
        .checked_sub(charged_emc)
        .ok_or(CondenserError::ArithmeticOverflow)?;

    Ok(OutputProductionPlan {
        produced_items: actually_inserted,
        charged_emc,
        next_state: CondenserState { stored_emc },
        ..plan
    })
}

/// Plan an EMC transfer into a Condenser, accepting no more than its free room.
pub fn plan_receive_emc(
    state: CondenserState,
    spec: CondenserSpec,
    offered_emc: u64,
) -> Result<(u64, CondenserState), CondenserError> {
    validate_state(state, spec)?;
    let accepted = offered_emc.min(spec.capacity - state.stored_emc);
    let next_balance = state
        .stored_emc
        .checked_add(accepted)
        .ok_or(CondenserError::ArithmeticOverflow)?;
    Ok((accepted, CondenserState { stored_emc: next_balance }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MK1: CondenserSpec = CondenserSpec {
        capacity: 10_000,
        max_input_items_per_tick: 1,
        max_output_items_per_tick: 1,
    };

    const MK2: CondenserSpec = CondenserSpec {
        capacity: 100_000,
        max_input_items_per_tick: 64,
        max_output_items_per_tick: 64,
    };

    #[test]
    fn input_respects_mk1_throughput() {
        let plan = plan_input_conversion(CondenserState::default(), MK1, 32, 64).unwrap();
        assert_eq!(plan.consumed_items, 1);
        assert_eq!(plan.credited_emc, 32);
        assert_eq!(plan.next_state.stored_emc, 32);
    }

    #[test]
    fn input_respects_mk2_batch_limit() {
        let plan = plan_input_conversion(CondenserState::default(), MK2, 32, 100).unwrap();
        assert_eq!(plan.consumed_items, 64);
        assert_eq!(plan.credited_emc, 2048);
    }

    #[test]
    fn input_is_limited_by_remaining_capacity() {
        let state = CondenserState { stored_emc: 9_950 };
        let plan = plan_input_conversion(state, MK1, 32, 10).unwrap();
        assert_eq!(plan.consumed_items, 1);
        assert_eq!(plan.credited_emc, 32);
        assert_eq!(plan.next_state.stored_emc, 9_982);
    }

    #[test]
    fn input_does_not_consume_when_one_item_does_not_fit() {
        let state = CondenserState { stored_emc: 9_990 };
        let plan = plan_input_conversion(state, MK1, 32, 10).unwrap();
        assert_eq!(plan.consumed_items, 0);
        assert_eq!(plan.credited_emc, 0);
        assert_eq!(plan.next_state, state);
    }

    #[test]
    fn input_settlement_credits_only_items_actually_consumed() {
        let plan = plan_input_conversion(CondenserState::default(), MK2, 32, 10).unwrap();
        let settled = settle_input_conversion(plan, 3).unwrap();
        assert_eq!(settled.consumed_items, 3);
        assert_eq!(settled.credited_emc, 96);
        assert_eq!(settled.next_state.stored_emc, 96);
    }

    #[test]
    fn input_settlement_rejects_more_than_planned() {
        let plan = plan_input_conversion(CondenserState::default(), MK1, 32, 10).unwrap();
        assert_eq!(
            settle_input_conversion(plan, 2),
            Err(CondenserError::SettlementExceedsPlan)
        );
    }

    #[test]
    fn zero_value_input_is_rejected() {
        assert_eq!(
            plan_input_conversion(CondenserState::default(), MK1, 0, 1),
            Err(CondenserError::ZeroEmcValue)
        );
    }

    #[test]
    fn output_respects_throughput_and_inventory_room() {
        let state = CondenserState { stored_emc: 8192 * 10 };
        let spec = CondenserSpec { capacity: 100_000, ..MK1 };
        let plan = plan_output_production(state, spec, 8192, 64).unwrap();
        assert_eq!(plan.produced_items, 1);
        assert_eq!(plan.charged_emc, 8192);
        assert_eq!(plan.next_state.stored_emc, 8192 * 9);
    }

    #[test]
    fn output_settlement_charges_only_items_actually_inserted() {
        let state = CondenserState { stored_emc: 8192 * 10 };
        let plan = plan_output_production(state, MK2, 8192, 10).unwrap();
        let settled = settle_output_production(plan, 4).unwrap();
        assert_eq!(settled.produced_items, 4);
        assert_eq!(settled.charged_emc, 8192 * 4);
        assert_eq!(settled.next_state.stored_emc, 8192 * 6);
    }

    #[test]
    fn output_does_not_charge_when_inventory_is_full() {
        let state = CondenserState { stored_emc: 50_000 };
        let plan = plan_output_production(state, MK2, 8192, 0).unwrap();
        assert_eq!(plan.produced_items, 0);
        assert_eq!(plan.charged_emc, 0);
        assert_eq!(plan.next_state, state);
    }

    #[test]
    fn zero_value_target_is_rejected() {
        assert_eq!(
            plan_output_production(CondenserState { stored_emc: 100 }, MK1, 0, 1),
            Err(CondenserError::ZeroEmcValue)
        );
    }

    #[test]
    fn receive_emc_is_limited_to_free_capacity() {
        let state = CondenserState { stored_emc: 9_950 };
        let (accepted, next) = plan_receive_emc(state, MK1, 100).unwrap();
        assert_eq!(accepted, 50);
        assert_eq!(next.stored_emc, 10_000);
    }

    #[test]
    fn invalid_capacity_and_corrupt_state_are_rejected() {
        let zero_capacity = CondenserSpec { capacity: 0, ..MK1 };
        assert_eq!(
            validate_state(CondenserState::default(), zero_capacity),
            Err(CondenserError::ZeroCapacity)
        );
        assert_eq!(
            validate_state(CondenserState { stored_emc: 10_001 }, MK1),
            Err(CondenserError::BufferExceedsCapacity)
        );
    }

    #[test]
    fn large_emc_arithmetic_is_checked() {
        let spec = CondenserSpec {
            capacity: u64::MAX,
            max_input_items_per_tick: u32::MAX,
            max_output_items_per_tick: u32::MAX,
        };
        let state = CondenserState::default();
        let plan = plan_input_conversion(state, spec, u64::MAX, u32::MAX).unwrap();
        assert_eq!(plan.consumed_items, 1);
        assert_eq!(plan.credited_emc, u64::MAX);
        assert_eq!(plan.next_state.stored_emc, u64::MAX);
    }
}
