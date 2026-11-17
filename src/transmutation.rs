//! Data-driven world transmutations for the Philosopher's Stone.
//!
//! The four files in `data/projecte/pe_world_transmutations/` are adapted from
//! ProjectE (MIT, `github.com/sinkillerj/ProjectE`); each file carries that
//! attribution in its `comment` field. They are shipped through
//! `data_pack_files` under `pe_world_transmutations/projecte:<name>` so data
//! packs can override them, and parsed here into a server-authoritative lookup
//! used when the stone is right-clicked on a block.
//!
//! Parsing is deliberately strict: a malformed file or an id that is not a
//! `namespace:path` identifier is rejected instead of silently producing an
//! empty rule set (which would make the stone appear broken for every block).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use mod_api::serde_json::{self, Value};

const DEFAULTS: &str = include_str!("../data/projecte/pe_world_transmutations/defaults.json");
const COLORS: &str = include_str!("../data/projecte/pe_world_transmutations/colors.json");
const OXIDIZATION: &str = include_str!("../data/projecte/pe_world_transmutations/oxidization.json");
const WOOD: &str = include_str!("../data/projecte/pe_world_transmutations/wood.json");

/// (file name, embedded bytes), in a fixed order so emitted files are stable.
const FILES: &[(&str, &str)] = &[
    ("defaults", DEFAULTS),
    ("colors", COLORS),
    ("oxidization", OXIDIZATION),
    ("wood", WOOD),
];

const ATTRIBUTION: &str = "github.com/sinkillerj/ProjectE";

/// A block id with optional state properties, as used by ProjectE's schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockSpec {
    pub name: String,
    pub properties: BTreeMap<String, String>,
}

/// One world-transmutation rule: `origin` becomes `result`, or `alt_result`
/// when the player is sneaking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub origin: BlockSpec,
    pub result: BlockSpec,
    pub alt_result: Option<BlockSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransmutationError {
    /// The file is not valid JSON.
    Json { file: &'static str, message: String },
    /// A field has the wrong shape or the MIT attribution is missing.
    Schema {
        file: &'static str,
        entry: usize,
        message: &'static str,
    },
}

fn valid_id(id: &str) -> bool {
    let Some((namespace, path)) = id.split_once(':') else {
        return false;
    };
    if namespace.is_empty() || path.is_empty() || path.contains(':') {
        return false;
    }
    namespace
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-'))
        && path.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-' | b'/')
        })
}

fn parse_spec(
    value: &Value,
    file: &'static str,
    entry: usize,
    field: &'static str,
) -> Result<BlockSpec, TransmutationError> {
    let invalid = || TransmutationError::Schema {
        file,
        entry,
        message: match field {
            "origin" => "invalid origin block",
            "result" => "invalid result block",
            _ => "invalid alt_result block",
        },
    };
    match value {
        Value::String(id) if valid_id(id) => Ok(BlockSpec {
            name: id.clone(),
            properties: BTreeMap::new(),
        }),
        Value::Object(object) => {
            let name = object.get("Name").and_then(Value::as_str).ok_or_else(invalid)?;
            if !valid_id(name) {
                return Err(invalid());
            }
            let mut properties = BTreeMap::new();
            match object.get("Properties") {
                None | Some(Value::Null) => {}
                Some(Value::Object(map)) => {
                    for (key, value) in map {
                        let value = value.as_str().ok_or_else(invalid)?;
                        properties.insert(key.clone(), value.to_owned());
                    }
                }
                Some(_) => return Err(invalid()),
            }
            for key in object.keys() {
                if key != "Name" && key != "Properties" {
                    return Err(invalid());
                }
            }
            Ok(BlockSpec {
                name: name.to_owned(),
                properties,
            })
        }
        _ => Err(invalid()),
    }
}

