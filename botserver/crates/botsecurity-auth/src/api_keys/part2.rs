use super::*;

impl ApiKeyScope {
    pub fn as_str(&self) -> String {
        match self {
            Self::Read => "read".into(),
            Self::Write => "write".into(),
            Self::Delete => "delete".into(),
            Self::Admin => "admin".into(),
            Self::Bot(id) => format!("bot:{id}"),
            Self::Resource(r) => format!("resource:{r}"),
            Self::Custom(c) => format!("custom:{c}"),
        }
    }
}
impl std::str::FromStr for ApiKeyScope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "delete" => Ok(Self::Delete),
            "admin" => Ok(Self::Admin),
            s if s.starts_with("bot:") => {
                let id = s.strip_prefix("bot:").ok_or(())?;
                Uuid::parse_str(id).map_err(|_| ()).map(Self::Bot)
            }
            s if s.starts_with("resource:") => {
                let r = s.strip_prefix("resource:").ok_or(())?;
                Ok(Self::Resource(r.to_string()))
            }
            s if s.starts_with("custom:") => {
                let c = s.strip_prefix("custom:").ok_or(())?;
                Ok(Self::Custom(c.to_string()))
            }
            _ => Err(()),
        }
    }
}
impl ApiKeyScope {
    pub fn includes(&self, other: &Self) -> bool {
        if self == &Self::Admin {
            return true;
        }
        if self == other {
            return true;
        }
        if self == &Self::Write && other == &Self::Read {
            return true;
        }
        false
    }
}
