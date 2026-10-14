use std::sync::Arc;

pub use botlib::db_pool::DbPool;

pub type S3PutFn = Arc<dyn Fn(&str, &str, Vec<u8>, Option<&str>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> + Send + Sync>;
pub type S3GetFn = Arc<dyn Fn(&str, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + Send>> + Send + Sync>;
pub type S3DeleteFn = Arc<dyn Fn(&str, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> + Send + Sync>;
pub type S3ListFn = Arc<dyn Fn(&str, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send>> + Send + Sync>;
pub type CallLlmFn = Arc<dyn Fn(&str, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct PaperState {
    pub conn: DbPool,
    pub bucket_name: String,
    pub s3_put: S3PutFn,
    pub s3_get: S3GetFn,
    pub s3_delete: S3DeleteFn,
    pub s3_list: S3ListFn,
    pub call_llm: CallLlmFn,
}

impl std::fmt::Debug for PaperState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaperState")
            .field("bucket_name", &self.bucket_name)
            .finish()
    }
}