fn parse_file(file: &'static str, text: &str) -> Result<Vec<Rule>, TransmutationError> {
    let root: Value = serde_json::from_str(text).map_err(|error| TransmutationError::Json {
        file,
        message: error.to_string(),
    })?;
    let schema = |message| TransmutationError::Schema {
        file,
        entry: 0,
        message,
    };
    let comment = root
        .get("comment")
        .and_then(Value::as_str)
        .ok_or_else(|| schema("missing comment"))?;
    if !comment.contains(ATTRIBUTION) {
        return Err(schema("missing MIT attribution"));
    }
    let entries = root
        .get("transmutations")
        .and_then(Value::as_array)
        .ok_or_else(|| schema("transmutations must be an array"))?;
    let mut rules = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let object = entry.as_object().ok_or_else(|| TransmutationError::Schema {
            file,
            entry: index,
            message: "entry must be an object",
        })?;
        for key in object.keys() {
            if key != "origin" && key != "result" && key != "alt_result" {
                return Err(TransmutationError::Schema {
                    file,
                    entry: index,
                    message: "unknown entry field",
                });
            }
        }
        let origin = parse_spec(
            object
                .get("origin")
                .ok_or_else(|| TransmutationError::Schema {
                    file,
                    entry: index,
                    message: "missing origin",
                })?,
            file,
            index,
            "origin",
        )?;
        let result = parse_spec(
            object
                .get("result")
                .ok_or_else(|| TransmutationError::Schema {
                    file,
                    entry: index,
                    message: "missing result",
                })?,
            file,
            index,
            "result",
        )?;
        let alt_result = object
            .get("alt_result")
            .map(|value| parse_spec(value, file, index, "alt_result"))
            .transpose()?;
        rules.push(Rule {
            origin,
            result,
            alt_result,
        });
    }
    Ok(rules)
}

static RULES: OnceLock<Result<Vec<Rule>, TransmutationError>> = OnceLock::new();

/// Parse all four files once. A parse failure is returned to the caller so the
/// hook can report it instead of quietly disabling world transmutation.
pub fn rules() -> Result<&'static [Rule], TransmutationError> {
    RULES
        .get_or_init(|| {
            let mut all = Vec::new();
            for (name, text) in FILES {
                all.extend(parse_file(name, text)?);
            }
            Ok(all)
        })
        .as_deref()
        .map_err(Clone::clone)
}

/// The result of transmuting `origin`, or `None` when no rule covers it.
/// Sneaking selects `alt_result` when the rule defines one.
pub fn resolve(origin: &str, sneaking: bool) -> Option<&'static str> {
    let rules = rules().ok()?;
    let rule = rules.iter().find(|rule| rule.origin.name == origin)?;
    if sneaking {
        Some(
            rule.alt_result
                .as_ref()
                .unwrap_or(&rule.result)
                .name
                .as_str(),
        )
    } else {
        Some(rule.result.name.as_str())
    }
}

