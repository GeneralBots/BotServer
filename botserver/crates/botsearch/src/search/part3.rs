use super::*;

impl SearchService {
    pub fn new(pool: Arc<DbPool>, config: Option<SearchConfig>) -> Self {
        Self {
            pool,
            config: config.unwrap_or_else(SearchConfig::default_config),
        }
    }

    pub async fn search(&self, query: SearchQuery) -> Result<SearchResponse, SearchError> {
        let start_time = std::time::Instant::now();

        let sources = query
            .sources
            .clone()
            .unwrap_or_else(|| self.config.enabled_sources.clone());

        let limit = query.limit.unwrap_or(self.config.max_results_per_source);
        let offset = query.offset.unwrap_or(0);

        let mut conn = self.pool.get().map_err(|e| {
            log::error!("Failed to get database connection: {e}");
            SearchError::DatabaseConnection
        })?;

        let sanitized_query = self.sanitize_query(&query.query);

        let source_filter: Vec<String> = sources.iter().map(|s| s.to_string()).collect();
        let source_list = source_filter.join("','");

        let sql = format!(
            r#"
            SELECT
                id,
                source,
                title,
                ts_headline($1, content, plainto_tsquery($1, $2),
                    'MaxWords=35, MinWords=15, StartSel=<mark>, StopSel=</mark>') as snippet,
                url,
                ts_rank_cd(search_vector, plainto_tsquery($1, $2)) as score,
                created_at,
                updated_at,
                metadata::text as metadata_json
            FROM search_index
            WHERE organization_id = $3
              AND source IN ('{source_list}')
              AND search_vector @@ plainto_tsquery($1, $2)
              {date_filter}
            ORDER BY score DESC
            LIMIT $4 OFFSET $5
            "#,
            date_filter = self.build_date_filter(&query.from_date, &query.to_date),
        );

        let results: Vec<FtsSearchRow> = diesel::sql_query(&sql)
            .bind::<Text, _>(&self.config.postgres_fts_config)
            .bind::<Text, _>(&sanitized_query)
            .bind::<diesel::sql_types::Uuid, _>(query.organization_id)
            .bind::<Integer, _>(limit)
            .bind::<Integer, _>(offset)
            .load(&mut conn)
            .map_err(|e| {
                log::error!("Search query failed: {e}");
                SearchError::QueryFailed(e.to_string())
            })?;

        let count_sql = format!(
            r#"
            SELECT COUNT(*) as count
            FROM search_index
            WHERE organization_id = $1
              AND source IN ('{source_list}')
              AND search_vector @@ plainto_tsquery($2, $3)
              {date_filter}
            "#,
            date_filter = self.build_date_filter(&query.from_date, &query.to_date),
        );

        let count_result: Vec<CountRow> = diesel::sql_query(&count_sql)
            .bind::<diesel::sql_types::Uuid, _>(query.organization_id)
            .bind::<Text, _>(&self.config.postgres_fts_config)
            .bind::<Text, _>(&sanitized_query)
            .load(&mut conn)
            .map_err(|e| {
                log::error!("Count query failed: {e}");
                SearchError::QueryFailed(e.to_string())
            })?;

        let total_count = count_result.first().map(|r| r.count).unwrap_or(0);

        let search_results: Vec<SearchResult> = results
            .into_iter()
            .map(|row| {
                let metadata: HashMap<String, String> = row
                    .metadata_json
                    .and_then(|json| serde_json::from_str(&json).ok())
                    .unwrap_or_default();

                SearchResult {
                    id: row.id,
                    source: self.parse_source(&row.source),
                    title: row.title,
                    snippet: row.snippet,
                    url: row.url,
                    score: row.score,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    metadata,
                }
            })
            .collect();

        let query_time_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "Search completed: query='{}' results={} time={}ms",
            query.query,
            search_results.len(),
            query_time_ms
        );

