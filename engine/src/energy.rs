//! Klein Star charge storage and capacity rules.
//!
//! The charge is kept in a ProjectE-owned data component until the pinned
//! trait-mod API exposes the native item-energy accessors. See API_WORKAROUNDS.md.

use horizon_client::items::ItemStack;
use horizon_client::mod_api::{Host, NativeComponentError, Tag};

pub(crate) const STORED_EMC_COMPONENT: &str = "projecte:stored_emc";

pub(crate) const KLEIN_STARS: [&str; 6] = [
    "projecte:klein_star_ein",
    "projecte:klein_star_zwei",
    "projecte:klein_star_drei",
    "projecte:klein_star_vier",
    "projecte:klein_star_sphere",
    "projecte:klein_star_omega",
];

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

pub(crate) fn stored_emc(
    host: &dyn Host,
    stack: &ItemStack,
) -> Result<i64, NativeComponentError> {
    let Some(capacity) = capacity(&stack.item) else {
        return Ok(0);
    };
    Ok(match host.component(stack, STORED_EMC_COMPONENT)? {
        Some(Tag::Long(value)) => (*value).clamp(0, capacity),
        Some(Tag::Int(value)) => i64::from(*value).clamp(0, capacity),
        _ => 0,
    })
}

/// Adds EMC to a Klein Star and returns the amount accepted.
pub(crate) fn charge(host: &dyn Host, stack: &mut ItemStack, available: i64) -> i64 {
    let Some(capacity) = capacity(&stack.item) else {
        return 0;
    };
    let Ok(current) = stored_emc(host, stack) else { return 0 };
    let moved = available.max(0).min(capacity - current);
    if moved == 0 {
        return 0;
    }
    if host
        .set_component(stack, STORED_EMC_COMPONENT, Tag::Long(current + moved))
        .is_ok()
    {
        moved
    } else {
        0
    }
}

/// Removes EMC from a Klein Star and returns the amount extracted.
pub(crate) fn discharge(host: &dyn Host, stack: &mut ItemStack, room: i64) -> i64 {
    let Ok(current) = stored_emc(host, stack) else { return 0 };
    let moved = current.min(room.max(0));
    if moved == 0 {
        return 0;
    }
    let result = if current == moved {
        host.remove_component(stack, STORED_EMC_COMPONENT)
    } else {
        host.set_component(stack, STORED_EMC_COMPONENT, Tag::Long(current - moved))
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
