use super::*;

/// Resolve the default bot id for the current deployment. `knowledge_sources`
/// and `knowledge_chunks` scope by `bot_id`, so every read must filter by it to
/// enforce data isolation between branches/tenants.
pub(super) fn resolve_bot_id(pool: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>) -> Uuid {
    let Ok(mut conn) = pool.get() else {
        return Uuid::nil();
    };
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct BotIdRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }
    diesel::sql_query(
        "SELECT id FROM bots WHERE is_default_for_branch = TRUE ORDER BY created_at ASC LIMIT 1",
    )
    .get_result::<BotIdRow>(&mut conn)
    .map(|r| r.id)
    .unwrap_or_else(|e| {
        log::error!("Failed to resolve default bot id: {e}");
        Uuid::nil()
    })
}
pub async fn handle_upload_document(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut file_name = String::new();
    let mut file_data: Vec<u8> = Vec::new();
    let mut collection = "default".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                file_name = field.file_name().unwrap_or("unknown").to_string();
                if let Ok(data) = field.bytes().await {
                    file_data = data.to_vec();
                }
            }
            "collection" => {
                if let Ok(text) = field.text().await {
                    collection = text;
                }
            }
            _ => {}
        }
    }

    if file_data.is_empty() {
        return Json(UploadResponse {
            success: false,
            source_id: None,
            message: "No file provided".to_string(),
            chunks_created: None,
        });
    }

    let extension = file_name.rsplit('.').next().unwrap_or("txt").to_lowercase();
    let source_type = SourceType::from(extension.as_str());

    let content = match extract_text_content(&file_data, &source_type) {
        Ok(text) => text,
        Err(e) => {
            log::error!("Failed to extract text from {}: {}", file_name, e);
            return Json(UploadResponse {
                success: false,
                source_id: None,
                message: format!("Failed to extract text: {}", e),
                chunks_created: None,
            });
        }
    };

    let content_hash = compute_content_hash(&content);
    let source_id = Uuid::new_v4().to_string();

    let chunks = chunk_text(&content, 512, 50);
    let chunk_count = chunks.len() as i32;

    let conn = state.conn.clone();
    let source_id_clone = source_id.clone();
    let file_name_clone = file_name.clone();
    let source_type_str = source_type.to_string();
    let content_hash_clone = content_hash.clone();
    let collection_clone = collection.clone();
    let chunks_clone = chunks.clone();
    let bot_id = resolve_bot_id(&conn);

    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Err(format!("Database connection error: {}", e));
            }
        };

        let insert_result = diesel::sql_query(
            "INSERT INTO knowledge_sources (id, name, source_type, content_hash, chunk_count, status, collection, bot_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, 'processing', $6, $7, NOW(), NOW())
             ON CONFLICT (id) DO NOTHING"
        )
        .bind::<diesel::sql_types::Text, _>(&source_id_clone)
        .bind::<diesel::sql_types::Text, _>(&file_name_clone)
        .bind::<diesel::sql_types::Text, _>(&source_type_str)
        .bind::<diesel::sql_types::Text, _>(&content_hash_clone)
        .bind::<diesel::sql_types::Integer, _>(chunk_count)
        .bind::<diesel::sql_types::Text, _>(&collection_clone)
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .execute(&mut db_conn);

        if let Err(e) = insert_result {
            log::error!("Failed to insert source: {}", e);
            return Err(format!("Failed to insert source: {}", e));
        }

        for (idx, chunk_content) in chunks_clone.iter().enumerate() {
            let chunk_id = Uuid::new_v4().to_string();
            let token_count = estimate_tokens(chunk_content);

            let chunk_result = diesel::sql_query(
                "INSERT INTO knowledge_chunks (id, source_id, chunk_index, content, token_count, created_at)
                 VALUES ($1, $2, $3, $4, $5, NOW())
                 ON CONFLICT (id) DO NOTHING"
            )
            .bind::<diesel::sql_types::Text, _>(&chunk_id)
            .bind::<diesel::sql_types::Text, _>(&source_id_clone)
            .bind::<diesel::sql_types::Integer, _>(idx as i32)
            .bind::<diesel::sql_types::Text, _>(chunk_content)
            .bind::<diesel::sql_types::Integer, _>(token_count)
            .execute(&mut db_conn);

            if let Err(e) = chunk_result {
                warn!("Failed to insert chunk {}: {}", idx, e);
            }
        }

        let _ = diesel::sql_query(
            "UPDATE knowledge_sources SET status = 'indexed', indexed_at = NOW(), updated_at = NOW() WHERE id = $1"
        )
        .bind::<diesel::sql_types::Text, _>(&source_id_clone)
        .execute(&mut db_conn);

        Ok(())
    })
    .await;

    match result {
        Ok(Ok(())) => {
            info!(
                "Successfully ingested {} with {} chunks",
                file_name, chunk_count
            );
            Json(UploadResponse {
                success: true,
                source_id: Some(source_id),
                message: format!(
                    "Successfully ingested '{}' with {} chunks",
                    file_name, chunk_count
                ),
                chunks_created: Some(chunk_count),
            })
        }
        Ok(Err(e)) => Json(UploadResponse {
            success: false,
            source_id: None,
            message: e,
            chunks_created: None,
        }),
        Err(e) => Json(UploadResponse {
            success: false,
            source_id: None,
            message: format!("Task error: {}", e),
            chunks_created: None,
        }),
    }
}
pub async fn handle_list_sources(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListSourcesQuery>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let status_filter = params.status.clone();
    let type_filter = params.source_type.clone();
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;
    let bot_id = resolve_bot_id(&conn);

    let sources = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        let query = format!(
            "SELECT id, name, source_type, file_path, url, content_hash, chunk_count, status,\n             created_at::text, updated_at::text, indexed_at::text\n             FROM knowledge_sources WHERE bot_id = $1\n             AND ($2::text IS NULL OR status = $2)\n             AND ($3::text IS NULL OR source_type = $3)\n             ORDER BY created_at DESC LIMIT {per_page} OFFSET {offset}",
        );

        diesel::sql_query(&query)
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(status_filter.as_deref())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(type_filter.as_deref())
            .load::<KnowledgeSourceRow>(&mut db_conn)
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::new();
    html.push_str("<div class=\"sources-list\">");

    if sources.is_empty() {
        html.push_str("<div class=\"empty-state\">");
        html.push_str("<p>No knowledge sources found</p>");
        html.push_str("<p class=\"hint\">Upload documents to build your knowledge base</p>");
        html.push_str("</div>");
    } else {
        for source in &sources {
            let status_class = match source.status.as_str() {
                "indexed" => "status-success",
                "processing" => "status-processing",
                "failed" => "status-error",
                "pending" => "status-pending",
                _ => "status-unknown",
            };

            let type_icon = match source.source_type.as_str() {
                "pdf" => "📄",
                "docx" | "doc" => "📝",
                "txt" | "text" => "📃",
                "markdown" | "md" => "📋",
                "html" => "🌐",
                "csv" => "📊",
                "xlsx" | "xls" => "📈",
                "url" => "🔗",
                _ => "📁",
            };

            html.push_str("<div class=\"source-item\" data-id=\"");
            html.push_str(&html_escape(&source.id));
            html.push_str("\">");

            html.push_str("<div class=\"source-icon\">");
            html.push_str(type_icon);
            html.push_str("</div>");

            html.push_str("<div class=\"source-info\">");
            html.push_str("<h4 class=\"source-name\">");
            html.push_str(&html_escape(&source.name));
            html.push_str("</h4>");
            html.push_str("<div class=\"source-meta\">");
            html.push_str("<span class=\"source-type\">");
            html.push_str(&html_escape(&source.source_type));
            html.push_str("</span>");
            html.push_str("<span class=\"source-chunks\">");
            html.push_str(&source.chunk_count.to_string());
            html.push_str(" chunks</span>");
            html.push_str("<span class=\"source-status ");
            html.push_str(status_class);
            html.push_str("\">");
            html.push_str(&html_escape(&source.status));
            html.push_str("</span>");
            html.push_str("</div>");
            html.push_str("</div>");

            html.push_str("<div class=\"source-actions\">");
            html.push_str("<button class=\"btn-icon-sm\" title=\"Reindex\" ");
            html.push_str("hx-post=\"/api/sources/kb/reindex\" ");
            html.push_str("hx-vals='{\"source_ids\":[\"");
            html.push_str(&html_escape(&source.id));
            html.push_str("\"]}' hx-swap=\"none\">");
            html.push_str("<svg width=\"14\" height=\"14\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\">");
            html.push_str("<polyline points=\"23 4 23 10 17 10\"></polyline>");
            html.push_str("<path d=\"M20.49 15a9 9 0 1 1-2.12-9.36L23 10\"></path>");
            html.push_str("</svg></button>");

            html.push_str("<button class=\"btn-icon-sm btn-danger\" title=\"Delete\" ");
            html.push_str("hx-delete=\"/api/sources/kb/");
            html.push_str(&html_escape(&source.id));
            html.push_str("\" hx-confirm=\"Delete this source?\" hx-target=\"closest .source-item\" hx-swap=\"outerHTML\">");
            html.push_str("<svg width=\"14\" height=\"14\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\">");
            html.push_str("<polyline points=\"3 6 5 6 21 6\"></polyline>");
            html.push_str("<path d=\"M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2\"></path>");
            html.push_str("</svg></button>");
            html.push_str("</div>");

            html.push_str("</div>");
        }
    }

    html.push_str("</div>");
    Html(html)
}
