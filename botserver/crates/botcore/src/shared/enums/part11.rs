use super::*;

impl ToSql<SmallInt, Pg> for ApprovalDecision {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for ApprovalDecision {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::Approve),
            1 => Ok(Self::Reject),
            2 => Ok(Self::Skip),
            _ => Err(format!("Unknown ApprovalDecision: {}", value).into()),
        }
    }
}
impl std::fmt::Display for ApprovalDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approve => write!(f, "approve"),
            Self::Reject => write!(f, "reject"),
            Self::Skip => write!(f, "skip"),
        }
    }
}

// ============================================================================
// INTENT TYPE
// ============================================================================
