use super::*;

impl std::fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageBackend::Local => write!(f, "local"),
            StorageBackend::S3 => write!(f, "s3"),
            StorageBackend::Azure => write!(f, "azure"),
            StorageBackend::Gcs => write!(f, "gcs"),
        }
    }
}
