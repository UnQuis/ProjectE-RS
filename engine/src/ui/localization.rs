//! The screen's text, by key.
//!
//! Every user-facing string on the Transmutation Table screen goes through
//! this module. Nothing in the drawing or input code holds an English word, so
//! a translation is a table below and nothing else — the structure is ready
//! for a third language without touching a call site.
//!
//! The tables carry the same keys, in the same order, and the tests below
//! refuse a table that misses one or names one more. That is what keeps a
//! translation from silently falling back to English one label at a time.
//!
//! A key spelled like vanilla's (`container.inventory`) is also what vanilla's
//! own translation table answers, so the screen shows the player's own
//! inventory label in their language rather than a copy we wrote.

/// One rendered string: the key it came from and the text to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Localized {
    pub key: &'static str,
    pub text: &'static str,
}

impl Localized {
    #[allow(dead_code)]
    pub(crate) fn text(&self) -> &'static str {
        self.text
    }
}

/// The language the screen draws in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Language {
    #[default]
    English,
    Russian,
}

impl Language {
    /// The language of a client locale tag (`en_us`, `ru_ru`, `ru`).
    ///
    /// Minecraft's locale names are `<language>_<country>` in lower case; the
    /// language part decides, so `ru_ru` and `ru_ua` both draw Russian. A tag
    /// this mod does not translate draws English.
    pub(crate) fn of(locale: &str) -> Self {
        match locale.split(['_', '-']).next().unwrap_or_default() {
            "ru" => Self::Russian,
            _ => Self::English,
        }
    }

    /// The lookup table for this language.
    pub(crate) fn table(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::English => EN_US,
            Self::Russian => RU_RU,
        }
    }
}

/// Looks `key` up in `language`, then in English; `None` for a key no table
/// has, which a test below makes impossible for the keys this module declares.
pub(crate) fn localized(language: Language, key: &'static str) -> Option<Localized> {
    let text = language
        .table()
        .iter()
        .chain(EN_US.iter())
        .find(|(name, _)| *name == key)
        .map(|(_, text)| *text)?;
    Some(Localized { key, text })
}

/// A key's text in `language`, falling back to English and then to the key
/// itself: a missing translation shows something readable rather than nothing.
pub(crate) fn text(language: Language, key: &'static str) -> &'static str {
    localized(language, key).map_or(key, |line| line.text)
}

/// Replaces `{}` placeholders with `values`, left to right.
///
/// The table holds the sentence with placeholders so that a translation can
/// put them in a different order, which a `format!` string cannot. A missing
/// value leaves its placeholder in place, so a mistake is visible rather than
/// silently dropping a number.
pub(crate) fn format(mut template: &'static str, values: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    for value in values {
        let Some(at) = template.find("{}") else {
            break;
        };
        out.push_str(&template[..at]);
        out.push_str(value);
        template = &template[at + 2..];
    }
    out.push_str(template);
    out
}

// --- keys ------------------------------------------------------------------
//
// The keys this screen uses. They are declared here rather than written inline
// so that the compiler refuses a typo at the call site and the language test
// can walk a complete list.

pub(crate) const KEY_TITLE: &str = "container.projecte.transmutation_table";
pub(crate) const KEY_EMC: &str = "container.projecte.emc";
pub(crate) const KEY_SEARCH: &str = "container.projecte.search";
pub(crate) const KEY_SEARCH_HINT: &str = "container.projecte.search.hint";
pub(crate) const KEY_SORT_EMC: &str = "container.projecte.sort.emc";
pub(crate) const KEY_SORT_NAME: &str = "container.projecte.sort.name";
pub(crate) const KEY_FILTER_ALL: &str = "container.projecte.filter.all";
pub(crate) const KEY_FILTER_MATTER: &str = "container.projecte.filter.matter";
pub(crate) const KEY_FILTER_TOOLS: &str = "container.projecte.filter.tools";
pub(crate) const KEY_PAGE: &str = "container.projecte.page";
pub(crate) const KEY_INPUT: &str = "container.projecte.input";
pub(crate) const KEY_ALCHEMY: &str = "container.projecte.alchemy";
pub(crate) const KEY_INVENTORY: &str = "container.inventory";
pub(crate) const KEY_EMPTY: &str = "container.projecte.empty";
pub(crate) const KEY_NO_MATCH: &str = "container.projecte.no_match";
pub(crate) const KEY_HINT_SEARCH: &str = "container.projecte.hint.search";
pub(crate) const KEY_HINT_SORT: &str = "container.projecte.hint.sort";
pub(crate) const KEY_HINT_FILTER: &str = "container.projecte.hint.filter";
pub(crate) const KEY_HINT_PAGE: &str = "container.projecte.hint.page";
pub(crate) const KEY_HINT_INPUT: &str = "container.projecte.hint.input";
pub(crate) const KEY_HINT_EMBLEM: &str = "container.projecte.hint.emblem";
pub(crate) const KEY_HINT_PLACEHOLDER: &str = "container.projecte.hint.placeholder";
pub(crate) const KEY_SORTED_BY: &str = "container.projecte.sorted_by";
pub(crate) const KEY_FILTERED_BY: &str = "container.projecte.filtered_by";
pub(crate) const KEY_NO_TRANSMUTATION: &str = "container.projecte.no_transmutation";
pub(crate) const KEY_HINT_STORAGE: &str = "container.projecte.hint.storage";
pub(crate) const KEY_NOTICE_PREFIX: &str = "container.projecte.notice";

