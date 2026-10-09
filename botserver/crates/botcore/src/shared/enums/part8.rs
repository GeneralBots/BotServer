use super::*;

impl ToSql<SmallInt, Pg> for ExecutionMode {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for ExecutionMode {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::Manual),
            1 => Ok(Self::Supervised),
            2 => Ok(Self::Autonomous),
            _ => Err(format!("Unknown ExecutionMode: {}", value).into()),
        }
    }
}
impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manual => write!(f, "manual"),
            Self::Supervised => write!(f, "supervised"),
            Self::Autonomous => write!(f, "autonomous"),
        }
    }
}

// ============================================================================
// RISK LEVEL
// ============================================================================
