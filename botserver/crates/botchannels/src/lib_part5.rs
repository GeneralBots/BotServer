use super::*;

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            providers: HashMap::new(),
        }
    }

    pub fn register_provider(&mut self, provider: Arc<dyn ChannelProvider>) {
        self.providers.insert(provider.channel_type(), provider);
    }

    pub async fn add_account(&self, account: ChannelAccount) {
        let mut accounts = self.accounts.write().await;
        accounts.insert(account.name.clone(), account);
    }

    pub async fn get_account(&self, name: &str) -> Option<ChannelAccount> {
        let accounts = self.accounts.read().await;
        accounts.get(name).cloned()
    }

    pub async fn remove_account(&self, name: &str) -> Option<ChannelAccount> {
        let mut accounts = self.accounts.write().await;
        accounts.remove(name)
    }

    pub async fn list_accounts(&self) -> Vec<ChannelAccount> {
        let accounts = self.accounts.read().await;
        accounts.values().cloned().collect()
    }

    pub async fn post_to(
        &self,
        account_name: &str,
        content: &PostContent,
    ) -> Result<PostResult, ChannelError> {
        let account = self
            .get_account(account_name)
            .await
            .ok_or_else(|| ChannelError::AccountNotFound(account_name.to_string()))?;

        let provider = self
            .providers
            .get(&account.channel_type)
            .ok_or(ChannelError::NotConfigured)?;

        provider.post(&account, content).await
    }

    pub async fn post_to_multiple(
        &self,
        account_names: &[String],
        content: &PostContent,
    ) -> Vec<Result<PostResult, ChannelError>> {
        let mut results = Vec::with_capacity(account_names.len());

        for name in account_names {
            let result = self.post_to(name, content).await;
            results.push(result);
        }

        results
    }

    pub async fn post_to_channels(
        &self,
        channels: &[ChannelType],
        content: &PostContent,
    ) -> Vec<Result<PostResult, ChannelError>> {
        let accounts = self.accounts.read().await;
        let mut results = Vec::new();

        for channel_type in channels {
            let matching_accounts: Vec<_> = accounts
                .values()
                .filter(|a| &a.channel_type == channel_type && a.is_active)
                .collect();

            for account in matching_accounts {
                if let Some(provider) = self.providers.get(channel_type) {
                    let result = provider.post(account, content).await;
                    results.push(result);
                } else {
                    results.push(Err(ChannelError::NotConfigured));
                }
            }
        }

        results
    }

    pub fn get_channel_limits(&self, channel_type: &ChannelType) -> Option<ChannelLimits> {
        self.providers.get(channel_type).map(|p| ChannelLimits {
            max_text_length: p.max_text_length(),
            supports_images: p.supports_images(),
            supports_video: p.supports_video(),
            supports_links: p.supports_links(),
        })
    }
}
impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}