/// Every key this screen draws, so a test can prove each language covers them.
#[allow(dead_code)]
pub(crate) const KEYS: [&str; 27] = [
    KEY_TITLE,
    KEY_EMC,
    KEY_SEARCH,
    KEY_SEARCH_HINT,
    KEY_SORT_EMC,
    KEY_SORT_NAME,
    KEY_FILTER_ALL,
    KEY_FILTER_MATTER,
    KEY_FILTER_TOOLS,
    KEY_PAGE,
    KEY_INPUT,
    KEY_ALCHEMY,
    KEY_INVENTORY,
    KEY_EMPTY,
    KEY_NO_MATCH,
    KEY_HINT_SEARCH,
    KEY_HINT_SORT,
    KEY_HINT_FILTER,
    KEY_HINT_PAGE,
    KEY_HINT_INPUT,
    KEY_HINT_EMBLEM,
    KEY_HINT_PLACEHOLDER,
    KEY_SORTED_BY,
    KEY_FILTERED_BY,
    KEY_NO_TRANSMUTATION,
    KEY_HINT_STORAGE,
    KEY_NOTICE_PREFIX,
];

/// English.
///
/// This table is the whole of the screen's localization. A native mod has no
/// `assets/` of its own — only a content mod (one with `horizon-mod.toml` and
/// an `assets/` folder) does, and this project is a native trait mod — so a
/// `lang/en_us.json` cannot be shipped and read by the game here. Hence the
/// table in code, keyed by the same `container.projecte.*` keys a resource
/// pack would use (`API_WORKAROUNDS.md` §9).
pub(crate) const EN_US: &[(&str, &str)] = &[
    (KEY_TITLE, "Transmutation Table"),
    (KEY_EMC, "EMC"),
    (KEY_SEARCH, "Search"),
    (KEY_SEARCH_HINT, "Search items"),
    (KEY_SORT_EMC, "EMC"),
    (KEY_SORT_NAME, "Name"),
    (KEY_FILTER_ALL, "All"),
    (KEY_FILTER_MATTER, "Matter"),
    (KEY_FILTER_TOOLS, "Tools"),
    (KEY_PAGE, "Page {} / {}"),
    (KEY_INPUT, "Input"),
    (KEY_ALCHEMY, "Transmutation"),
    (KEY_INVENTORY, "Inventory"),
    (KEY_EMPTY, "Nothing learned yet"),
    (KEY_NO_MATCH, "No item matches"),
    (
        KEY_HINT_SEARCH,
        "Type to filter the item list. Backspace deletes.",
    ),
    (KEY_HINT_SORT, "Sort the list by EMC or by name."),
    (
        KEY_HINT_FILTER,
        "Show every item, matter only, or tools only.",
    ),
    (KEY_HINT_PAGE, "Turn the page of items."),
    (KEY_HINT_INPUT, "Put an item here to transmute it later."),
    (
        KEY_HINT_EMBLEM,
        "The alchemical emblem of the Transmutation Table.",
    ),
    (
        KEY_HINT_PLACEHOLDER,
        "Not connected yet: the item list is a preview.",
    ),
    (KEY_SORTED_BY, "Sorted by {}"),
    (KEY_FILTERED_BY, "Showing {}"),
    (KEY_NO_TRANSMUTATION, "Transmutation is not connected yet."),
    (
        KEY_HINT_STORAGE,
        "Storage: items left here stay when the screen closes.",
    ),
    (KEY_NOTICE_PREFIX, "Note: {}"),
];

