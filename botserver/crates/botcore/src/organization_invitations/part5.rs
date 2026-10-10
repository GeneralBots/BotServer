use super::*;

impl InvitationService {
    pub async fn resend_invitation(
        &self,
        invitation_id: Uuid,
        organization_name: &str,
        extend_expiry: bool,
    ) -> Result<OrganizationInvitation, String> {
        let mut invitation = self.get_invitation(invitation_id).await.ok_or("Invitation not found")?;

        if invitation.status != InvitationStatus::Pending
            && invitation.status != InvitationStatus::Expired
        {
            return Err("Only pending or expired invitations can be resent".to_string());
        }

        let now = Utc::now();

        if extend_expiry || invitation.expires_at < now {
            invitation.expires_at = now + Duration::days(7);
        }
        invitation.status = InvitationStatus::Pending;
        invitation.updated_at = now;

        // Persist the refreshed token/status/expiry.
        let pool = self.pool.clone();
        let id = invitation.id;
        let status_str = "pending";
        let expires = invitation.expires_at;
        tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
            sql_query(
                "UPDATE organization_invitations \
                 SET status = $1, expires_at = $2, updated_at = $3 WHERE id = $4",
            )
            .bind::<diesel::sql_types::Text, _>(status_str)
            .bind::<diesel::sql_types::Timestamptz, _>(expires)
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to update invitation: {e}"))?;
            Ok::<_, String>(())
        })
        .await
        .map_err(|e| format!("Task join error: {e}"))??;

        let invitation_clone = invitation.clone();
        self.send_invitation_email(&invitation_clone, organization_name)
            .await;

        Ok(invitation_clone)
    }

    pub async fn list_invitations(
        &self,
        organization_id: Uuid,
        status_filter: Option<InvitationStatus>,
        page: u32,
        per_page: u32,
    ) -> InvitationListResponse {
        let status_filter_str = status_filter.map(|s| match s {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Expired => "expired",
            InvitationStatus::Revoked => "revoked",
        }.to_string());

        let pool = self.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().map_err(|e| format!("DB pool error: {e}"))?;
            let mut sql = String::from(
                "SELECT id, org_id, email, role, status, message, invited_by, invited_by_name, \
                        token, groups, created_at, updated_at, expires_at, accepted_at, accepted_by \
                 FROM organization_invitations WHERE org_id = $1",
            );
            if status_filter_str.is_some() {
                sql.push_str(" AND status = $2");
            }
            sql.push_str(" ORDER BY created_at DESC LIMIT 500");
            // Diesel's `bind` changes the query type on every call, so the
            // filtered and unfiltered shapes must be built as separate queries
            // rather than reassigning a single `let mut` query.
            let rows = match status_filter_str.as_deref() {
                Some(s) => sql_query(&sql)
                    .bind::<diesel::sql_types::Uuid, _>(organization_id)
                    .bind::<diesel::sql_types::Text, _>(s)
                    .load::<InvitationRow>(&mut conn),
                None => sql_query(&sql)
                    .bind::<diesel::sql_types::Uuid, _>(organization_id)
                    .load::<InvitationRow>(&mut conn),
            };
            rows.map_err(|e| format!("Failed to list invitations: {e}"))
        })
        .await
        .map_err(|e| format!("Task join error: {e}"));

        let rows = match result {
            Ok(Ok(rows)) => rows,
            _ => Vec::new(),
        };

        let all: Vec<InvitationResponse> = rows
            .iter()
            .map(|r| self.to_response(&r.to_invitation(), "Organization"))
            .collect();

        let total = all.len() as u32;
        let total_pages = total.div_ceil(per_page.max(1));
        let start = ((page.saturating_sub(1)) * per_page.max(1)) as usize;
        let end = (start + per_page as usize).min(all.len());

        InvitationListResponse {
            invitations: all[start..end.min(all.len())].to_vec(),
            total,
            page,
            per_page,
            total_pages,
        }
    }

    pub async fn get_invitation(&self, invitation_id: Uuid) -> Option<OrganizationInvitation> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().ok()?;
            sql_query(
                "SELECT id, org_id, email, role, status, message, invited_by, invited_by_name, \
                        token, groups, created_at, updated_at, expires_at, accepted_at, accepted_by \
                 FROM organization_invitations WHERE id = $1 LIMIT 1",
            )
            .bind::<diesel::sql_types::Uuid, _>(invitation_id)
            .get_result::<InvitationRow>(&mut conn)
            .ok()
            .map(|r| r.to_invitation())
        })
        .await
        .ok()
        .flatten()
    }

    pub async fn get_invitation_by_token(&self, token: &str) -> Option<OrganizationInvitation> {
        let pool = self.pool.clone();
        let token_owned = token.to_string();
        tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().ok()?;
            sql_query(
                "SELECT id, org_id, email, role, status, message, invited_by, invited_by_name, \
                        token, groups, created_at, updated_at, expires_at, accepted_at, accepted_by \
                 FROM organization_invitations WHERE token = $1 LIMIT 1",
            )
            .bind::<diesel::sql_types::Text, _>(&token_owned)
            .get_result::<InvitationRow>(&mut conn)
            .ok()
            .map(|r| r.to_invitation())
        })
        .await
        .ok()
        .flatten()
    }

    pub async fn cleanup_expired_invitations(&self) {
        let pool = self.pool.clone();
        let _ = tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().ok()?;
            sql_query(
                "UPDATE organization_invitations \
                 SET status = 'expired', updated_at = $1 \
                 WHERE status = 'pending' AND expires_at < $1",
            )
            .bind::<diesel::sql_types::Timestamptz, _>(Utc::now())
            .execute(&mut conn)
            .ok()
        })
        .await;
    }

    pub(crate) async fn find_pending_invitation(
        &self,
        organization_id: &Uuid,
        email: &str,
    ) -> Option<OrganizationInvitation> {
        let pool = self.pool.clone();
        let org = *organization_id;
        let email_owned = email.to_string();
        tokio::task::spawn_blocking(move || {
            use diesel::sql_query;
            use diesel::RunQueryDsl;
            let mut conn = pool.get().ok()?;
            sql_query(
                "SELECT id, org_id, email, role, status, message, invited_by, invited_by_name, \
                        token, groups, created_at, updated_at, expires_at, accepted_at, accepted_by \
                 FROM organization_invitations \
                 WHERE org_id = $1 AND email = $2 AND status = 'pending' LIMIT 1",
            )
            .bind::<diesel::sql_types::Uuid, _>(org)
            .bind::<diesel::sql_types::Text, _>(&email_owned)
            .get_result::<InvitationRow>(&mut conn)
            .ok()
            .map(|r| r.to_invitation())
        })
        .await
        .ok()
        .flatten()
    }

    pub(super) fn to_response(
        &self,
        invitation: &OrganizationInvitation,
        org_name: &str,
    ) -> InvitationResponse {
        let now = Utc::now();
        InvitationResponse {
            id: invitation.id,
            organization_id: invitation.organization_id,
            organization_name: org_name.to_string(),
            email: invitation.email.clone(),
            role: invitation.role.as_str().to_string(),
            groups: invitation.groups.clone(),
            invited_by_name: invitation.invited_by_name.clone(),
            status: format!("{:?}", invitation.status).to_lowercase(),
            message: invitation.message.clone(),
            expires_at: invitation.expires_at,
            created_at: invitation.created_at,
            is_expired: invitation.expires_at < now,
        }
    }

    pub(super) fn generate_secure_token(&self) -> String {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        let mut token = String::with_capacity(64);
        let hasher_builder = RandomState::new();

        for _ in 0..4 {
            let mut hasher = hasher_builder.build_hasher();
            hasher.write_u128(Uuid::new_v4().as_u128());
            hasher.write_i64(Utc::now().timestamp_nanos_opt().unwrap_or(0));
            token.push_str(&format!("{:016x}", hasher.finish()));
        }

        token
    }

    pub(super) fn is_valid_email(&self, email: &str) -> bool {
        let email = email.trim();

        if email.is_empty() || email.len() > 254 {
            return false;
        }

        let at_pos = match email.find('@') {
            Some(pos) => pos,
            None => return false,
        };

        let local = &email[..at_pos];
        let domain = &email[at_pos + 1..];

        if local.is_empty() || local.len() > 64 {
            return false;
        }

        if domain.is_empty() || !domain.contains('.') {
            return false;
        }

        let domain_parts: Vec<&str> = domain.split('.').collect();
        if domain_parts.iter().any(|p| p.is_empty()) {
            return false;
        }

        true
    }
}
