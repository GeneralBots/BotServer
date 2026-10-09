use super::*;

impl ToSql<SmallInt, Pg> for TaskStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let v = *self as i16;
        out.write_all(&v.to_be_bytes())?;
        Ok(serialize::IsNull::No)
    }
}
impl FromSql<SmallInt, Pg> for TaskStatus {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let value = <i16 as FromSql<SmallInt, Pg>>::from_sql(bytes)?;
        match value {
            0 => Ok(Self::Pending),
            1 => Ok(Self::Ready),
            2 => Ok(Self::Running),
            3 => Ok(Self::Paused),
            4 => Ok(Self::WaitingApproval),
            5 => Ok(Self::Completed),
            6 => Ok(Self::Failed),
            7 => Ok(Self::Cancelled),
            _ => Err(format!("Unknown TaskStatus: {}", value).into()),
        }
    }
}
impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Ready => write!(f, "ready"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::WaitingApproval => write!(f, "waiting_approval"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}
impl std::str::FromStr for TaskStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "ready" => Ok(Self::Ready),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "waiting_approval" | "waitingapproval" => Ok(Self::WaitingApproval),
            "completed" | "done" => Ok(Self::Completed),
            "failed" | "error" => Ok(Self::Failed),
            "cancelled" | "canceled" => Ok(Self::Cancelled),
            _ => Err(format!("Unknown task status: {}", s)),
        }
    }
}

// ============================================================================
// TASK PRIORITY
// ============================================================================
