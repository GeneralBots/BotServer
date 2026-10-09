use super::*;

pub(super) fn bot_scope_clause(bot_id: &Option<uuid::Uuid>) -> String {
    match bot_id {
        Some(id) => format!(" AND bot_id = '{}'", id),
        None => String::new(),
    }
}
pub async fn handle_source_counts<S: ResearchState>(
    State(state): State<Arc<S>>,
) -> impl IntoResponse {
    let conn = state.db_pool().clone();
    let bot_id = state.bot_id();
    let scope = bot_scope_clause(&bot_id);

    let (all, web) = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return (0i64, 0i64);
            }
        };

        let total: i64 = diesel::sql_query(&format!(
            "SELECT COUNT(*) AS n FROM kb_documents WHERE 1 = 1{}",
            scope
        ))
        .get_result::<SourceCountRow>(&mut db_conn)
        .map(|r| r.n)
        .unwrap_or(0);

        (total, 0i64)
    })
    .await
    .unwrap_or((0, 0));

    let docs = all.saturating_sub(web);
    Json(serde_json::json!({
        "all": all,
        "web": web,
        "docs": docs,
        "kb": all,
    }))
}
pub async fn handle_list_collections<S: ResearchState>(
    State(state): State<Arc<S>>,
) -> impl IntoResponse {
    let conn = state.db_pool().clone();
    let bot_id = state.bot_id();
    let scope = bot_scope_clause(&bot_id);

    let collections = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        let result: Result<Vec<CollectionRow>, _> = diesel::sql_query(&format!(
            "SELECT id, name, folder_path, document_count::bigint AS document_count FROM kb_collections WHERE 1 = 1{} ORDER BY name ASC",
            scope
        ))
        .load(&mut db_conn);

        match result {
            Ok(colls) => colls,
            Err(e) => {
                log::error!("Failed to load research collections: {}", e);
                Vec::new()
            }
        }
    })
    .await
    .unwrap_or_default();

    let mut html = String::new();

    for c in &collections {
        html.push_str("<div class=\"collection-item\" data-id=\"");
        html.push_str(&html_escape(&c.id.to_string()));
        html.push_str("\">");
        html.push_str("<div class=\"collection-icon\"></div>");
        html.push_str("<div class=\"collection-info\">");
        html.push_str("<span class=\"collection-name\">");
        html.push_str(&html_escape(&c.name));
        html.push_str("</span>");
        html.push_str("<span class=\"collection-desc\">");
        html.push_str(&html_escape(&c.folder_path));
        html.push_str(" · ");
        html.push_str(&c.document_count.to_string());
        html.push_str(" documents</span>");
        html.push_str("</div>");
        html.push_str("<button class=\"btn-icon-sm\" hx-get=\"/api/ui/research/collections/");
        html.push_str(&html_escape(&c.id.to_string()));
        html.push_str("\" hx-target=\"#main-results\">");
        html.push_str("<svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><polyline points=\"9 18 15 12 9 6\"></polyline></svg>");
        html.push_str("</button>");
        html.push_str("</div>");
    }

    if collections.is_empty() {
        html.push_str("<div class=\"empty-state\">");
        html.push_str("<p>No collections yet</p>");
        html.push_str("<p class=\"hint\">Create a collection to organize your knowledge base</p>");
        html.push_str("</div>");
    }

    Html(html)
}
pub async fn handle_create_collection<S: ResearchState>(
    State(state): State<Arc<S>>,
    Json(payload): Json<NewCollectionRequest>,
) -> impl IntoResponse {
    let conn = state.db_pool().clone();
    let id = uuid::Uuid::new_v4();
    let bot_id = state.bot_id();
    let name = payload.name.clone();
    let description = payload.description.unwrap_or_default();
    let qdrant_collection = format!("kb_{}", name.to_lowercase().replace(' ', "_"));

    let id_clone = id;
    let name_clone = name.clone();
    let folder_clone = description.clone();
    let folder_for_closure = folder_clone.clone();

    let _ = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return;
            }
        };

        let _ = diesel::sql_query(
            "INSERT INTO kb_collections (id, bot_id, name, folder_path, qdrant_collection, document_count) VALUES ($1, $2, $3, $4, $5, 0)",
        )
        .bind::<diesel::sql_types::Uuid, _>(&id_clone)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(bot_id)
        .bind::<diesel::sql_types::Text, _>(&name)
        .bind::<diesel::sql_types::Text, _>(&folder_for_closure)
        .bind::<diesel::sql_types::Text, _>(&qdrant_collection)
        .execute(&mut db_conn);
    })
    .await;

    let mut html = String::new();
    html.push_str("<div class=\"collection-item new-item\" data-id=\"");
    html.push_str(&html_escape(&id_clone.to_string()));
    html.push_str("\">");
    html.push_str("<div class=\"collection-icon\"></div>");
    html.push_str("<div class=\"collection-info\">");
    html.push_str("<span class=\"collection-name\">");
    html.push_str(&html_escape(&name_clone));
    html.push_str("</span>");
    html.push_str("<span class=\"collection-desc\">");
    html.push_str(&html_escape(&folder_clone));
    html.push_str("</span>");
    html.push_str("</div>");
    html.push_str("</div>");

    Html(html)
}
pub async fn handle_get_collection<S: ResearchState>(
    State(state): State<Arc<S>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = state.db_pool().clone();

    let documents = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        let collection_id = match uuid::Uuid::parse_str(&id) {
            Ok(u) => u.to_string(),
            Err(_) => return Vec::new(),
        };

        diesel::sql_query(
            "SELECT id, file_path, collection_name, metadata FROM kb_documents WHERE collection_name = $1 ORDER BY file_path ASC LIMIT 50",
        )
        .bind::<diesel::sql_types::Text, _>(&collection_id)
        .load::<KbDocumentRow>(&mut db_conn)
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::new();
    html.push_str("<div class=\"collection-results\">");
    html.push_str("<div class=\"results-header\">");
    html.push_str("<h3>Collection Contents</h3>");
    html.push_str("<span class=\"result-count\">");
    html.push_str(&documents.len().to_string());
    html.push_str(" documents</span>");
    html.push_str("</div>");
    html.push_str("<div class=\"results-list\">");

    if documents.is_empty() {
        html.push_str("<div class=\"empty-state\">");
        html.push_str("<p>No documents in this collection</p>");
        html.push_str("<p class=\"hint\">Add documents to build your knowledge base</p>");
        html.push_str("</div>");
    } else {
        for doc in &documents {
            html.push_str(&format_search_result(doc));
        }
    }

    html.push_str("</div>");
    html.push_str("</div>");

    Html(html)
}