        Ok(SearchResponse {
            results: search_results,
            total_count,
            query_time_ms,
            sources_searched: sources,
        })
    }

    /// CRM contact lookup for the chat mention autocomplete (#1437): matches
    /// `crm_contacts` rows owned by the caller's org so the selected person
    /// carries its record id for the `app://crm?person_id=` deep link.
    /// Returns empty on an unusable index (missing/miscreated table) so the
    /// mention picker degrades to type hints instead of failing the chat.
    pub async fn mention_people(
        &self,
        org_id: Uuid,
        term: &str,
        limit: i64,
    ) -> Result<Vec<CrmPersonMatch>, SearchError> {
        let trimmed = term.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let pattern = format!("%{}%", trimmed.to_lowercase());
        let mut conn = self.pool.get().map_err(|e| {
            log::error!("mention people: failed to get database connection: {e}");
            SearchError::DatabaseConnection
        })?;
        let rows: Vec<CrmPersonMatch> = diesel::sql_query(
            "SELECT c.id AS id, \
                    COALESCE(c.first_name, '') || ' ' || COALESCE(c.last_name, '') AS title, \
                    COALESCE(c.email, '') AS subtitle \
             FROM crm_contacts c \
             WHERE (c.org_id = $1 OR c.branch_id = $1) \
               AND (LOWER(c.first_name) LIKE $2 OR LOWER(c.last_name) LIKE $2 OR LOWER(c.email) LIKE $2) \
             ORDER BY c.created_at DESC \
             LIMIT $3",
        )
        .bind::<diesel::sql_types::Uuid, _>(org_id)
        .bind::<diesel::sql_types::Text, _>(&pattern)
        .bind::<diesel::sql_types::BigInt, _>(limit)
        .load(&mut conn)
        .map_err(|e| {
            log::warn!("mention people: query failed: {e}");
            SearchError::QueryFailed(e.to_string())
        })?;
        Ok(rows)
    }

    pub async fn index_document(&self, doc: DocumentToIndex) -> Result<(), SearchError> {
        let mut conn = self.pool.get().map_err(|e| {
            log::error!("Failed to get database connection: {e}");
            SearchError::DatabaseConnection
        })?;

        let metadata_json = serde_json::to_string(&doc.metadata).unwrap_or_else(|_| "{}".to_string());

        let sql = r#"
            INSERT INTO search_index (id, source, organization_id, title, content, url, metadata, search_vector, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb,
                    setweight(to_tsvector($8, $4), 'A') || setweight(to_tsvector($8, $5), 'B'),
                    $9, NOW())
            ON CONFLICT (id, source) DO UPDATE SET
                title = EXCLUDED.title,
                content = EXCLUDED.content,
                url = EXCLUDED.url,
                metadata = EXCLUDED.metadata,
                search_vector = EXCLUDED.search_vector,
                updated_at = NOW()
        "#;

        diesel::sql_query(sql)
            .bind::<Text, _>(&doc.id)
            .bind::<Text, _>(doc.source.to_string())
            .bind::<diesel::sql_types::Uuid, _>(doc.organization_id)
            .bind::<Text, _>(&doc.title)
            .bind::<Text, _>(&doc.content)
            .bind::<Nullable<Text>, _>(doc.url.as_deref())
            .bind::<Text, _>(&metadata_json)
            .bind::<Text, _>(&self.config.postgres_fts_config)
            .bind::<Timestamptz, _>(doc.created_at)
            .execute(&mut conn)
            .map_err(|e| {
                log::error!("Failed to index document {}: {e}", doc.id);
                SearchError::IndexFailed(e.to_string())
            })?;

        debug!("Indexed document: id={} source={}", doc.id, doc.source);
        Ok(())
    }

    pub async fn index_documents(&self, docs: Vec<DocumentToIndex>) -> Result<IndexResult, SearchError> {
        let mut success_count = 0;
        let mut error_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for doc in docs {
            match self.index_document(doc.clone()).await {
                Ok(()) => success_count += 1,
                Err(e) => {
                    error_count += 1;
                    errors.push(format!("{}: {e}", doc.id));
                }
            }
        }

        Ok(IndexResult {
            success_count,
            error_count,
            errors,
        })
    }

    pub async fn delete_document(
        &self,
        organization_id: Uuid,
        id: &str,
        source: SearchSource,
    ) -> Result<(), SearchError> {
        let mut conn = self.pool.get().map_err(|e| {
            log::error!("Failed to get database connection: {e}");
            SearchError::DatabaseConnection
        })?;

        diesel::sql_query(
            "DELETE FROM search_index WHERE organization_id = $1 AND id = $2 AND source = $3",
        )
        .bind::<diesel::sql_types::Uuid, _>(organization_id)
        .bind::<Text, _>(id)
        .bind::<Text, _>(source.to_string())
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to delete document {}: {e}", id);
            SearchError::DeleteFailed(e.to_string())
        })?;

        debug!("Deleted document from index: id={} source={}", id, source);
        Ok(())
    }

    pub async fn get_index_stats(&self, organization_id: Uuid) -> Result<Vec<IndexStats>, SearchError> {
        let mut conn = self.pool.get().map_err(|e| {
            log::error!("Failed to get database connection: {e}");
            SearchError::DatabaseConnection
        })?;

        let sql = r#"
            SELECT
                source,
                COUNT(*) as document_count,
                MAX(updated_at) as last_indexed,
                pg_total_relation_size('search_index') /
                    GREATEST((SELECT COUNT(DISTINCT source) FROM search_index WHERE organization_id = $1), 1) as index_size_bytes
            FROM search_index
            WHERE organization_id = $1
            GROUP BY source
        "#;

        let rows: Vec<IndexStatsRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .map_err(|e| {
                log::error!("Failed to get index stats: {e}");
                SearchError::QueryFailed(e.to_string())
            })?;

        let stats = rows
            .into_iter()
            .map(|row| IndexStats {
                source: self.parse_source(&row.source),
                document_count: row.document_count,
                last_indexed: row.last_indexed,
                index_size_bytes: row.index_size_bytes,
            })
            .collect();

        Ok(stats)
    }
}
