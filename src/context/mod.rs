use async_trait::async_trait;
use langchain_rust::{
    embedding::openai::openai_embedder::OpenAiEmbedder,
    vectorstore::qdrant::{Qdrant, StoreBuilder},
    vectorstore::{VectorStore, VecStoreOptions},
    schemas::Document,
};
use qdrant_client::qdrant::Qdrant as QdrantClient;
use sqlx::PgPool;
use uuid::Uuid;

#[async_trait]
pub trait ContextProvider: Send + Sync {
    async fn get_context(&self, session_id: Uuid, user_id: Uuid, query: &str) -> Result<String, Box<dyn std::error::Error>>;
    async fn store_embedding(&self, text: &str, embedding: Vec<f32>, metadata: Value) -> Result<(), Box<dyn std::error::Error>>;
    async fn search_similar(&self, embedding: Vec<f32>, limit: u32) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>>;
}

pub struct LangChainContextProvider {
    pool: PgPool,
    vector_store: Qdrant,
    embedder: OpenAiEmbedder,
}

impl LangChainContextProvider {
    pub async fn new(pool: PgPool, qdrant_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let embedder = OpenAiEmbedder::default();

        let client = QdrantClient::from_url(qdrant_url).build()?;
        let vector_store = StoreBuilder::new()
            .embedder(embedder.clone())
            .client(client)
            .collection_name("conversations")
            .build()
            .await?;

        Ok(Self {
            pool,
            vector_store,
            embedder,
        })
    }
}

#[async_trait]
impl ContextProvider for LangChainContextProvider {
    async fn get_context(&self, session_id: Uuid, user_id: Uuid, query: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Get conversation history
        let history = sqlx::query(
            "SELECT role, content_encrypted FROM message_history
             WHERE session_id = $1 AND user_id = $2
             ORDER BY message_index DESC LIMIT 5"
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut context = String::from("Conversation history:\n");
        for row in history.iter().rev() {
            let role: String = row.get("role");
            let content: String = row.get("content_encrypted");
            context.push_str(&format!("{}: {}\n", role, content));
        }

        // Search for similar documents using LangChain
        let similar_docs = self.vector_store
            .similarity_search(query, 3, &VecStoreOptions::default())
            .await?;

        if !similar_docs.is_empty() {
            context.push_str("\nRelevant context:\n");
            for doc in similar_docs {
                context.push_str(&format!("- {}\n", doc.page_content));
            }
        }

        context.push_str(&format!("\nCurrent message: {}", query));
        Ok(context)
    }

    async fn store_embedding(&self, text: &str, embedding: Vec<f32>, metadata: Value) -> Result<(), Box<dyn std::error::Error>> {
        let document = Document::new(text).with_metadata(metadata);

        self.vector_store
            .add_documents(&[document], &VecStoreOptions::default())
            .await?;

        Ok(())
    }

    async fn search_similar(&self, embedding: Vec<f32>, limit: u32) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        // LangChain handles this through the vector store interface
        // This method would need adaptation to work with LangChain's search patterns
        Ok(vec![])
    }
}
