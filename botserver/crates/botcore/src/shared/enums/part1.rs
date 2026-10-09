use super::*;

impl ToSql<SmallInt, Pg> for ChannelType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for ChannelType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::Web),
            1 => Ok(Self::WhatsApp),
            2 => Ok(Self::Telegram),
            3 => Ok(Self::MsTeams),
            4 => Ok(Self::Slack),
            5 => Ok(Self::Email),
            6 => Ok(Self::Sms),
            7 => Ok(Self::Voice),
            8 => Ok(Self::Instagram),
            9 => Ok(Self::Api),
            _ => Err(format!("Unknown ChannelType: {}", value).into()),
        }
    }
}
impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Web => write!(f, "web"),
            Self::WhatsApp => write!(f, "whatsapp"),
            Self::Telegram => write!(f, "telegram"),
            Self::MsTeams => write!(f, "msteams"),
            Self::Slack => write!(f, "slack"),
            Self::Email => write!(f, "email"),
            Self::Sms => write!(f, "sms"),
            Self::Voice => write!(f, "voice"),
            Self::Instagram => write!(f, "instagram"),
            Self::Api => write!(f, "api"),
        }
    }
}
impl std::str::FromStr for ChannelType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web" => Ok(Self::Web),
            "whatsapp" => Ok(Self::WhatsApp),
            "telegram" => Ok(Self::Telegram),
            "msteams" | "ms_teams" | "teams" => Ok(Self::MsTeams),
            "slack" => Ok(Self::Slack),
            "email" => Ok(Self::Email),
            "sms" => Ok(Self::Sms),
            "voice" => Ok(Self::Voice),
            "instagram" => Ok(Self::Instagram),
            "api" => Ok(Self::Api),
            _ => Err(format!("Unknown channel type: {}", s)),
        }
    }
}

// ============================================================================
// MESSAGE ROLE
// ============================================================================
