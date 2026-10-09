use super::*;

impl UserRole {
    pub fn new(
        user_id: Uuid,
        organization_id: Uuid,
        role_id: Uuid,
        role_name: &str,
        assigned_by: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            organization_id,
            role_id,
            role_name: role_name.to_string(),
            assigned_by,
            assigned_at: Utc::now(),
            expires_at: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }
}

// ============================================================================
// Bot Access Control
// ============================================================================
