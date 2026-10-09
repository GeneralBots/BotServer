use super::*;

impl std::fmt::Display for FaceApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigurationError(e) => write!(f, "Configuration error: {e}"),
            Self::NetworkError(e) => write!(f, "Network error: {e}"),
            Self::ApiError(e) => write!(f, "API error: {e}"),
            Self::ParseError(e) => write!(f, "Parse error: {e}"),
            Self::FaceNotFound(id) => write!(f, "Face not found: {id}"),
            Self::MissingEmbedding(e) => write!(f, "Missing embedding: {e}"),
            Self::AttributesNotAvailable => write!(f, "Face attributes not available"),
            Self::ProviderNotImplemented(p) => write!(f, "Provider not implemented: {p}"),
        }
    }
}
impl std::error::Error for FaceApiError {}
pub fn create_azure_config(endpoint: &str, api_key: &str) -> FaceApiConfig {
    FaceApiConfig {
        provider: FaceApiProvider::AzureFaceApi,
        endpoint: Some(endpoint.to_string()),
        api_key: Some(api_key.to_string()),
        region: None,
        model_path: None,
    }
}
pub fn create_aws_config(region: &str) -> FaceApiConfig {
    FaceApiConfig {
        provider: FaceApiProvider::AwsRekognition,
        endpoint: None,
        api_key: None,
        region: Some(region.to_string()),
        model_path: None,
    }
}
pub fn create_opencv_config(model_path: &str) -> FaceApiConfig {
    FaceApiConfig {
        provider: FaceApiProvider::OpenCv,
        endpoint: None,
        api_key: None,
        region: None,
        model_path: Some(model_path.to_string()),
    }
}
pub fn create_insightface_config(model_path: &str) -> FaceApiConfig {
    FaceApiConfig {
        provider: FaceApiProvider::InsightFace,
        endpoint: None,
        api_key: None,
        region: None,
        model_path: Some(model_path.to_string()),
    }
}
