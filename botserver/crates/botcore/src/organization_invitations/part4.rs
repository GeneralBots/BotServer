use super::*;

impl InvitationService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_invitation(
        &self,
        params: CreateInvitationParams<'_>,
    ) -> Result<OrganizationInvitation, String> {
        let email_lower = params.email.to_lowercase().trim().to_string();

        if !self.is_valid_email(&email_lower) {
            return Err("Invalid email address".to_string());
        }

        let existing = self
            .find_pending_invitation(&params.organization_id, &email_lower)
            .await;
        if existing.is_some() {
            return Err("An invitation already exists for this email".to_string());
        }

        let now = Utc::now();
        let invitation_id = Uuid::new_v4();
        let token = self.generate_secure_token();
        let groups_json =
            serde_json::to_string(&params.groups).unwrap_or_else(|_| "[]".to_string());

        // Owned copies for the `'static` spawn_blocking closure: the params
        // struct borrows `&str` from the caller, so nothing borrowed may cross
        // into the task. Everything the insert needs is materialized first.
        let org_id = params.organization_id;
        let role_str = params.role.as_str().to_string();
        let message_owned = params.message.clone();
        let invited_by = params.invited_by;
        let invited_by_name = params.invited_by_name.to_string();
        let invited_by_name_for_db = invited_by_name.clone();
        let email_for_db = email_lower.clone();
        let token_for_db = token.clone();
        let expires_at = now + Duration::days(params.expires_in_days);

        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
            sql_query(
                "INSERT INTO organization_invitations \
                 (id, org_id, email, role, status, message, invited_by, invited_by_name, \
                  token, groups, created_at, updated_at, expires_at) \
                 VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7, $8, $9::jsonb, $10, $10, $11)",
            )
            .bind::<diesel::sql_types::Uuid, _>(invitation_id)
            .bind::<diesel::sql_types::Uuid, _>(org_id)
            .bind::<diesel::sql_types::Text, _>(&email_for_db)
            .bind::<diesel::sql_types::Text, _>(&role_str)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(message_owned.as_deref())
            .bind::<diesel::sql_types::Uuid, _>(invited_by)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(Some(invited_by_name_for_db.as_str()))
            .bind::<diesel::sql_types::Text, _>(&token_for_db)
            .bind::<diesel::sql_types::Text, _>(&groups_json)
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .bind::<diesel::sql_types::Timestamptz, _>(expires_at)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to persist invitation: {e}"))?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))??;

        let invitation = OrganizationInvitation {
            id: invitation_id,
            organization_id: org_id,
            email: email_lower,
            role: params.role,
            groups: params.groups,
            invited_by,
            invited_by_name,
            status: InvitationStatus::Pending,
            token,
            message: params.message,
            expires_at,
            created_at: now,
            updated_at: now,
            accepted_at: None,
            accepted_by: None,
        };

        self.send_invitation_email(&invitation, params.organization_name)
            .await;

        Ok(invitation)
    }

    pub async fn bulk_invite(&self, params: BulkInviteParams<'_>) -> BulkInviteResponse {
        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for email in params.emails {
            match self
                .create_invitation(CreateInvitationParams {
                    organization_id: params.organization_id,
                    organization_name: params.organization_name,
                    email: &email,
                    role: params.role.clone(),
                    groups: params.groups.clone(),
                    invited_by: params.invited_by,
                    invited_by_name: params.invited_by_name,
                    message: params.message.clone(),
                    expires_in_days: 7,
                })
                .await
            {
                Ok(invitation) => {
                    successful.push(self.to_response(&invitation, params.organization_name));
                }
                Err(error) => {
                    failed.push(BulkInviteError { email, error });
                }
            }
        }

        BulkInviteResponse { successful, failed }
    }

    pub async fn accept_invitation(
        &self,
        token: &str,
        user_id: Uuid,
    ) -> Result<AcceptInvitationResponse, String> {
        let invitation = self.get_invitation_by_token(token).await.ok_or("Invalid invitation token")?;

        if invitation.status != InvitationStatus::Pending {
            return Err(format!(
                "Invitation is not pending (status: {:?})",
                invitation.status
            ));
        }

        if invitation.expires_at < Utc::now() {
            self.update_status(&invitation.id, InvitationStatus::Expired, None, None)
                .await?;
            return Err("Invitation has expired".to_string());
        }

        let now = Utc::now();
        let pool = self.pool.clone();
        let org_id = invitation.organization_id;
        let role = invitation.role.as_str().to_string();
        let role_for_db = role.clone();
        let invitation_id = invitation.id;
        tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
            sql_query(
                "UPDATE organization_invitations \
                 SET status = 'accepted', accepted_at = $1, accepted_by = $2, updated_at = $1 \
                 WHERE id = $3",
            )
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Uuid, _>(invitation_id)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to update invitation: {e}"))?;

            // Bind the user to the org so the suite scopes them correctly
            // (this is the missing collaborative glue — before this, accept
            // only flipped an in-memory flag and the user never joined).
            let binding_id = Uuid::new_v4();
            sql_query(
                "INSERT INTO user_organizations (id, user_id, org_id, role, is_default, joined_at) \
                 VALUES ($1, $2, $3, $4, false, $5) \
                 ON CONFLICT (user_id, org_id) DO NOTHING",
            )
            .bind::<diesel::sql_types::Uuid, _>(binding_id)
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Uuid, _>(org_id)
            .bind::<diesel::sql_types::Text, _>(&role_for_db)
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to bind user to organization: {e}"))?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))??;

        Ok(AcceptInvitationResponse {
            success: true,
            organization_id: org_id,
            organization_name: "Organization".to_string(),
            role,
            message: "Successfully joined the organization".to_string(),
        })
    }

    async fn update_status(
        &self,
        invitation_id: &Uuid,
        status: InvitationStatus,
        accepted_at: Option<DateTime<Utc>>,
        accepted_by: Option<Uuid>,
    ) -> Result<(), String> {
        let now = Utc::now();
        let pool = self.pool.clone();
        let id = *invitation_id;
        let status_str = match status {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Expired => "expired",
            InvitationStatus::Revoked => "revoked",
        }
        .to_string();
        tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
            sql_query(
                "UPDATE organization_invitations \
                 SET status = $1, accepted_at = $2, accepted_by = $3, updated_at = $4 \
                 WHERE id = $5",
            )
            .bind::<diesel::sql_types::Text, _>(&status_str)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(accepted_at)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(accepted_by)
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to update invitation status: {e}"))?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))?
    }

    pub async fn decline_invitation(&self, token: &str) -> Result<(), String> {
        let invitation = self.get_invitation_by_token(token).await.ok_or("Invalid invitation token")?;
        if invitation.status != InvitationStatus::Pending {
            return Err("Only pending invitations can be declined".to_string());
        }
        self.update_status(&invitation.id, InvitationStatus::Declined, None, None)
            .await
    }

    pub async fn revoke_invitation(&self, invitation_id: Uuid) -> Result<(), String> {
        let invitation = self.get_invitation(invitation_id).await.ok_or("Invitation not found")?;
        if invitation.status != InvitationStatus::Pending {
            return Err("Only pending invitations can be revoked".to_string());
        }
        self.update_status(&invitation_id, InvitationStatus::Revoked, None, None)
            .await
    }
}
