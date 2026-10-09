use super::*;

impl ToSql<SmallInt, Pg> for IntentType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for IntentType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::AppCreate),
            2 => Ok(Self::Todo),
            3 => Ok(Self::Monitor),
            4 => Ok(Self::Action),
            5 => Ok(Self::Schedule),
            6 => Ok(Self::Goal),
            7 => Ok(Self::Tool),
            8 => Ok(Self::Query),
            _ => Err(format!("Unknown IntentType: {}", value).into()),
        }
    }
}
impl std::fmt::Display for IntentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "UNKNOWN"),
            Self::AppCreate => write!(f, "APP_CREATE"),
            Self::Todo => write!(f, "TODO"),
            Self::Monitor => write!(f, "MONITOR"),
            Self::Action => write!(f, "ACTION"),
            Self::Schedule => write!(f, "SCHEDULE"),
            Self::Goal => write!(f, "GOAL"),
            Self::Tool => write!(f, "TOOL"),
            Self::Query => write!(f, "QUERY"),
        }
    }
}
impl std::str::FromStr for IntentType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "UNKNOWN" => Ok(Self::Unknown),
            "APP_CREATE" | "APPCREATE" | "APP" | "APPLICATION" | "CREATE_APP" => Ok(Self::AppCreate),
            "TODO" | "TASK" | "REMINDER" => Ok(Self::Todo),
            "MONITOR" | "WATCH" | "ALERT" | "ON_CHANGE" => Ok(Self::Monitor),
            "ACTION" | "DO" | "EXECUTE" | "RUN" => Ok(Self::Action),
            "SCHEDULE" | "SCHEDULED" | "CRON" | "TIMER" => Ok(Self::Schedule),
            "GOAL" | "OBJECTIVE" | "TARGET" => Ok(Self::Goal),
            "TOOL" | "FUNCTION" | "UTILITY" => Ok(Self::Tool),
            "QUERY" | "SEARCH" | "FIND" | "LOOKUP" => Ok(Self::Query),
            _ => Ok(Self::Unknown),
        }
    }
}
