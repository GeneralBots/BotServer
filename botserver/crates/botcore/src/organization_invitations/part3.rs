use super::*;

impl<'a> Default for CreateInvitationParams<'a> {
    fn default() -> Self {
        Self {
            organization_id: Uuid::default(),
            organization_name: "",
            email: "",
            role: InvitationRole::Member,
            groups: Vec::new(),
            invited_by: Uuid::default(),
            invited_by_name: "",
            message: None,
            expires_in_days: 7,
        }
    }
}
