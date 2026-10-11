//! Locale negotiation, extractors and the public i18n surface.
//!
//! The translation catalogue lives in [`translations`].

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::ACCEPT_LANGUAGE, request::Parts},
};
use botlib::i18n::{self, Locale as BotlibLocale, MessageArgs as BotlibMessageArgs};

mod error;
mod routes;
mod translations;
#[cfg(test)]
mod tests;

pub use error::LocalizedError;
pub use routes::configure_i18n_routes;
pub use translations::get_translations_json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locale {
    language: String,
    region: Option<String>,
}

impl Locale {
    pub fn new(locale_str: &str) -> Option<Self> {
        if locale_str.is_empty() {
            return None;
        }

        let parts: Vec<&str> = locale_str.split(&['-', '_'][..]).collect();

        let language = parts.first()?.to_lowercase();
        if language.len() < 2 || language.len() > 3 {
            return None;
        }

        let region = parts.get(1).map(|r| r.to_uppercase());

        Some(Self { language, region })
    }

    #[must_use]
    pub fn language(&self) -> &str {
        &self.language
    }

    #[must_use]
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    #[must_use]
    pub fn to_bcp47(&self) -> String {
        match &self.region {
            Some(r) => format!("{}-{r}", self.language),
            None => self.language.clone(),
        }
    }

    fn to_botlib_locale(&self) -> BotlibLocale {
        BotlibLocale::new(&self.to_bcp47()).unwrap_or_default()
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            region: None,
        }
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_bcp47())
    }
}

const AVAILABLE_LOCALES: &[&str] = &["en", "pt-BR", "es", "zh-CN", "fr", "de", "ja", "ko"];

pub struct RequestLocale(pub Locale);

impl RequestLocale {
    #[must_use]
    pub fn locale(&self) -> &Locale {
        &self.0
    }

    #[must_use]
    pub fn language(&self) -> &str {
        self.0.language()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RequestLocale
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let locale = parts
            .headers
            .get(ACCEPT_LANGUAGE)
            .and_then(|h| h.to_str().ok())
            .map(parse_accept_language)
            .and_then(|langs| negotiate_locale(&langs))
            .unwrap_or_default();

        Ok(Self(locale))
    }
}

fn parse_accept_language(header: &str) -> Vec<(String, f32)> {
    let mut langs: Vec<(String, f32)> = header
        .split(',')
        .filter_map(|part| {
            let mut iter = part.trim().split(';');
            let lang = iter.next()?.trim().to_string();

            if lang.is_empty() || lang == "*" {
                return None;
            }

            let quality = iter
                .next()
                .and_then(|q| q.trim().strip_prefix("q="))
                .and_then(|q| q.parse().ok())
                .unwrap_or(1.0);

            Some((lang, quality))
        })
        .collect();

    langs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    langs
}

fn negotiate_locale(requested: &[(String, f32)]) -> Option<Locale> {
    for (lang, _) in requested {
        let requested_locale = Locale::new(lang)?;

        for available in AVAILABLE_LOCALES {
            let avail_locale = Locale::new(available)?;

            if requested_locale.language == avail_locale.language
                && requested_locale.region == avail_locale.region
            {
                return Some(avail_locale);
            }
        }

        for available in AVAILABLE_LOCALES {
            let avail_locale = Locale::new(available)?;
            if requested_locale.language == avail_locale.language {
                return Some(avail_locale);
            }
        }
    }

    Some(Locale::default())
}

pub type MessageArgs = botlib::i18n::MessageArgs;

pub fn init_i18n(locales_path: &str) -> Result<(), String> {
    i18n::init(locales_path).map_err(|e| format!("Failed to initialize i18n: {e}"))
}

pub fn is_i18n_initialized() -> bool {
    i18n::is_initialized()
}

pub fn t(locale: &Locale, key: &str) -> String {
    t_with_args(locale, key, None)
}

pub fn t_with_args(locale: &Locale, key: &str, args: Option<&MessageArgs>) -> String {
    let botlib_locale = locale.to_botlib_locale();
    let botlib_args: Option<BotlibMessageArgs> = args.map(|a| {
        a.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    });
    i18n::get_with_args(&botlib_locale, key, botlib_args.as_ref())
}

pub fn available_locales() -> Vec<String> {
    if is_i18n_initialized() {
        i18n::available_locales()
    } else {
        AVAILABLE_LOCALES.iter().map(|s| (*s).to_string()).collect()
    }
}

