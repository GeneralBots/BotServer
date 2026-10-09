use super::*;

impl RedditChannel {
    pub async fn is_authenticated(&self) -> bool {
        let tokens = self.tokens.read().await;
        tokens.is_some()
    }
}
