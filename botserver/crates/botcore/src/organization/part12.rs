use super::*;

impl OrganizationService {
    pub fn new(db_pool: DbPool) -> Self {
        Self { _db_pool: db_pool }
    }

    /// Get a database connection from the pool
    pub(super) fn _get_conn(&self) -> Result<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>, OrganizationError> {
        self._db_pool.get().map_err(|e| {
            OrganizationError::DatabaseError(format!("Failed to get database connection: {}", e))
        })
    }

    /// Create a new organization with default roles and groups
    pub async fn create_organization(
        &self,
        name: String,
        owner_id: Uuid,
        plan_id: Option<String>,
    ) -> Result<OrganizationCreationResult, OrganizationError> {
        // Create organization
        let mut org = Organization::new(name, owner_id);
        if let Some(plan) = plan_id {
            org = org.with_plan(plan);
        }

        // Create default roles for org (custom roles in addition to system roles)
        let roles = default_roles();

        // Create default groups
        let groups = default_groups(org.id);

        // Create owner membership
        let owner_member = OrganizationMember::new(org.id, owner_id, "owner");

        // Assign owner role
        let owner_role = roles.iter().find(|r| r.name == "owner")
            .ok_or_else(|| OrganizationError::InvalidRole("Owner role not found in default roles".to_string()))?;
        let owner_role_assignment = UserRole::new(
            owner_id,
            org.id,
            owner_role.id,
            &owner_role.name,
            None,
        );

        Ok(OrganizationCreationResult {
            organization: org,
            roles,
            groups,
            owner_member,
            owner_role: owner_role_assignment,
        })
    }

    /// Invite a user to an organization
    pub fn create_invitation(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        role: &str,
        invited_by: Uuid,
    ) -> OrganizationMember {
        OrganizationMember::new(organization_id, user_id, role)
            .as_invited(invited_by)
    }

    /// Check if user has permission in organization
    pub fn check_permission(
        &self,
        user: &UserAccessContext,
        permission: &str,
    ) -> bool {
        user.has_permission(permission)
    }

    /// Get effective permissions for a user (from roles + groups)
    pub fn get_effective_permissions(
        &self,
        roles: &[Role],
        groups: &[Group],
    ) -> Vec<String> {
        let mut permissions = Vec::new();

        for role in roles {
            for perm in &role.permissions {
                if !permissions.contains(perm) {
                    permissions.push(perm.clone());
                }
            }
        }

        for group in groups {
            for perm in &group.permissions {
                if !permissions.contains(perm) {
                    permissions.push(perm.clone());
                }
            }
        }

        permissions
    }
}
