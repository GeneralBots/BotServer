use super::*;

impl SearchService {
    pub async fn reindex_source(
        &self,
        organization_id: Uuid,
        source: SearchSource,
    ) -> Result<IndexResult, SearchError> {
        info!("Starting reindex for source={} org={}", source, organization_id);

        let documents = match source {
            SearchSource::Email => self.fetch_emails_for_indexing(organization_id).await?,
            SearchSource::Drive => self.fetch_drive_files_for_indexing(organization_id).await?,
            SearchSource::Calendar => self.fetch_calendar_events_for_indexing(organization_id).await?,
            SearchSource::Tasks => self.fetch_tasks_for_indexing(organization_id).await?,
            SearchSource::Transcription => {
                self.fetch_transcriptions_for_indexing(organization_id).await?
            }
            SearchSource::Chat => self.fetch_chat_messages_for_indexing(organization_id).await?,
            SearchSource::Contacts => self.fetch_contacts_for_indexing(organization_id).await?,
            SearchSource::Notes => self.fetch_notes_for_indexing(organization_id).await?,
        };

        let result = self.index_documents(documents).await?;

        info!(
            "Reindex completed for source={}: success={} errors={}",
            source, result.success_count, result.error_count
        );

        Ok(result)
    }

    pub async fn cleanup_old_index_entries(
        &self,
        organization_id: Uuid,
        before_date: DateTime<Utc>,
    ) -> Result<i64, SearchError> {
        let mut conn = self.pool.get().map_err(|e| {
            log::error!("Failed to get database connection: {e}");
            SearchError::DatabaseConnection
        })?;

        let result = diesel::sql_query(
            "DELETE FROM search_index WHERE organization_id = $1 AND updated_at < $2 RETURNING id",
        )
        .bind::<diesel::sql_types::Uuid, _>(organization_id)
        .bind::<Timestamptz, _>(before_date)
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to cleanup old index entries: {e}");
            SearchError::DeleteFailed(e.to_string())
        })?;

        info!("Cleaned up {} old index entries", result);
        Ok(result as i64)
    }

    async fn fetch_emails_for_indexing(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<DocumentToIndex>, SearchError> {
        let mut conn = self.pool.get().map_err(|_| SearchError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, subject, body_text, sender, recipients, created_at
            FROM emails
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT 10000
        "#;

        #[derive(QueryableByName)]
        struct EmailRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            subject: String,
            #[diesel(sql_type = Text)]
            body_text: String,
            #[diesel(sql_type = Text)]
            sender: String,
            #[diesel(sql_type = Nullable<Text>)]
            recipients: Option<String>,
            #[diesel(sql_type = Timestamptz)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<EmailRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .unwrap_or_default();

        let documents = rows
            .into_iter()
            .map(|row| {
                let mut metadata = HashMap::new();
                metadata.insert("sender".to_string(), row.sender.clone());
                if let Some(ref recipients) = row.recipients {
                    metadata.insert("recipients".to_string(), recipients.clone());
                }

                DocumentToIndex {
                    id: row.id,
                    source: SearchSource::Email,
                    organization_id,
                    title: row.subject,
                    content: row.body_text,
                    url: None,
                    metadata,
                    created_at: row.created_at,
                }
            })
            .collect();

        Ok(documents)
    }

    async fn fetch_drive_files_for_indexing(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<DocumentToIndex>, SearchError> {
        let mut conn = self.pool.get().map_err(|_| SearchError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, name, content_text, mime_type, path, created_at
            FROM drive_files
            WHERE organization_id = $1 AND content_text IS NOT NULL
            ORDER BY created_at DESC
            LIMIT 10000
        "#;

        #[derive(QueryableByName)]
        struct DriveRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            name: String,
            #[diesel(sql_type = Nullable<Text>)]
            content_text: Option<String>,
            #[diesel(sql_type = Text)]
            mime_type: String,
            #[diesel(sql_type = Text)]
            path: String,
            #[diesel(sql_type = Timestamptz)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<DriveRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .unwrap_or_default();

        let documents = rows
            .into_iter()
            .filter_map(|row| {
                let content = row.content_text?;
                let mut metadata = HashMap::new();
                metadata.insert("mime_type".to_string(), row.mime_type);
                metadata.insert("path".to_string(), row.path.clone());

                Some(DocumentToIndex {
                    id: row.id,
                    source: SearchSource::Drive,
                    organization_id,
                    title: row.name,
                    content,
                    url: Some(row.path),
                    metadata,
                    created_at: row.created_at,
                })
            })
            .collect();

        Ok(documents)
    }

    async fn fetch_calendar_events_for_indexing(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<DocumentToIndex>, SearchError> {
        let mut conn = self.pool.get().map_err(|_| SearchError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, title, description, location, start_time, created_at
            FROM calendar_events
            WHERE organization_id = $1
            ORDER BY start_time DESC
            LIMIT 10000
        "#;

        #[derive(QueryableByName)]
        struct CalendarRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            title: String,
            #[diesel(sql_type = Nullable<Text>)]
            description: Option<String>,
            #[diesel(sql_type = Nullable<Text>)]
            location: Option<String>,
            #[diesel(sql_type = Timestamptz)]
            start_time: DateTime<Utc>,
            #[diesel(sql_type = Timestamptz)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<CalendarRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .unwrap_or_default();

        let documents = rows
            .into_iter()
            .map(|row| {
                let mut metadata = HashMap::new();
                if let Some(ref loc) = row.location {
                    metadata.insert("location".to_string(), loc.clone());
                }
                metadata.insert("start_time".to_string(), row.start_time.to_rfc3339());

                let content = format!(
                    "{}\n{}",
                    row.description.as_deref().unwrap_or(""),
                    row.location.as_deref().unwrap_or("")
                );

                DocumentToIndex {
                    id: row.id,
                    source: SearchSource::Calendar,
                    organization_id,
                    title: row.title,
                    content,
                    url: None,
                    metadata,
                    created_at: row.created_at,
                }
            })
            .collect();

        Ok(documents)
    }

    async fn fetch_tasks_for_indexing(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<DocumentToIndex>, SearchError> {
        let mut conn = self.pool.get().map_err(|_| SearchError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, title, description, status, priority, created_at
            FROM tasks
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT 10000
        "#;

        #[derive(QueryableByName)]
        struct TaskRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            title: String,
            #[diesel(sql_type = Nullable<Text>)]
            description: Option<String>,
            #[diesel(sql_type = Text)]
            status: String,
            #[diesel(sql_type = Nullable<Text>)]
            priority: Option<String>,
            #[diesel(sql_type = Timestamptz)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<TaskRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .unwrap_or_default();

        let documents = rows
            .into_iter()
            .map(|row| {
                let mut metadata = HashMap::new();
                metadata.insert("status".to_string(), row.status);
                if let Some(ref pri) = row.priority {
                    metadata.insert("priority".to_string(), pri.clone());
                }

                DocumentToIndex {
                    id: row.id,
                    source: SearchSource::Tasks,
                    organization_id,
                    title: row.title,
                    content: row.description.unwrap_or_default(),
                    url: None,
                    metadata,
                    created_at: row.created_at,
                }
            })
            .collect();

        Ok(documents)
    }
}
