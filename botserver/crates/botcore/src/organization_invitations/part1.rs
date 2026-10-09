use super::*;

impl std::str::FromStr for InvitationRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "manager" => Ok(Self::Manager),
            "member" => Ok(Self::Member),
            "viewer" => Ok(Self::Viewer),
            "guest" => Ok(Self::Guest),
            _ => Err(()),
        }
    }
}
impl InvitationRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Manager => "manager",
            Self::Member => "member",
            Self::Viewer => "viewer",
            Self::Guest => "guest",
        }
    }
}
