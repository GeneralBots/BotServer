use crate::kb::qdrant_client::{get_qdrant_client, QdrantPoint};
use crate::shared::state::AppState;
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

const CHUNK_SIZE: usize = 512; // Characters per chunk
const CHUNK_OVERLAP: usize = 50; // Overlap between chunks

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Generate embeddings using local LLM server
pub async fn generate_embeddings(
    texts: Vec<String>,
) -> Result<Vec<Vec<f32>>, Box<dyn Error + Send + Sync>> {
    let llm_url = std::env::var("LLM_URL").unwrap_or_else(|_| "http://localhost:8081".to_string());
    let url = format!("{}/v1/embeddings", llm_url);

    debug!("Generating embeddings for {} texts", texts.len());

    let client = Client::new();

    let request = EmbeddingRequest {
        input: texts,
        model: "text-embedding-ada-002".to_string(),
    };

    let response = client
        .post(&url)
        .json(&request)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        error!("Embedding generation failed: {}", error_text);
        return Err(format!("Embedding generation failed: {}", error_text).into());
    }

    let embedding_response: EmbeddingResponse = response.json().await?;

    let embeddings: Vec<Vec<f32>> = embedding_response
        .data
        .into_iter()
        .map(|d| d.embedding)
        .collect();

    debug!("Generated {} embeddings", embeddings.len());

    Ok(embeddings)
}

/// Split text into chunks with overlap
pub fn split_into_chunks(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

    if total_chars == 0 {
        return chunks;
    }

    let mut start = 0;

    while start < total_chars {
        let end = std::cmp::min(start + CHUNK_SIZE, total_chars);
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);

        if end >= total_chars {
            break;
        }

        // Move forward, but with overlap
        start += CHUNK_SIZE - CHUNK_OVERLAP;
    }

    debug!("Split text into {} chunks", chunks.len());

    chunks
}

/// Index a document by splitting it into chunks and storing embeddings
pub async fn index_document(
    state: &AppState,
    collection_name: &str,
    file_path: &str,
    content: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Indexing document: {}", file_path);

    // Split document into chunks
    let chunks = split_into_chunks(content);

    if chunks.is_empty() {
        info!("Document is empty, skipping: {}", file_path);
        return Ok(());
    }

    // Generate embeddings for all chunks
    let embeddings = generate_embeddings(chunks.clone()).await?;

    if embeddings.len() != chunks.len() {
        error!(
            "Embedding count mismatch: {} embeddings for {} chunks",
            embeddings.len(),
            chunks.len()
        );
        return Err("Embedding count mismatch".into());
    }

    // Create Qdrant points
    let mut points = Vec::new();

    for (idx, (chunk, embedding)) in chunks.iter().zip(embeddings.iter()).enumerate() {
        let point_id = format!("{}_{}", file_path.replace('/', "_"), idx);

        let payload = serde_json::json!({
            "file_path": file_path,
            "chunk_index": idx,
            "chunk_text": chunk,
            "total_chunks": chunks.len(),
        });

        points.push(QdrantPoint {
            id: point_id,
            vector: embedding.clone(),
            payload,
        });
    }

    // Upsert points to Qdrant
    let client = get_qdrant_client(state)?;
    client.upsert_points(collection_name, points).await?;

    info!(
        "Document indexed successfully: {} ({} chunks)",
        file_path,
        chunks.len()
    );

    Ok(())
}

/// Delete a document from the collection
pub async fn delete_document(
    state: &AppState,
    collection_name: &str,
    file_path: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Deleting document from index: {}", file_path);

    let client = get_qdrant_client(state)?;

    // Find all point IDs for this file path
    // Note: This is a simplified approach. In production, you'd want to search
    // by payload filter or maintain an index of point IDs per file.
    let prefix = file_path.replace('/', "_");

    // For now, we'll generate potential IDs based on common chunk counts
    let mut point_ids = Vec::new();
    for idx in 0..1000 {
        // Max 1000 chunks
        point_ids.push(format!("{}_{}", prefix, idx));
    }

    client.delete_points(collection_name, point_ids).await?;

    info!("Document deleted from index: {}", file_path);

    Ok(())
}

/// Search for similar documents
pub async fn search_similar(
    state: &AppState,
    collection_name: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, Box<dyn Error + Send + Sync>> {
    debug!("Searching for: {}", query);

    // Generate embedding for query
    let embeddings = generate_embeddings(vec![query.to_string()]).await?;

    if embeddings.is_empty() {
        error!("Failed to generate query embedding");
        return Err("Failed to generate query embedding".into());
    }

    let query_embedding = embeddings[0].clone();

    // Search in Qdrant
    let client = get_qdrant_client(state)?;
    let results = client
        .search(collection_name, query_embedding, limit)
        .await?;

    // Convert to our SearchResult format
    let search_results: Vec<SearchResult> = results
        .into_iter()
        .map(|r| SearchResult {
            file_path: r
                .payload
                .as_ref()
                .and_then(|p| p.get("file_path"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            chunk_text: r
                .payload
                .as_ref()
                .and_then(|p| p.get("chunk_text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            score: r.score,
            chunk_index: r
                .payload
                .as_ref()
                .and_then(|p| p.get("chunk_index"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as usize,
        })
        .collect();

    debug!("Found {} similar documents", search_results.len());

    Ok(search_results)
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: String,
    pub chunk_text: String,
    pub score: f32,
    pub chunk_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_into_chunks() {
        let text = "a".repeat(1000);
        let chunks = split_into_chunks(&text);

        // Should have at least 2 chunks
        assert!(chunks.len() >= 2);

        // First chunk should be CHUNK_SIZE
        assert_eq!(chunks[0].len(), CHUNK_SIZE);
    }

    #[test]
    fn test_split_short_text() {
        let text = "Short text";
        let chunks = split_into_chunks(text);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_split_empty_text() {
        let text = "";
        let chunks = split_into_chunks(text);

        assert_eq!(chunks.len(), 0);
    }
}
