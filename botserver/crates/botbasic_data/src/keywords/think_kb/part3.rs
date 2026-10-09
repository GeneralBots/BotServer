use super::*;

pub(super) fn get_accessible_kb_collections(
    conn: &mut diesel::PgConnection,
    bot_id: uuid::Uuid,
    accessible_ids: &[uuid::Uuid],
) -> Result<Vec<KbCollectionRow>, String> {
    if accessible_ids.is_empty() {
        return Ok(Vec::new());
    }

    let id_str: String = accessible_ids.iter().map(|id| format!("'{}'", id)).collect::<Vec<_>>().join(",");

    let query_str = format!(
        "SELECT name, qdrant_collection, document_count FROM kb_collections WHERE bot_id = $1 AND id IN ({})",
        id_str
    );

    diesel::sql_query(&query_str)
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .load::<KbCollectionRow>(conn)
        .map_err(|e| format!("Failed to query KB collections: {e}"))
}
pub(super) fn get_recent_consolidations(
    conn: &mut diesel::PgConnection,
    bot_id: uuid::Uuid,
    limit: i64,
) -> Vec<ConsolidationRow> {
    diesel::sql_query(
        "SELECT insight, summary, source_ids, created_at FROM kb_consolidations
        WHERE bot_id = $1
        ORDER BY created_at DESC
        LIMIT $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .bind::<diesel::sql_types::BigInt, _>(limit)
    .load::<ConsolidationRow>(conn)
    .unwrap_or_default()
}
pub(super) fn get_kb_memory_stats(
    conn: &mut diesel::PgConnection,
    bot_name: &str,
    bot_id: uuid::Uuid,
) -> serde_json::Value {
    #[derive(QueryableByName)]
    struct CountRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    let file_prefix = format!("{}.gbai/%", bot_name);

    let total_memories: i64 = diesel::sql_query(
        "SELECT COUNT(*) as count FROM drive_files WHERE file_path LIKE $1 AND file_type = 'kb' AND indexed = true",
    )
    .bind::<diesel::sql_types::Text, _>(&file_prefix)
    .get_result::<CountRow>(conn)
    .map(|r| r.count)
    .unwrap_or(0);

    let total_consolidations: i64 = diesel::sql_query(
        "SELECT COUNT(*) as count FROM kb_consolidations WHERE bot_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .get_result::<CountRow>(conn)
    .map(|r| r.count)
    .unwrap_or(0);



    let unconsolidated: i64 = diesel::sql_query(
        "SELECT COUNT(*) as count FROM drive_files WHERE file_path LIKE $1 AND file_type = 'kb' AND indexed = true
        AND NOT EXISTS (
            SELECT 1 FROM kb_consolidation_sources kcs
            JOIN kb_consolidations kc ON kc.id = kcs.consolidation_id
            WHERE kcs.file_path = drive_files.file_path
        )",
    )
    .bind::<diesel::sql_types::Text, _>(&file_prefix)
    .get_result::<CountRow>(conn)
    .map(|r| r.count)
    .unwrap_or(0);

    json!({
        "total_memories": total_memories,
        "total_consolidations": total_consolidations,
        "unconsolidated": unconsolidated
    })
}
pub(super) async fn search_qdrant_with_real_embeddings(
    collection_name: &str,
    query: &str,
    limit: usize,
    bot_id: uuid::Uuid,
    db_pool: &botbasic_types::types::DbPool,
) -> Result<Vec<serde_json::Value>, String> {
    let qdrant_url = std::env::var("QDRANT_URL")
        .ok()
        .or_else(|| std::env::var("VECTORDB_URL").ok())
        .unwrap_or_else(|| "http://127.0.0.1:6333".to_string());
    let api_key = std::env::var("QDRANT_API_KEY")
        .ok()
        .or_else(|| std::env::var("VECTORDB_API_KEY").ok())
        .unwrap_or_default();

    let client = botlib::security::create_tls_client(None);

    let check_url = format!("{}/collections/{}", qdrant_url.trim_end_matches('/'), collection_name);
    let dim = match client.get(&check_url).header("api-key", &api_key).send().await {
        Ok(r) if r.status().is_success() => {
            let info: serde_json::Value = r.json().await.unwrap_or_default();
            info["result"]["config"]["params"]["vectors"]["size"].as_u64().map(|d| d as usize)
        }
        _ => None,
    };

    let dimensions = dim.unwrap_or(384);

    let vector = {
        let config = botcore::kb::EmbeddingConfig::from_bot_config(db_pool, &bot_id);
        if config.embedding_url.is_empty() {
            generate_hash_embedding(query, dimensions)
        } else {
            let generator = botcore::kb::KbEmbeddingGenerator::new(config);
            match generator.generate_single_embedding(query).await {
                Ok(embedding) => embedding.vector,
                Err(e) => {
                    warn!("THINK KB: embedding generation failed ({}), using hash fallback", e);
                    generate_hash_embedding(query, dimensions)
                }
            }
        }
    };

    let search_url = format!("{}/collections/{}/points/search", qdrant_url.trim_end_matches('/'), collection_name);
    let mut request = client
        .post(&search_url)
        .json(&serde_json::json!({
            "limit": limit,
            "with_vector": false,
            "with_payload": true,
            "params": { "hnsw_ef": 128, "exact": false },
            "vector": vector
        }));

    if !api_key.is_empty() {
        request = request.header("api-key", &api_key);
    }

    let response = request.send().await.map_err(|e| format!("Qdrant request failed: {e}"))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }

    if !response.status().is_success() {
        return Ok(Vec::new());
    }

    let result: serde_json::Value = response.json().await.map_err(|e| format!("Qdrant parse error: {e}"))?;

    let points = result["result"].as_array().cloned().unwrap_or_default();

    let mut results = Vec::new();
    for point in &points {
        let score = point.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        if score < 0.15 {
            continue;
        }

        let payload = point.get("payload");
        let content = payload
            .and_then(|p| p.get("content").or_else(|| p.get("text")).or_else(|| p.get("data")))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let source = payload
            .and_then(|p| p.get("document_path").or_else(|| p.get("source")).or_else(|| p.get("file")))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if content.is_empty() || content.len() < 10 {
            continue;
        }

        results.push(json!({
            "content": content,
            "source": source,
            "score": score,
        }));
    }

    Ok(results)
}
pub(super) fn fnv1a_hash(text: &str, salt: usize) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for &b in text.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash ^= salt as u64;
    hash = hash.wrapping_mul(1099511628211);
    hash
}
pub(super) fn generate_hash_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let mut embedding = Vec::with_capacity(dimensions);
    for i in 0..dimensions {
        let hash = fnv1a_hash(text, i);
        embedding.push((hash as f32) / (u64::MAX as f32) * 2.0 - 1.0);
    }
    embedding
}
pub(super) fn calculate_confidence(avg_score: f32, total_results: usize, consolidation_count: usize) -> f32 {
    let score_factor = avg_score;
    let result_factor = (total_results as f32 / 10.0).min(1.0) * 0.3;
    let consolidation_factor = (consolidation_count as f32 / 5.0).min(1.0) * 0.2;
    (score_factor * 0.5 + result_factor + consolidation_factor).min(1.0)
}
