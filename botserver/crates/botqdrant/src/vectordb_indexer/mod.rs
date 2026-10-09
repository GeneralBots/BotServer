pub(crate) use anyhow::Result;
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use log::{info, warn};
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tokio::time::{sleep, Duration};
pub(crate) use uuid::Uuid;

#[cfg(feature = "mail")]
mod email_types {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::qdrant_native::QdrantClient;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EmailDocument {
        pub id: String,
        pub account_id: String,
        pub from_email: String,
        pub from_name: String,
        pub to_email: String,
        pub subject: String,
        pub body_text: String,
        pub date: DateTime<Utc>,
        pub folder: String,
        pub has_attachments: bool,
        pub thread_id: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EmailSearchQuery {
        pub query_text: String,
        pub from: Option<String>,
        pub folder: Option<String>,
        pub date_from: Option<DateTime<Utc>>,
        pub date_to: Option<DateTime<Utc>>,
        pub limit: usize,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EmailSearchResult {
        pub email: EmailDocument,
        pub score: f32,
        pub snippet: String,
    }

    pub struct UserEmailVectorDB {
        user_id: Uuid,
        bot_id: Uuid,
        collection_name: String,
        client: Option<Arc<QdrantClient>>,
    }

    impl UserEmailVectorDB {
        pub fn new(user_id: Uuid, bot_id: Uuid) -> Self {
            let collection_name = format!("email_{}_{}", bot_id, user_id);
            Self { user_id, bot_id, collection_name, client: None }
        }

        pub fn user_id(&self) -> Uuid { self.user_id }
        pub fn bot_id(&self) -> Uuid { self.bot_id }
        pub fn collection_name(&self) -> &str { &self.collection_name }

        pub async fn initialize(&mut self, qdrant_url: &str) -> anyhow::Result<()> {
            let client = QdrantClient::from_url(qdrant_url).build()?;
            let collections = client.list_collections().await?;
            let exists = collections.collections.iter().any(|c| c.name == self.collection_name);
            if !exists {
                client.create_collection(&self.collection_name, 1536, "Cosine").await?;
                log::info!("Created email collection: {}", self.collection_name);
            }
            self.client = Some(Arc::new(client));
            Ok(())
        }

        pub async fn index_email(&self, email: &EmailDocument, embedding: Vec<f32>) -> anyhow::Result<()> {
            if let Some(client) = self.client.as_ref() {
                let point = serde_json::json!({
                    "id": email.id,
                    "vector": embedding,
                    "payload": serde_json::to_value(email)?
                });
                client.upsert_points(&self.collection_name, vec![point]).await?;
                log::debug!("Indexed email: {} - {}", email.id, email.subject);
            }
            Ok(())
        }

        pub async fn index_emails_batch(&self, emails: &[(EmailDocument, Vec<f32>)]) -> anyhow::Result<()> {
            for (email, embedding) in emails {
                self.index_email(email, embedding.clone()).await?;
            }
            Ok(())
        }

        pub async fn search(&self, query: &EmailSearchQuery, query_embedding: Vec<f32>) -> anyhow::Result<Vec<EmailSearchResult>> {
            if let Some(client) = self.client.as_ref() {
                let results = client.search_points(&self.collection_name, &query_embedding, query.limit, None).await?;
                let mut search_results = Vec::new();
                for point in results {
                    let payload = point.get("payload").and_then(|p| p.as_object()).cloned().unwrap_or_default();
                    let get_str = |key: &str| -> String {
                        payload.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_default()
                    };
                    let email = EmailDocument {
                        id: get_str("id"),
                        account_id: get_str("account_id"),
                        from_email: get_str("from_email"),
                        from_name: get_str("from_name"),
                        to_email: get_str("to_email"),
                        subject: get_str("subject"),
                        body_text: get_str("body_text"),
                        date: Utc::now(),
                        folder: get_str("folder"),
                        has_attachments: false,
                        thread_id: payload.get("thread_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    };
                    let score = point.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let body_preview: String = email.body_text.chars().take(200).collect();
                    search_results.push(EmailSearchResult { email, score, snippet: body_preview });
                }
                Ok(search_results)
            } else {
                Err(anyhow::anyhow!("Qdrant client not initialized"))
            }
        }

        pub async fn delete_email(&self, email_id: &str) -> anyhow::Result<()> {
            if let Some(client) = self.client.as_ref() {
                client.delete_points(&self.collection_name, vec![email_id.to_string()]).await?;
            }
            Ok(())
        }
    }
}

#[cfg(feature = "mail")]
use email_types::{EmailDocument, EmailSearchQuery, EmailSearchResult, UserEmailVectorDB};

use crate::embedding::EmbeddingGenerator;
use crate::drive_vectordb::{FileDocument, FileContentExtractor, UserDriveVectorDB};
use botcore::shared::utils::DbPool;

#[derive(Debug, Clone)]
struct UserWorkspace {
    root: PathBuf,
    bot_id: Uuid,
    user_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexingStatus {
    Idle,
    Running,
    Paused,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct IndexingStats {
    pub emails_indexed: u64,
    pub files_indexed: u64,
    pub emails_pending: u64,
    pub files_pending: u64,
    pub last_run: Option<DateTime<Utc>>,
    pub errors: u64,
}

struct UserIndexingJob {
    user_id: Uuid,
    bot_id: Uuid,
    workspace: UserWorkspace,
    #[cfg(feature = "mail")]
    email_db: Option<UserEmailVectorDB>,
    drive_db: Option<UserDriveVectorDB>,
    stats: IndexingStats,
    status: IndexingStatus,
}

pub struct VectorDBIndexer {
    db_pool: DbPool,
    work_root: PathBuf,
    qdrant_url: String,
    embedding_generator: Arc<EmbeddingGenerator>,
    jobs: Arc<RwLock<HashMap<Uuid, UserIndexingJob>>>,
    running: Arc<RwLock<bool>>,
    interval_seconds: u64,
    batch_size: usize,
}

mod part1;
mod part2;
mod part3;
