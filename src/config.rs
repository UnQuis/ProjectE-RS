//! ProjectE-RS settings: the covalence loss override.
//!
//! The hook wiring lives in `lib.rs`; this module owns the default text, the
//! strict parser and the process-wide value. Parsing is a pure function so
//! every rejection (out-of-range, malformed, unknown key) is covered by tests.
//! The numeric conversion itself is `transaction::Covalence`'s — no second
//! fixed-point implementation lives here.

use std::sync::Mutex;

use mod_api::{Config, ConfigKind, RegisterConfigsEvent};

pub(crate) const MOD_ID: &str = "projecte";

/// The shipped `config/projecte-common.toml`.
pub(crate) const DEFAULT_TEXT: &str = "\
# ProjectE-RS: EMC sell-value multiplier (0.1 .. 1.0) and low-value rounding.\n\
covalence_loss = 1.0\n\
covalence_rounding = true\n";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Settings {
    pub loss: f64,
    pub rounding: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            loss: super::machines::COVALENCE_DEFAULT,
            rounding: super::machines::COVALENCE_ROUND_DEFAULT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ConfigError {
    InvalidLine(usize),
    UnknownKey(String),
    InvalidNumber(String),
    InvalidBool(String),
    DuplicateKey(String),
}

impl ConfigError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::InvalidLine(line) => format!("malformed config line {line}"),
            Self::UnknownKey(key) => format!("unknown config key {key:?}"),
            Self::InvalidNumber(value) => {
                format!("covalence_loss must be a number in 0.1..=1.0, got {value:?}")
            }
            Self::InvalidBool(value) => format!("covalence_rounding must be true/false, got {value:?}"),
            Self::DuplicateKey(key) => format!("duplicate config key {key:?}"),
        }
    }
}

/// Parse a config file's text. Missing keys keep their defaults; a present but
/// invalid value is refused rather than silently clamped (fail explicitly).
pub(crate) fn parse(text: &str) -> Result<Settings, ConfigError> {
    let mut settings = Settings::default();
    let mut seen_loss = false;
    let mut seen_rounding = false;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or(ConfigError::InvalidLine(index + 1))?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "covalence_loss" => {
                if seen_loss {
                    return Err(ConfigError::DuplicateKey(key.to_owned()));
                }
                seen_loss = true;
                let loss: f64 = value.parse().map_err(|_| ConfigError::InvalidNumber(value.to_owned()))?;
                if !super::transaction::Covalence::new(loss, settings.rounding).is_some() {
                    return Err(ConfigError::InvalidNumber(value.to_owned()));
                }
                settings.loss = loss;
            }
            "covalence_rounding" => {
                if seen_rounding {
                    return Err(ConfigError::DuplicateKey(key.to_owned()));
                }
                seen_rounding = true;
                settings.rounding = match value {
                    "true" => true,
                    "false" => false,
                    _ => return Err(ConfigError::InvalidBool(value.to_owned())),
                };
            }
            _ => return Err(ConfigError::UnknownKey(key.to_owned())),
        }
    }
    Ok(settings)
}

static SETTINGS: Mutex<Settings> = Mutex::new(Settings {
    loss: super::machines::COVALENCE_DEFAULT,
    rounding: super::machines::COVALENCE_ROUND_DEFAULT,
});

fn store(settings: Settings) {
    match SETTINGS.lock() {
        Ok(mut guard) => *guard = settings,
        Err(poisoned) => *poisoned.into_inner() = settings,
    }
}

/// The live settings, for hooks that need both fields.
pub(crate) fn current() -> Settings {
    SETTINGS
        .lock()
        .map(|guard| *guard)
        .unwrap_or_else(|poisoned| *poisoned.into_inner())
}

/// The live covalence value, in the one implementation that computes prices.
pub(crate) fn covalence() -> super::transaction::Covalence {
    let settings = current();
    super::transaction::Covalence::new(settings.loss, settings.rounding)
        .expect("loaded settings were validated at parse time")
}

/// For `Hooks::register_configs`.
pub(crate) fn register(event: &mut RegisterConfigsEvent<'_>) {
    event.configs.push(Config::new(MOD_ID, ConfigKind::Common, DEFAULT_TEXT));
}

fn apply(config: &mod_api::ConfigFile) {
    if config.mod_id != MOD_ID {
        return;
    }
    match parse(&config.text) {
        Ok(settings) => store(settings),
        Err(error) => eprintln!(
            "[projecte] ignoring invalid {}: {}; keeping {:.1} covalence loss",
            config.file,
            error.message(),
            super::transaction::Covalence::DEFAULT.loss()
        ),
    }
}

/// For both `Hooks::config_loading` and `Hooks::config_reloading`.
pub(crate) fn loading(config: &mod_api::ConfigFile) {
    apply(config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_config_round_trips() {
        assert_eq!(parse(DEFAULT_TEXT).unwrap(), Settings::default());
    }

    #[test]
    fn missing_keys_keep_the_reference_defaults() {
        assert_eq!(parse("").unwrap(), Settings::default());
        assert_eq!(
            parse("covalence_loss = 0.5\n").unwrap(),
            Settings { loss: 0.5, rounding: true }
        );
    }

    #[test]
    fn comments_and_whitespace_are_allowed() {
        let text = "  # note\ncovalence_loss = 0.25 # inline\ncovalence_rounding = false\n";
        assert_eq!(
            parse(text).unwrap(),
            Settings { loss: 0.25, rounding: false }
        );
    }

    #[test]
    fn values_outside_range_are_refused_not_clamped() {
        for value in ["0.0", "1.1", "abc", "-0.5", "NaN"] {
            assert!(parse(&format!("covalence_loss = {value}")).is_err(), "{value}");
        }
        assert!(parse("covalence_rounding = yes").is_err());
    }

    #[test]
    fn unknown_and_duplicate_keys_are_refused() {
        assert!(matches!(
            parse("foo = 1"),
            Err(ConfigError::UnknownKey(_))
        ));
        assert!(matches!(
            parse("covalence_loss = 0.5\ncovalence_loss = 0.6"),
            Err(ConfigError::DuplicateKey(_))
        ));
        assert!(matches!(parse("no equals sign"), Err(ConfigError::InvalidLine(1))));
    }

    #[test]
    fn the_live_covalence_is_the_one_used_for_prices() {
        let saved = current();
        store(Settings { loss: 0.5, rounding: false });
        assert!((covalence().loss() - 0.5).abs() < f64::EPSILON);
        assert_eq!(covalence().sell_value(10), Some(5));
        store(saved);
    }
}
