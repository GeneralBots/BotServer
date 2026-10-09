use super::*;

impl ToSql<SmallInt, Pg> for ApprovalStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for ApprovalStatus {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::Pending),
            1 => Ok(Self::Approved),
            2 => Ok(Self::Rejected),
            3 => Ok(Self::Expired),
            4 => Ok(Self::Skipped),
            _ => Err(format!("Unknown ApprovalStatus: {}", value).into()),
        }
    }
}
impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::Expired => write!(f, "expired"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

// ============================================================================
// APPROVAL DECISION
// ============================================================================
