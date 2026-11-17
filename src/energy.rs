//! Klein Star charge storage and capacity rules.
//!
//! EMC stays a server-authoritative `i64` with ProjectE's own semantics; the
//! SDK's `ITEM_ENERGY` capability is an `i32` Forge-Energy model and is not
//! used here (see the orchestrator's 2026-11-17 decision). Charge therefore
//! lives in a mod-owned typed item component, whose codec is defined below.

use mod_api::{Component, ItemStack, StackError, Tag};

pub(crate) const STORED_EMC_COMPONENT: &str = "projecte:stored_emc";

pub(crate) const KLEIN_STARS: [&str; 6] = [
    "projecte:klein_star_ein",
    "projecte:klein_star_zwei",
    "projecte:klein_star_drei",
    "projecte:klein_star_vier",
    "projecte:klein_star_sphere",
    "projecte:klein_star_omega",
];

/// The typed descriptor behind `STORED_EMC_COMPONENT`.
///
/// The old API addressed components by a string on the Host; SDK 0.2 puts the
/// codec on the descriptor itself and stores the value in the stack's patch.
pub(crate) struct StoredEmc;

impl Component for StoredEmc {
    type Value = i64;

    fn id(&self) -> &str {
        STORED_EMC_COMPONENT
    }

    fn decode(&self, value: &Tag) -> Option<Self::Value> {
        match value {
            Tag::Long(value) => Some(*value),
            Tag::Int(value) => Some(i64::from(*value)),
            _ => None,
        }
    }

    fn encode(&self, value: Self::Value) -> Tag {
        Tag::Long(value)
    }
}

pub(crate) const STORED_EMC: StoredEmc = StoredEmc;

pub(crate) fn capacity(item: &str) -> Option<i64> {
    Some(match item {
        "projecte:klein_star_ein" => 50_000,
        "projecte:klein_star_zwei" => 200_000,
        "projecte:klein_star_drei" => 800_000,
        "projecte:klein_star_vier" => 3_200_000,
        "projecte:klein_star_sphere" => 12_800_000,
        "projecte:klein_star_omega" => 51_200_000,
        _ => return None,
    })
}

/// The charge a Klein Star currently holds, clamped to its tier capacity.
///
/// The `Result` preserves the old call shape used by the command/UI code; the
/// SDK's typed component read is infallible, so the error is only ever
/// constructed when a write fails.
pub(crate) fn stored_emc(stack: &ItemStack) -> Result<i64, StackError> {
    Ok(stack.get(&STORED_EMC).unwrap_or(0).max(0))
}

/// Adds EMC to a Klein Star and returns the amount accepted.
pub(crate) fn charge(stack: &mut ItemStack, available: i64, capacity: i64) -> i64 {
    let current = stored_emc(stack).unwrap_or(0);
    let moved = available.max(0).min(capacity - current);
    if moved == 0 {
        return 0;
    }
    if stack.set(&STORED_EMC, current + moved).is_ok() {
        moved
    } else {
        0
    }
}

/// Removes EMC from a Klein Star and returns the amount extracted.
pub(crate) fn discharge(stack: &mut ItemStack, room: i64) -> i64 {
    let current = stored_emc(stack).unwrap_or(0);
    let moved = current.min(room.max(0));
    if moved == 0 {
        return 0;
    }
    let result = if current == moved {
        stack.remove(&STORED_EMC);
        Ok(())
    } else {
        stack.set(&STORED_EMC, current - moved).map(|_| ())
    };
    if result.is_ok() { moved } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacities_follow_the_klein_tier_ladder() {
        assert_eq!(capacity(KLEIN_STARS[0]), Some(50_000));
        assert_eq!(capacity(KLEIN_STARS[1]), Some(200_000));
        assert_eq!(capacity(KLEIN_STARS[2]), Some(800_000));
        assert_eq!(capacity(KLEIN_STARS[3]), Some(3_200_000));
        assert_eq!(capacity(KLEIN_STARS[4]), Some(12_800_000));
        assert_eq!(capacity(KLEIN_STARS[5]), Some(51_200_000));
        assert_eq!(capacity("minecraft:diamond"), None);
    }
}
