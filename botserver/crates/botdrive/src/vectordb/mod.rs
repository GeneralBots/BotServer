pub(crate) use anyhow::Result;
#[cfg(feature = "sheet")]
pub(crate) use calamine::Reader;
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::path::{Path, PathBuf};
#[cfg(feature = "vectordb")]
pub(crate) use std::sync::Arc;
pub(crate) use tokio::fs;
pub(crate) use uuid::Uuid;
#[cfg(feature = "drive")]
pub(crate) use pdf_extract;
#[cfg(feature = "vectordb")]
pub(crate) use botqdrant::{Distance, PointStruct, Qdrant, VectorParams};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDocument {
pub id: String,
pub file_path: String,
pub file_name: String,
pub file_type: String,
pub file_size: u64,
pub bucket: String,
pub content_text: String,
pub content_summary: Option<String>,
pub created_at: DateTime<Utc>,
pub modified_at: DateTime<Utc>,
pub indexed_at: DateTime<Utc>,
pub mime_type: Option<String>,
pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchQuery {
pub query_text: String,
pub bucket: Option<String>,
pub file_type: Option<String>,
pub date_from: Option<DateTime<Utc>>,
pub date_to: Option<DateTime<Utc>>,
pub tags: Vec<String>,
pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSearchResult {
pub file: FileDocument,
pub score: f32,
pub snippet: String,
pub highlights: Vec<String>,
}

pub struct UserDriveVectorDB {
user_id: Uuid,
bot_id: Uuid,
collection_name: String,
db_path: PathBuf,
#[cfg(feature = "vectordb")]
client: Option<Arc<Qdrant>>,
}

#[derive(Debug)]
pub struct FileContentExtractor;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
