use super::*;

impl ToSql<SmallInt, Pg> for MessageType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for MessageType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::Text),
            1 => Ok(Self::Image),
            2 => Ok(Self::Audio),
            3 => Ok(Self::Video),
            4 => Ok(Self::Document),
            5 => Ok(Self::Location),
            6 => Ok(Self::Contact),
            7 => Ok(Self::Sticker),
            8 => Ok(Self::Reaction),
            _ => Err(format!("Unknown MessageType: {}", value).into()),
        }
    }
}
impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Image => write!(f, "image"),
            Self::Audio => write!(f, "audio"),
            Self::Video => write!(f, "video"),
            Self::Document => write!(f, "document"),
            Self::Location => write!(f, "location"),
            Self::Contact => write!(f, "contact"),
            Self::Sticker => write!(f, "sticker"),
            Self::Reaction => write!(f, "reaction"),
        }
    }
}

// ============================================================================
// LLM PROVIDER
// ============================================================================
