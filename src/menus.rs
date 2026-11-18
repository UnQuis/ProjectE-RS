//! Mod menu declarations for ProjectE-RS machines.
//!
//! Each file under `data/projecte/horizon/menu/` declares one Horizon
//! `MenuType` (`<id>:<name>`) with its slot count, synced data slots and the
//! slots players may only take from. The files are contributed through
//! `data_pack_files` so data packs can override them; [`validate`] checks the
//! shipped defaults before they are used.

use std::collections::BTreeSet;

use mod_api::serde_json::{self, Value};

const ALCHEMICAL_CHEST: &str = include_str!("../data/projecte/horizon/menu/alchemical_chest.json");
const COLLECTOR_MK1: &str = include_str!("../data/projecte/horizon/menu/collector_mk1.json");
const COLLECTOR_MK2: &str = include_str!("../data/projecte/horizon/menu/collector_mk2.json");
const COLLECTOR_MK3: &str = include_str!("../data/projecte/horizon/menu/collector_mk3.json");
const CONDENSER_MK2: &str = include_str!("../data/projecte/horizon/menu/condenser_mk2.json");
const CONDENSER_MK3: &str = include_str!("../data/projecte/horizon/menu/condenser_mk3.json");
const CONDENSER_MK3_INPUT: &str =
    include_str!("../data/projecte/horizon/menu/condenser_mk3_input.json");
const CONDENSER_MK3_OUTPUT: &str =
    include_str!("../data/projecte/horizon/menu/condenser_mk3_output.json");
const ENERGY_CONDENSER: &str = include_str!("../data/projecte/horizon/menu/condenser_mk1.json");
const RELAY_MK1: &str = include_str!("../data/projecte/horizon/menu/relay_mk1.json");
const RELAY_MK2: &str = include_str!("../data/projecte/horizon/menu/relay_mk2.json");
const RELAY_MK3: &str = include_str!("../data/projecte/horizon/menu/relay_mk3.json");
const TRANSMUTATION_TABLE: &str =
    include_str!("../data/projecte/horizon/menu/transmutation_table.json");

