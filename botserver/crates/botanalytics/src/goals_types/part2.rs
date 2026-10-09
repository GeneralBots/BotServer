use super::*;

impl Visibility {
    pub fn from_str(s: &str) -> Self {
        match s {
            "private" => Self::Private,
            "organization" => Self::Organization,
            _ => Self::Team,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Team => "team",
            Self::Organization => "organization",
        }
    }
}