/// The four data-pack files that carry these rules.
pub fn data_pack_files() -> Vec<(String, Vec<u8>)> {
    FILES
        .iter()
        .map(|(name, text)| (format!("pe_world_transmutations/projecte:{name}"), text.as_bytes().to_vec()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_reference_files_parse_with_expected_counts() {
        for (name, expected) in [
            ("defaults", 27),
            ("colors", 112),
            ("oxidization", 36),
            ("wood", 188),
        ] {
            let text = FILES.iter().find(|(file, _)| *file == name).unwrap().1;
            assert_eq!(parse_file(name, text).unwrap().len(), expected, "{name}");
        }
    }

    #[test]
    fn every_file_keeps_its_mit_attribution() {
        for (name, text) in FILES {
            let comment = serde_json::from_str::<Value>(text).unwrap()["comment"]
                .as_str()
                .unwrap()
                .to_owned();
            assert!(comment.contains(ATTRIBUTION), "{name}: {comment}");
        }
    }

    #[test]
    fn loading_rejects_bad_json_missing_attribution_and_bad_ids() {
        assert!(matches!(
            parse_file("bad", "{not json"),
            Err(TransmutationError::Json { .. })
        ));
        assert_eq!(
            parse_file(
                "bad",
                r#"{"comment":"no attribution","transmutations":[]}"#
            ),
            Err(TransmutationError::Schema {
                file: "bad",
                entry: 0,
                message: "missing MIT attribution"
            })
        );
        assert_eq!(
            parse_file(
                "bad",
                r#"{"comment":"MIT github.com/sinkillerj/ProjectE","transmutations":[{"origin":"diamond","result":"minecraft:stone"}]}"#
            ),
            Err(TransmutationError::Schema {
                file: "bad",
                entry: 0,
                message: "invalid origin block"
            })
        );
    }

    #[test]
    fn block_specs_accept_string_and_object_forms() {
        let text = r#"{
            "comment":"MIT github.com/sinkillerj/ProjectE",
            "transmutations":[{
                "origin":"minecraft:stone",
                "result":{"Name":"minecraft:water","Properties":{"level":"0"}},
                "alt_result":"minecraft:grass_block"
            }]
        }"#;
        let parsed = parse_file("test", text).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].origin.name, "minecraft:stone");
        assert_eq!(parsed[0].result.name, "minecraft:water");
        assert_eq!(parsed[0].result.properties["level"], "0");
        assert_eq!(
            parsed[0].alt_result.as_ref().unwrap().name,
            "minecraft:grass_block"
        );
    }

    #[test]
    fn rules_have_unique_origins_and_valid_ids() {
        let all = rules().unwrap();
        assert_eq!(all.len(), 27 + 112 + 36 + 188);
        let mut origins = std::collections::BTreeSet::new();
        for rule in all {
            assert!(valid_id(&rule.origin.name), "{:?}", rule.origin);
            assert!(valid_id(&rule.result.name), "{:?}", rule.result);
            assert!(origins.insert(&rule.origin.name), "duplicate {:?}", rule.origin);
            if let Some(alt) = &rule.alt_result {
                assert!(valid_id(&alt.name), "{alt:?}");
            }
        }
    }

    #[test]
    fn reference_pairs_resolve_in_both_directions() {
        assert_eq!(resolve("minecraft:stone", false), Some("minecraft:cobblestone"));
        assert_eq!(resolve("minecraft:stone", true), Some("minecraft:grass_block"));
        assert_eq!(resolve("minecraft:cobblestone", false), Some("minecraft:stone"));
        assert_eq!(resolve("minecraft:gravel", false), Some("minecraft:sandstone"));
        assert_eq!(resolve("minecraft:water", false), Some("minecraft:ice"));
        assert_eq!(resolve("minecraft:oak_log", false), Some("minecraft:birch_log"));
        assert_eq!(resolve("minecraft:oak_log", true), Some("minecraft:cherry_log"));
        assert_eq!(
            resolve("minecraft:white_concrete", false),
            Some("minecraft:orange_concrete")
        );
        assert_eq!(
            resolve("minecraft:copper_block", true),
            Some("minecraft:oxidized_copper")
        );
    }

    #[test]
    fn unknown_and_property_specific_origins_do_not_resolve() {
        assert_eq!(resolve("minecraft:diamond", false), None);
        assert_eq!(resolve("example:stone", false), None);
        assert_eq!(resolve("", false), None);
    }

    #[test]
    fn data_pack_files_are_well_formed_deterministic_and_attributed() {
        let files = data_pack_files();
        assert_eq!(files.len(), 4);
        assert_eq!(data_pack_files(), files);
        let mut keys = std::collections::BTreeSet::new();
        for (index, (key, bytes)) in files.iter().enumerate() {
            assert!(keys.insert(key), "duplicate key {key}");
            assert_eq!(key, &format!("pe_world_transmutations/projecte:{}", FILES[index].0));
            let text = std::str::from_utf8(bytes).unwrap();
            assert_eq!(text, FILES[index].1);
            let value: Value = serde_json::from_str(text).unwrap();
            assert!(value["comment"].as_str().unwrap().contains(ATTRIBUTION));
            assert!(value["transmutations"].as_array().unwrap().len() > 0);
        }
    }
}
