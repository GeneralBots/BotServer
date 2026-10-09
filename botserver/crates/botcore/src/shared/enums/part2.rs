use super::*;

impl ToSql<SmallInt, Pg> for MessageRole {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for MessageRole {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            1 => Ok(Self::User),
            2 => Ok(Self::Assistant),
            3 => Ok(Self::System),
            4 => Ok(Self::Tool),
            9 => Ok(Self::Episodic),
            10 => Ok(Self::Compact),
            _ => Err(format!("Unknown MessageRole: {}", value).into()),
        }
    }
}
impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
            Self::Tool => write!(f, "tool"),
            Self::Episodic => write!(f, "episodic"),
            Self::Compact => write!(f, "compact"),
        }
    }
}
impl std::str::FromStr for MessageRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            "tool" => Ok(Self::Tool),
            "episodic" => Ok(Self::Episodic),
            "compact" => Ok(Self::Compact),
            _ => Err(format!("Unknown message role: {}", s)),
        }
    }
}

// ============================================================================
// MESSAGE TYPE
// ============================================================================
