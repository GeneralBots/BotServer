//! Every translation key exposed by the i18n API.

use super::{t, Locale};

mod part1;
mod part2;

/// Every translation key exposed by the API, in catalogue order.
fn translation_keys() -> impl Iterator<Item = &'static str> {
    part1::TRANSLATION_KEYS
        .iter()
        .copied()
        .chain(part2::TRANSLATION_KEYS.iter().copied())
}

pub fn get_translations_json(locale: &Locale) -> serde_json::Value {
    let mut translations = serde_json::Map::new();

    for key in translation_keys() {
        let value = t(locale, key);
        if value.contains('[') && value == format!("[{key}]") {
            log::warn!("i18n: Missing translation for key: {} in locale: {:?}", key, locale);
        }
        translations.insert(key.to_string(), serde_json::Value::String(value));
    }

    log::info!("i18n: Returning {} translations for locale: {}", translations.len(), locale);
    serde_json::Value::Object(translations)
}

