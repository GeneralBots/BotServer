use super::*;

impl Locator {
    pub fn css(selector: &str) -> Self {
        Self::Css(selector.to_string())
    }

    pub fn xpath(expr: &str) -> Self {
        Self::XPath(expr.to_string())
    }

    pub fn id(id: &str) -> Self {
        Self::Id(id.to_string())
    }

    pub fn as_selector(&self) -> &str {
        match self {
            Self::Css(s) | Self::XPath(s) | Self::Id(s) => s,
        }
    }
}
