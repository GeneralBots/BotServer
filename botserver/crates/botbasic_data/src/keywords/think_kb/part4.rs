use super::*;

pub(super) async fn think_kb_search(
    db_pool: &botbasic_types::types::DbPool,
    session_id: uuid::Uuid,
    bot_id: uuid::Uuid,
    user_id: uuid::Uuid,
    query: &str,
) -> Result<serde_json::Value, String> {
    use botbasic_types::schema::bots;

    let (bot_name, kb_collections, consolidations, memory_stats) = {
        let mut conn = db_pool.get().map_err(|e| format!("DB error: {e}"))?;

        let bot_name: String = bots::table
            .filter(bots::id.eq(bot_id))
            .select(bots::name)
            .first(&mut conn)
            .map_err(|e| format!("Failed to get bot name for id {bot_id}: {e}"))?;

        let accessible_ids = get_accessible_kb_ids(&mut conn, user_id)?;
        let kb_collections = get_accessible_kb_collections(&mut conn, bot_id, &accessible_ids)?;
        let consolidations = get_recent_consolidations(&mut conn, bot_id, 10);
        let memory_stats = get_kb_memory_stats(&mut conn, &bot_name, bot_id);

        (bot_name, kb_collections, consolidations, memory_stats)
    };

    info!(
        "THINK KB: bot_name={}, session={}, query='{}', {} KBs, {} consolidations",
        bot_name, session_id, query, kb_collections.len(), consolidations.len()
    );

    let mut all_results = Vec::new();
    let mut total_score = 0.0f32;
    let mut scored_count = 0usize;

    for kb in &kb_collections {
        match search_qdrant_with_real_embeddings(&kb.qdrant_collection, query, 10, bot_id, db_pool).await {
            Ok(results) if !results.is_empty() => {
                info!("THINK KB: {} results from KB '{}' ({})", results.len(), kb.name, kb.qdrant_collection);
                for r in &results {
                    if let Some(s) = r.get("score").and_then(|v| v.as_f64()) {
                        total_score += s as f32;
                        scored_count += 1;
                    }
                }
                all_results.push(json!({
                    "kb_name": kb.name,
                    "collection": kb.qdrant_collection,
                    "document_count": kb.document_count,
                    "results": results,
                }));
            }
            Ok(_) => {
                info!("THINK KB: no results from KB '{}' ({})", kb.name, kb.qdrant_collection);
            }
            Err(e) => {
                warn!("THINK KB: search failed for KB '{}': {}", kb.name, e);
            }
        }
    }

    let consolidation_insights: Vec<serde_json::Value> = consolidations.iter().map(|c| {
        json!({
            "insight": c.insight,
            "summary": c.summary,
            "created_at": c.created_at,
        })
    }).collect();

    let sources: Vec<serde_json::Value> = all_results.iter().flat_map(|kb| {
        kb["results"].as_array().unwrap_or(&Vec::new()).iter().filter_map(|r| {
            r.get("source").and_then(|s| s.as_str()).filter(|s| !s.is_empty()).map(|s| json!(s))
        }).collect::<Vec<_>>()
    }).take(20).collect();

    let total_results_count: usize = all_results.iter().map(|kb| kb["results"].as_array().map(|a| a.len()).unwrap_or(0)).sum();

    let content = if let Some(first_insight) = consolidation_insights.first() {
        let insight = first_insight["insight"].as_str().unwrap_or("");
        if !insight.is_empty() {
            insight.to_string()
        } else {
            String::new()
        }
    } else {
        let mut lines: Vec<String> = Vec::new();
        for kb in &all_results {
            if let Some(results) = kb["results"].as_array() {
                for r in results {
                    let text = r.get("payload")
                        .and_then(|p| p.get("content").and_then(|v| v.as_str()))
                        .or_else(|| r.get("payload").and_then(|p| p.get("text").and_then(|v| v.as_str())))
                        .or_else(|| r.get("content").and_then(|v| v.as_str()))
                        .unwrap_or("");
                    if !text.is_empty() {
                        lines.push(text.to_string());
                    }
                }
            }
        }
        if lines.is_empty() {
            String::new()
        } else {
            lines.join("\n")
        }
    };

    let avg_score = if scored_count > 0 { total_score / scored_count as f32 } else { 0.0 };
    let confidence = calculate_confidence(avg_score, all_results.len(), consolidation_insights.len());

    Ok(json!({
        "results": all_results,
        "consolidation_insights": consolidation_insights,
        "confidence": confidence,
        "total_results": total_results_count,
        "sources": sources,
        "content": content,
        "query": query,
        "bot_name": bot_name,
        "memory_stats": memory_stats,
    }))
}
pub(super) fn json_to_dynamic(value: serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s),
        serde_json::Value::Array(arr) => {
            let mut rhai_array = rhai::Array::new();
            for item in arr {
                rhai_array.push(json_to_dynamic(item));
            }
            Dynamic::from(rhai_array)
        }
        serde_json::Value::Object(obj) => {
            let mut rhai_map = Map::new();
            for (key, val) in obj {
                rhai_map.insert(key.into(), json_to_dynamic(val));
            }
            Dynamic::from(rhai_map)
        }
    }
}
