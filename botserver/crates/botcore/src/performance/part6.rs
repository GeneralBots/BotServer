use super::*;

impl QueryOptimizer {
    pub fn new(slow_query_threshold_ms: u64) -> Self {
        Self {
            slow_query_threshold_ms,
            query_history: Arc::new(RwLock::new(Vec::new())),
            table_statistics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_query(
        &self,
        query_pattern: &str,
        execution_time_ms: u64,
        rows_examined: u64,
        rows_returned: u64,
    ) {
        let query_hash = self.hash_query(query_pattern);
        let is_slow = execution_time_ms > self.slow_query_threshold_ms;

        let mut history = self.query_history.write().await;

        if let Some(existing) = history.iter_mut().find(|m| m.query_hash == query_hash) {
            existing.execution_count += 1;
            existing.total_time_ms += execution_time_ms;
            existing.avg_time_ms = existing.total_time_ms as f64 / existing.execution_count as f64;
            existing.max_time_ms = existing.max_time_ms.max(execution_time_ms);
            existing.min_time_ms = existing.min_time_ms.min(execution_time_ms);
            existing.rows_examined += rows_examined;
            existing.rows_returned += rows_returned;
            existing.last_executed = Utc::now();
            existing.is_slow = is_slow || existing.is_slow;
        } else {
            history.push(QueryMetrics {
                query_hash,
                query_pattern: query_pattern.to_string(),
                execution_count: 1,
                total_time_ms: execution_time_ms,
                avg_time_ms: execution_time_ms as f64,
                max_time_ms: execution_time_ms,
                min_time_ms: execution_time_ms,
                rows_examined,
                rows_returned,
                last_executed: Utc::now(),
                is_slow,
            });
        }

        if history.len() > 10000 {
            history.sort_by_key(|m| std::cmp::Reverse(m.last_executed));
            history.truncate(5000);
        }
    }

    pub async fn get_slow_queries(&self, limit: usize) -> Vec<QueryMetrics> {
        let history = self.query_history.read().await;
        let mut slow: Vec<_> = history.iter().filter(|m| m.is_slow).cloned().collect();
        slow.sort_by(|a, b| b.avg_time_ms.partial_cmp(&a.avg_time_ms).unwrap_or(std::cmp::Ordering::Equal));
        slow.truncate(limit);
        slow
    }

    pub async fn get_query_suggestions(&self, query_pattern: &str) -> Vec<QuerySuggestion> {
        let mut suggestions = Vec::new();

        if query_pattern.to_lowercase().contains("select *") {
            suggestions.push(QuerySuggestion {
                suggestion_type: PerformanceSuggestionType::RemoveSelectStar,
                description: "Replace SELECT * with specific columns to reduce data transfer".to_string(),
                impact: Impact::Medium,
                implementation: Some("SELECT specific_column1, specific_column2 FROM ...".to_string()),
            });
        }

        if !query_pattern.to_lowercase().contains("limit") {
            suggestions.push(QuerySuggestion {
                suggestion_type: PerformanceSuggestionType::AddLimit,
                description: "Add LIMIT clause to prevent fetching too many rows".to_string(),
                impact: Impact::High,
                implementation: Some("... LIMIT 100".to_string()),
            });
        }

        if query_pattern.to_lowercase().contains("where")
            && !query_pattern.to_lowercase().contains("index")
        {
            suggestions.push(QuerySuggestion {
                suggestion_type: PerformanceSuggestionType::AddIndex,
                description: "Consider adding an index on filtered columns".to_string(),
                impact: Impact::High,
                implementation: None,
            });
        }

        if query_pattern.to_lowercase().contains("in (select") {
            suggestions.push(QuerySuggestion {
                suggestion_type: PerformanceSuggestionType::UseJoinInsteadOfSubquery,
                description: "Replace IN subquery with JOIN for better performance".to_string(),
                impact: Impact::Medium,
                implementation: Some("Use INNER JOIN instead of IN (SELECT ...)".to_string()),
            });
        }

        suggestions
    }

    pub async fn update_table_stats(&self, table_name: &str, stats: TableStats) {
        let mut table_stats = self.table_statistics.write().await;
        table_stats.insert(table_name.to_string(), stats);
    }

    pub async fn get_table_stats(&self, table_name: &str) -> Option<TableStats> {
        let table_stats = self.table_statistics.read().await;
        table_stats.get(table_name).cloned()
    }

    pub async fn analyze_query(&self, query_pattern: &str) -> QueryPlan {
        let suggestions = self.get_query_suggestions(query_pattern).await;

        let mut warnings = Vec::new();

        if query_pattern.to_lowercase().contains("select *") {
            warnings.push("SELECT * can impact performance".to_string());
        }

        if !query_pattern.to_lowercase().contains("limit") {
            warnings.push("Query has no LIMIT clause".to_string());
        }

        QueryPlan {
            estimated_rows: 1000,
            estimated_cost: 100.0,
            index_usage: Vec::new(),
            warnings,
            suggestions,
        }
    }

    pub(super) fn hash_query(&self, query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;

        let normalized = self.normalize_query(query);
        let mut hasher = DefaultHasher::new();
        normalized.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub(super) fn normalize_query(&self, query: &str) -> String {
        let mut result = query.to_lowercase();

        let patterns = [
            (r"'[^']*'", "'?'"),
            (r"\d+", "?"),
            (r"\s+", " "),
        ];

        for (pattern, replacement) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                result = re.replace_all(&result, replacement).to_string();
            }
        }

        result.trim().to_string()
    }

    pub async fn get_performance_report(&self) -> PerformanceReport {
        let history = self.query_history.read().await;

        let total_queries: u64 = history.iter().map(|m| m.execution_count).sum();
        let slow_queries: u64 = history.iter().filter(|m| m.is_slow).map(|m| m.execution_count).sum();

        let avg_time: f64 = if total_queries > 0 {
            history.iter().map(|m| m.total_time_ms).sum::<u64>() as f64 / total_queries as f64
        } else {
            0.0
        };

        let top_slow = self.get_slow_queries(10).await;

        PerformanceReport {
            generated_at: Utc::now(),
            total_queries,
            slow_queries,
            slow_query_percentage: if total_queries > 0 {
                (slow_queries as f64 / total_queries as f64) * 100.0
            } else {
                0.0
            },
            average_query_time_ms: avg_time,
            top_slow_queries: top_slow,
            recommendations: self.generate_recommendations(&history).await,
        }
    }

    async fn generate_recommendations(&self, history: &[QueryMetrics]) -> Vec<String> {
        let mut recommendations = Vec::new();

        let slow_count = history.iter().filter(|m| m.is_slow).count();
        if slow_count > history.len() / 10 {
            recommendations.push(
                "More than 10% of queries are slow. Consider reviewing query patterns and adding indexes."
                    .to_string(),
            );
        }

        let high_row_scans: Vec<_> = history
            .iter()
            .filter(|m| m.rows_examined > m.rows_returned * 100)
            .collect();

        if !high_row_scans.is_empty() {
            recommendations.push(
                format!(
                    "{} queries examine many more rows than returned. Consider adding appropriate indexes.",
                    high_row_scans.len()
                )
            );
        }

        if recommendations.is_empty() {
            recommendations.push("Query performance looks good. Continue monitoring.".to_string());
        }

        recommendations
    }
}
