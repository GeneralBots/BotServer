use super::*;

impl InvitationRow {
    pub(super) fn to_invitation(&self) -> OrganizationInvitation {
        OrganizationInvitation {
            id: self.id,
            organization_id: self.org_id,
            email: self.email.clone(),
            role: self.role.parse().unwrap_or(InvitationRole::Member),
            groups: self
                .groups
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            invited_by: self.invited_by,
            invited_by_name: self.invited_by_name.clone().unwrap_or_default(),
            status: match self.status.as_str() {
                "accepted" => InvitationStatus::Accepted,
                "declined" => InvitationStatus::Declined,
                "expired" => InvitationStatus::Expired,
                "revoked" => InvitationStatus::Revoked,
                _ => InvitationStatus::Pending,
            },
            token: self.token.clone().unwrap_or_default(),
            message: self.message.clone(),
            expires_at: self.expires_at.unwrap_or_else(Utc::now),
            created_at: self.created_at,
            updated_at: self.updated_at.unwrap_or(self.created_at),
            accepted_at: self.accepted_at,
            accepted_by: self.accepted_by,
        }
    }
}
