mod qdrant_native;
pub mod bm25_config;
pub mod drive_vectordb;
pub mod embedding;
pub mod hybrid_search;
pub mod vectordb_indexer;

pub use qdrant_native::*;
pub use drive_vectordb::{FileDocument, FileContentExtractor, UserDriveVectorDB};

#[cfg(feature = "mail")]
pub use vectordb_indexer::{UserEmailVectorDB, EmailDocument, EmailSearchQuery, EmailSearchResult};

mod re_cache;