/// (menu name, embedded JSON) in a fixed order for deterministic output.
const FILES: &[(&str, &str)] = &[
    ("alchemical_chest", ALCHEMICAL_CHEST),
    ("collector_mk1", COLLECTOR_MK1),
    ("collector_mk2", COLLECTOR_MK2),
    ("collector_mk3", COLLECTOR_MK3),
    ("condenser_mk2", CONDENSER_MK2),
    ("condenser_mk3", CONDENSER_MK3),
    ("condenser_mk3_input", CONDENSER_MK3_INPUT),
    ("condenser_mk3_output", CONDENSER_MK3_OUTPUT),
    ("condenser_mk1", ENERGY_CONDENSER),
    ("relay_mk1", RELAY_MK1),
    ("relay_mk2", RELAY_MK2),
    ("relay_mk3", RELAY_MK3),
    ("transmutation_table", TRANSMUTATION_TABLE),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuError {
    Json { file: &'static str, message: String },
    Schema { file: &'static str, message: &'static str },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuSpec {
    pub slots: u32,
    pub data_slots: u32,
    pub result_count: usize,
    pub first_result: Option<u32>,
    pub last_result: Option<u32>,
}

fn parse_file(file: &'static str, text: &str) -> Result<MenuSpec, MenuError> {
    let json: Value = serde_json::from_str(text).map_err(|error| MenuError::Json {
        file,
        message: error.to_string(),
    })?;
    let schema = |message| MenuError::Schema { file, message };
    let slots = json
        .get("slots")
        .and_then(Value::as_u64)
        .ok_or_else(|| schema("missing or non-integer slots"))?;
    let data_slots = json
        .get("data_slots")
        .and_then(Value::as_u64)
        .ok_or_else(|| schema("missing or non-integer data_slots"))?;
    if !(1..=256).contains(&slots) {
        return Err(schema("slots must be between 1 and 256"));
    }
    if data_slots > 256 {
        return Err(schema("data_slots must be at most 256"));
    }
    let mut seen = BTreeSet::new();
    for entry in json
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| schema("missing results array"))?
    {
        let slot = entry.as_u64().ok_or_else(|| schema("invalid result slot"))?;
        if slot >= slots || !seen.insert(slot) {
            return Err(schema("result slot out of range or duplicated"));
        }
    }
    let results: Vec<u32> = seen.iter().map(|slot| *slot as u32).collect();
    Ok(MenuSpec {
        slots: slots as u32,
        data_slots: data_slots as u32,
        result_count: results.len(),
        first_result: results.first().copied(),
        last_result: results.last().copied(),
    })
}

/// Validate every shipped menu declaration.
pub fn validate() -> Result<Vec<(&'static str, MenuSpec)>, MenuError> {
    FILES
        .iter()
        .map(|(name, text)| Ok((*name, parse_file(name, text)?)))
        .collect()
}

/// The menu declarations, keyed as `data_pack_files` expects.
pub fn data_pack_files() -> Vec<(String, Vec<u8>)> {
    // A malformed shipped declaration is a bug, not data to quietly ship:
    // report it loudly at every data load rather than letting the engine fail
    // later with an unexplained missing menu type.
    if let Err(error) = validate() {
        eprintln!("ProjectE-RS: invalid shipped menu declaration: {error:?}");
    }
    FILES
        .iter()
        .map(|(name, text)| {
            (
                format!("horizon/menu/projecte:{name}"),
                text.as_bytes().to_vec(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_shipped_menus_match_the_machine_layouts() {
        let specs: std::collections::BTreeMap<_, _> = validate().unwrap().into_iter().collect();
        for (name, slots, data_slots, result) in [
            ("collector_mk1", 11, 4, Some(1)),
            ("collector_mk2", 15, 4, Some(1)),
            ("collector_mk3", 19, 4, Some(1)),
            ("relay_mk1", 8, 3, Some(7)),
            ("relay_mk2", 14, 3, Some(13)),
            ("relay_mk3", 22, 3, Some(21)),
            ("condenser_mk1", 91, 2, None),
            ("alchemical_chest", 104, 0, None),
        ] {
            let spec = specs[name];
            assert_eq!(spec.slots, slots, "{name}");
            assert_eq!(spec.data_slots, data_slots, "{name}");
            assert_eq!(spec.first_result, result, "{name}");
        }
        let mk2 = specs["condenser_mk2"];
        assert_eq!(mk2.slots, 84);
        assert_eq!(mk2.result_count, 42);
        assert_eq!(mk2.first_result, Some(42));
        assert_eq!(mk2.last_result, Some(83));
        assert_eq!(specs["condenser_mk3"].slots, 91);
        assert_eq!(specs["condenser_mk3_input"].slots, 91);
        assert_eq!(specs["condenser_mk3_output"].slots, 180);
        assert_eq!(specs["condenser_mk3_output"].result_count, 180);
        assert_eq!(specs["transmutation_table"].slots, 27);
        assert_eq!(specs["transmutation_table"].data_slots, 1);
    }

    #[test]
    fn declarations_reject_invalid_shapes() {
        assert!(matches!(
            parse_file("bad", "{not json"),
            Err(MenuError::Json { .. })
        ));
        assert_eq!(
            parse_file("bad", r#"{"slots":0,"data_slots":0,"results":[]}"#),
            Err(MenuError::Schema {
                file: "bad",
                message: "slots must be between 1 and 256"
            })
        );
        assert_eq!(
            parse_file("bad", r#"{"slots":3,"data_slots":0,"results":[3]}"#),
            Err(MenuError::Schema {
                file: "bad",
                message: "result slot out of range or duplicated"
            })
        );
        assert_eq!(
            parse_file("bad", r#"{"slots":3,"data_slots":0,"results":[1,1]}"#),
            Err(MenuError::Schema {
                file: "bad",
                message: "result slot out of range or duplicated"
            })
        );
    }

    #[test]
    fn data_pack_files_are_well_formed_and_deterministic() {
        let files = data_pack_files();
        assert_eq!(files.len(), FILES.len());
        assert_eq!(data_pack_files(), files);
        let mut keys = BTreeSet::new();
        for (index, (key, bytes)) in files.iter().enumerate() {
            assert!(keys.insert(key), "duplicate key {key}");
            assert_eq!(key, &format!("horizon/menu/projecte:{}", FILES[index].0));
            let text = std::str::from_utf8(bytes).unwrap();
            assert_eq!(text, FILES[index].1);
            let value: Value = serde_json::from_str(text).unwrap();
            assert!(value.get("slots").is_some(), "{key}");
        }
    }
}
