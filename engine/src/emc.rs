//! Typed `projecte:emc` Data Map and checked EMC arithmetic.
//!
//! Defaults are datapack data in `engine/data/projecte/data_maps/item/emc.json`.
//! Horizon owns pack merging, tag expansion, conditions, replacement and removal.
//! Data Maps are server-side and are not synced to clients; client tooltips never
//! fall back to stale local defaults.

use std::sync::atomic::{AtomicBool, Ordering};

use mod_api::{DataMap, DataMapError, Host, Identifier, ItemKey, Tag, Text};

pub(crate) struct Emc;

impl DataMap for Emc {
    type Entry = ItemKey;
    type Value = u64;

    fn id(&self) -> &str {
        "projecte:emc"
    }

    fn decode(&self, value: &Tag) -> Option<u64> {
        match value {
            Tag::Int(value) => u64::try_from(*value).ok(),
            Tag::Long(value) => u64::try_from(*value).ok(),
            _ => None,
        }
    }
}

pub(crate) const MAP: Emc = Emc;
static MAP_ERROR_REPORTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn value(host: &dyn Host, id: &str) -> Result<Option<u64>, DataMapError> {
    let Ok(identifier) = Identifier::parse(id) else {
        return Ok(None);
    };
    let Some(item) = host.item_key(&identifier) else {
        return Ok(None);
    };
    let result = item.data(host, &MAP);
    if result.is_ok() {
        MAP_ERROR_REPORTED.store(false, Ordering::Relaxed);
    }
    result
}

pub(crate) fn files() -> Vec<(String, Vec<u8>)> {
    vec![(
        "data_maps/projecte:item/emc".to_owned(),
        include_bytes!("../data/projecte/data_maps/item/emc.json").to_vec(),
    )]
}

pub(crate) fn report_map_error(host: &mut dyn Host, error: DataMapError) {
    if !MAP_ERROR_REPORTED.swap(true, Ordering::Relaxed) {
        host.feedback(Text::literal(format!(
            "§cProjectE could not read projecte:emc ({error:?}); EMC-valued machine inputs are being left untouched."
        )));
    }
}

pub fn checked_cost(value: u64, count: u32) -> Option<i64> {
    let total = value.checked_mul(u64::from(count))?;
    i64::try_from(total).ok()
}

pub fn display_name(id: &str) -> String {
    let path = id.rsplit(':').next().unwrap_or(id);
    let mut out = String::with_capacity(path.len());
    let mut upper = true;
    for c in path.chars() {
        if c == '_' {
            out.push(' ');
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
pub(crate) fn default_value(id: &str) -> Option<u64> {
    let data: mod_api::serde_json::Value =
        mod_api::serde_json::from_str(include_str!("../data/projecte/data_maps/item/emc.json"))
            .ok()?;
    data.get("values")?.get(id)?.as_u64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_and_resource_path_are_stable() {
        let ty = MAP.ty();
        assert_eq!(ty.id, "projecte:emc");
        assert_eq!(ty.registry, "minecraft:item");

        let files = files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "data_maps/projecte:item/emc");

        let data: mod_api::serde_json::Value =
            mod_api::serde_json::from_slice(&files[0].1).unwrap();
        assert_eq!(data["values"]["minecraft:diamond"].as_u64(), Some(8192));
        assert_eq!(
            data["values"]["projecte:energy_condenser"].as_u64(),
            Some(33568)
        );
        assert_eq!(data["values"]["#minecraft:logs"].as_u64(), Some(32));
        assert_eq!(data["values"]["#minecraft:wool_carpets"].as_u64(), Some(32));
        assert_eq!(data["values"]["#minecraft:iron_ores"].as_u64(), Some(384));
    }

    #[test]
    fn decoder_accepts_only_non_negative_integer_nbt() {
        assert_eq!(MAP.decode(&Tag::Int(32)), Some(32));
        assert_eq!(MAP.decode(&Tag::Long(3_000_000_000)), Some(3_000_000_000));
        assert_eq!(MAP.decode(&Tag::Int(0)), Some(0));
        assert_eq!(MAP.decode(&Tag::Int(-1)), None);
        assert_eq!(MAP.decode(&Tag::Double(1.5)), None);
        assert_eq!(MAP.decode(&Tag::String("8192".into())), None);
    }

    #[test]
    fn defaults_are_data_not_a_second_runtime_table() {
        assert_eq!(default_value("minecraft:cobblestone"), Some(1));
        assert_eq!(default_value("minecraft:diamond"), Some(8192));
        assert_eq!(default_value("minecraft:iron_ore"), Some(384));
        assert_eq!(default_value("minecraft:crimson_stem"), Some(32));
        assert_eq!(default_value("minecraft:iron_nugget"), Some(28));
        assert_eq!(default_value("projecte:energy_condenser"), Some(33568));
        assert_eq!(default_value("example:iron_ingot"), None);
    }
    #[test]
    fn checked_cost_handles_ordinary_values() {
        assert_eq!(checked_cost(1, 1), Some(1));
        assert_eq!(checked_cost(8192, 64), Some(524_288));
        assert_eq!(checked_cost(32, 16), Some(512));
        assert_eq!(checked_cost(0, 64), Some(0));
    }

    #[test]
    fn checked_cost_rejects_multiplication_and_balance_overflow() {
        assert_eq!(checked_cost(u64::MAX, 2), None);
        assert_eq!(checked_cost(u64::MAX, u32::MAX), None);
        assert_eq!(checked_cost(1u64 << 63, 2), None);
        assert_eq!(checked_cost(i64::MAX as u64 + 1, 1), None);
        assert_eq!(checked_cost(i64::MAX as u64, 2), None);
    }

    #[test]
    fn checked_cost_zero_count_is_always_zero() {
        assert_eq!(checked_cost(8192, 0), Some(0));
        assert_eq!(checked_cost(0, 0), Some(0));
        assert_eq!(checked_cost(u64::MAX, 0), Some(0));
        assert_eq!(checked_cost(i64::MAX as u64, 0), Some(0));
    }
}
