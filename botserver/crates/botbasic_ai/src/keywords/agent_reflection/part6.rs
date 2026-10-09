use super::*;

impl ReflectionEngine {
    pub fn get_insights(&self, limit: usize) -> Result<Vec<ReflectionResult>, String> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

        #[derive(QueryableByName)]
        struct ReflectionRow {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            session_id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)]
            reflection_type: String,
            #[diesel(sql_type = diesel::sql_types::Float)]
            score: f32,
            #[diesel(sql_type = diesel::sql_types::Text)]
            insights: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            improvements: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            positive_patterns: String,
            #[diesel(sql_type = diesel::sql_types::Integer)]
            messages_analyzed: i32,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let rows: Vec<ReflectionRow> = diesel::sql_query(
            "SELECT id, session_id, reflection_type, score, insights, improvements, \
             positive_patterns, messages_analyzed, created_at \
             FROM bot_reflections WHERE bot_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(self.bot_id)
        .bind::<diesel::sql_types::Integer, _>(limit as i32)
        .load(&mut conn)
        .unwrap_or_default();

        let results: Vec<ReflectionResult> = rows
            .into_iter()
            .map(|row| {
                let reflection_type: ReflectionType =
                    serde_json::from_str(&row.reflection_type).unwrap_or_default();
                let insights: Vec<String> = serde_json::from_str(&row.insights).unwrap_or_default();
                let improvements: Vec<String> =
                    serde_json::from_str(&row.improvements).unwrap_or_default();
                let positive_patterns: Vec<String> =
                    serde_json::from_str(&row.positive_patterns).unwrap_or_default();

                ReflectionResult {
                    id: row.id,
                    bot_id: self.bot_id,
                    session_id: row.session_id,
                    reflection_type,
                    score: row.score,
                    insights,
                    improvements,
                    positive_patterns,
                    concerns: Vec::new(),
                    raw_response: String::new(),
                    timestamp: row.created_at,
                    messages_analyzed: row.messages_analyzed as usize,
                }
            })
            .collect();

        Ok(results)
    }

    pub fn should_reflect(&self, session_id: Uuid) -> bool {
        if !self.config.enabled {
            return false;
        }

        if let Ok(mut conn) = self.state.db_pool().get() {
            #[derive(QueryableByName)]
            struct CountRow {
                #[diesel(sql_type = diesel::sql_types::BigInt)]
                count: i64,
            }

            let result: Option<CountRow> = diesel::sql_query(
                "SELECT COUNT(*) as count FROM conversation_messages WHERE session_id = $1",
            )
            .bind::<diesel::sql_types::Uuid, _>(session_id)
            .get_result(&mut conn)
            .ok();

            if let Some(row) = result {
                return row.count > 0 && (row.count as u32).is_multiple_of(self.config.interval);
            }
        }

        false
    }
}
