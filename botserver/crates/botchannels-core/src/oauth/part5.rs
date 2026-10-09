use super::*;

impl SocialOAuthService {
    pub async fn get_account_by_platform(
        &self,
        organization_id: Uuid,
        platform: SocialPlatform,
    ) -> Option<ConnectedAccount> {
        let accounts = self.accounts.read().await;
        accounts
            .values()
            .find(|a| a.organization_id == organization_id && a.platform == platform)
            .cloned()
    }

    pub async fn disconnect_account(&self, account_id: Uuid) -> Result<(), OAuthServiceError> {
        let mut accounts = self.accounts.write().await;
        accounts
            .remove(&account_id)
            .ok_or(OAuthServiceError::AccountNotFound)?;
        Ok(())
    }

    pub async fn cleanup_expired_states(&self) {
        let mut states = self.states.write().await;
        states.retain(|_, state| !state.is_expired());
    }
}