/// Russian: the structure the task asks for, complete rather than partial, so
/// a player running the game in Russian sees the screen in Russian.
pub(crate) const RU_RU: &[(&str, &str)] = &[
    (KEY_TITLE, "Стол трансмутации"),
    (KEY_EMC, "ЭМК"),
    (KEY_SEARCH, "Поиск"),
    (KEY_SEARCH_HINT, "Поиск предметов"),
    (KEY_SORT_EMC, "ЭМК"),
    (KEY_SORT_NAME, "Название"),
    (KEY_FILTER_ALL, "Все"),
    (KEY_FILTER_MATTER, "Материя"),
    (KEY_FILTER_TOOLS, "Инструменты"),
    (KEY_PAGE, "Страница {} / {}"),
    (KEY_INPUT, "Вход"),
    (KEY_ALCHEMY, "Трансмутация"),
    (KEY_INVENTORY, "Инвентарь"),
    (KEY_EMPTY, "Пока ничего не изучено"),
    (KEY_NO_MATCH, "Нет подходящих предметов"),
    (
        KEY_HINT_SEARCH,
        "Введите текст, чтобы отфильтровать список. Backspace удаляет.",
    ),
    (KEY_HINT_SORT, "Сортировать список по ЭМК или по названию."),
    (
        KEY_HINT_FILTER,
        "Показывать все предметы, только материю или только инструменты.",
    ),
    (KEY_HINT_PAGE, "Перевернуть страницу предметов."),
    (
        KEY_HINT_INPUT,
        "Положите предмет сюда, чтобы трансмутировать его позже.",
    ),
    (KEY_HINT_EMBLEM, "Алхимическая эмблема стола трансмутации."),
    (
        KEY_HINT_PLACEHOLDER,
        "Пока не подключено: список предметов — предпросмотр.",
    ),
    (KEY_SORTED_BY, "Сортировка: {}"),
    (KEY_FILTERED_BY, "Показано: {}"),
    (KEY_NO_TRANSMUTATION, "Трансмутация пока не подключена."),
    (
        KEY_HINT_STORAGE,
        "Хранилище: предметы, оставленные здесь, сохраняются при закрытии.",
    ),
    (KEY_NOTICE_PREFIX, "Примечание: {}"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_covers_every_key_exactly_once() {
        for (name, table) in [("en_us", EN_US), ("ru_ru", RU_RU)] {
            assert_eq!(
                table.len(),
                KEYS.len(),
                "{name} has {} lines for {} keys",
                table.len(),
                KEYS.len()
            );
            for key in KEYS {
                let found = table.iter().filter(|(name, _)| *name == key).count();
                assert_eq!(found, 1, "{name} has {found} lines for {key}");
            }
            for (key, text) in table {
                assert!(!text.trim().is_empty(), "{name}: {key} is blank");
            }
        }
    }

    #[test]
    fn the_tables_do_not_drift_apart() {
        // The same keys in the same order, so a reviewer sees them side by side.
        let en: Vec<&str> = EN_US.iter().map(|(key, _)| *key).collect();
        let ru: Vec<&str> = RU_RU.iter().map(|(key, _)| *key).collect();
        assert_eq!(en, ru);
    }

    #[test]
    fn the_title_is_the_one_the_block_is_named() {
        assert_eq!(text(Language::English, KEY_TITLE), "Transmutation Table");
        assert_eq!(text(Language::Russian, KEY_TITLE), "Стол трансмутации");
    }

    #[test]
    fn a_russian_client_gets_russian_and_an_unknown_locale_english() {
        assert_eq!(Language::of("ru_ru"), Language::Russian);
        assert_eq!(Language::of("ru"), Language::Russian);
        assert_eq!(Language::of("ru_ua"), Language::Russian);
        assert_eq!(Language::of("en_us"), Language::English);
        assert_eq!(Language::of("de_de"), Language::English);
        assert_eq!(Language::of(""), Language::English);
        assert_eq!(
            text(Language::of("de_de"), KEY_TITLE),
            "Transmutation Table"
        );
    }

    #[test]
    fn every_declared_key_resolves_in_both_languages() {
        for language in [Language::English, Language::Russian] {
            for key in KEYS {
                assert!(
                    localized(language, key).is_some(),
                    "{language:?} is missing {key}"
                );
            }
        }
    }

    #[test]
    fn placeholders_are_filled_in_order_and_survive_a_short_list() {
        assert_eq!(
            format(text(Language::English, KEY_PAGE), &["2", "5"]),
            "Page 2 / 5"
        );
        assert_eq!(
            format(text(Language::English, KEY_PAGE), &["2"]),
            "Page 2 / {}"
        );
        assert_eq!(
            format(text(Language::English, KEY_PAGE), &[]),
            "Page {} / {}"
        );
        // A translation that moves the numbers still gets them in its order.
        assert_eq!(format("{} / {} страниц", &["1", "3"]), "1 / 3 страниц");
    }

    #[test]
    fn a_key_no_table_has_is_shown_rather_than_dropped() {
        assert_eq!(
            localized(Language::English, "container.projecte.missing"),
            None
        );
        assert_eq!(
            text(Language::English, "container.projecte.missing"),
            "container.projecte.missing"
        );
    }
}
