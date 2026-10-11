

/// Builds the Vertex AI OpenAI-compatible endpoint URL
pub fn build_vertex_url(project: &str, location: &str) -> String {
    format!(
        "https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/endpoints/openapi"
    )
}
/// Check if a URL is a Vertex AI endpoint
pub fn is_vertex_ai_url(url: &str) -> bool {
    url.contains("aiplatform.googleapis.com") || url.contains("generativelanguage.googleapis.com")
}
