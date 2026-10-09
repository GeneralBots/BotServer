use super::*;

impl AccessCheckResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Allowed => None,
            Self::Denied(r) => Some(r),
            Self::RequiresElevation(r) => Some(r),
        }
    }
}

// ============================================================================
// Organization Service
// ============================================================================
