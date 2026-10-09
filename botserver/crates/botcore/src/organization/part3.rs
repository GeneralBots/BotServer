use super::*;

impl OrganizationMember {
    pub fn new(organization_id: Uuid, user_id: Uuid, role: &str) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            organization_id,
            user_id,
            role: role.to_string(),
            status: MemberStatus::Active,
            invited_by: None,
            invited_at: None,
            joined_at: Some(now),
            last_active_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn as_invited(mut self, invited_by: Uuid) -> Self {
        self.status = MemberStatus::Invited;
        self.invited_by = Some(invited_by);
        self.invited_at = Some(Utc::now());
        self.joined_at = None;
        self
    }

    pub fn accept_invitation(&mut self) {
        self.status = MemberStatus::Active;
        self.joined_at = Some(Utc::now());
        self.last_active_at = Some(Utc::now());
    }

    pub fn is_active(&self) -> bool {
        self.status == MemberStatus::Active
    }
}
