use super::*;

impl SearchService {
    pub(crate) async fn fetch_transcriptions_for_indexing(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<DocumentToIndex>, SearchError> {
        let mut conn = self.pool.get().map_err(|_| SearchError::DatabaseConnection)?;

        let sql = r#"
            SELECT mt.id, m.title as meeting_title, mt.content, mt.speaker, m.started_at as created_at
            FROM meeting_transcriptions mt
            JOIN meetings m ON m.id = mt.meeting_id
            WHERE m.organization_id = $1
            ORDER BY m.started_at DESC
            LIMIT 10000
        "#;

        #[derive(QueryableByName)]
        struct TranscriptionRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            meeting_title: String,
            #[diesel(sql_type = Text)]
            content: String,
            #[diesel(sql_type = Nullable<Text>)]
            speaker: Option<String>,
            #[diesel(sql_type = Timestamptz)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<TranscriptionRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .unwrap_or_default();

        let documents = rows
            .into_iter()
            .map(|row| {
                let mut metadata = HashMap::new();
                if let Some(ref speaker) = row.speaker {
                    metadata.insert("speaker".to_string(), speaker.clone());
                }

                DocumentToIndex {
                    id: row.id,
                    source: SearchSource::Transcription,
                    organization_id,
                    title: row.meeting_title,
                    content: row.content,
                    url: None,
                    metadata,
                    created_at: row.created_at,
                }
            })
            .collect();

        Ok(documents)
    }

    pub(crate) async fn fetch_chat_messages_for_indexing(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<DocumentToIndex>, SearchError> {
        let mut conn = self.pool.get().map_err(|_| SearchError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, content, sender_name, channel_name, created_at
            FROM chat_messages
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT 10000
        "#;

        #[derive(QueryableByName)]
        struct ChatRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            content: String,
            #[diesel(sql_type = Nullable<Text>)]
            sender_name: Option<String>,
            #[diesel(sql_type = Nullable<Text>)]
            channel_name: Option<String>,
            #[diesel(sql_type = Timestamptz)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<ChatRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .unwrap_or_default();

        let documents = rows
            .into_iter()
            .map(|row| {
                let mut metadata = HashMap::new();
                if let Some(ref sender) = row.sender_name {
                    metadata.insert("sender".to_string(), sender.clone());
                }
                if let Some(ref channel) = row.channel_name {
                    metadata.insert("channel".to_string(), channel.clone());
                }

                let title = row.channel_name.unwrap_or_else(|| "Chat".to_string());

                DocumentToIndex {
                    id: row.id,
                    source: SearchSource::Chat,
                    organization_id,
                    title,
                    content: row.content,
                    url: None,
                    metadata,
                    created_at: row.created_at,
                }
            })
            .collect();

        Ok(documents)
    }

    pub(crate) async fn fetch_contacts_for_indexing(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<DocumentToIndex>, SearchError> {
        let mut conn = self.pool.get().map_err(|_| SearchError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, name, email, phone, company, notes, created_at
            FROM contacts
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT 10000
        "#;

        #[derive(QueryableByName)]
        struct ContactRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            name: String,
            #[diesel(sql_type = Nullable<Text>)]
            email: Option<String>,
            #[diesel(sql_type = Nullable<Text>)]
            phone: Option<String>,
            #[diesel(sql_type = Nullable<Text>)]
            company: Option<String>,
            #[diesel(sql_type = Nullable<Text>)]
            notes: Option<String>,
            #[diesel(sql_type = Timestamptz)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<ContactRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .unwrap_or_default();

        let documents = rows
            .into_iter()
            .map(|row| {
                let mut metadata = HashMap::new();
                if let Some(ref email) = row.email {
                    metadata.insert("email".to_string(), email.clone());
                }
                if let Some(ref phone) = row.phone {
                    metadata.insert("phone".to_string(), phone.clone());
                }
                if let Some(ref company) = row.company {
                    metadata.insert("company".to_string(), company.clone());
                }

                let content = format!(
                    "{} {} {} {}",
                    row.email.as_deref().unwrap_or(""),
                    row.phone.as_deref().unwrap_or(""),
                    row.company.as_deref().unwrap_or(""),
                    row.notes.as_deref().unwrap_or("")
                );

                DocumentToIndex {
                    id: row.id,
                    source: SearchSource::Contacts,
                    organization_id,
                    title: row.name,
                    content,
                    url: None,
                    metadata,
                    created_at: row.created_at,
                }
            })
            .collect();

        Ok(documents)
    }

    pub(crate) async fn fetch_notes_for_indexing(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<DocumentToIndex>, SearchError> {
        let mut conn = self.pool.get().map_err(|_| SearchError::DatabaseConnection)?;

        let sql = r#"
            SELECT id, title, content, created_at
            FROM notes
            WHERE organization_id = $1
            ORDER BY created_at DESC
            LIMIT 10000
        "#;

        #[derive(QueryableByName)]
        struct NoteRow {
            #[diesel(sql_type = Text)]
            id: String,
            #[diesel(sql_type = Text)]
            title: String,
            #[diesel(sql_type = Text)]
            content: String,
            #[diesel(sql_type = Timestamptz)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<NoteRow> = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Uuid, _>(organization_id)
            .load(&mut conn)
            .unwrap_or_default();

        let documents = rows
            .into_iter()
            .map(|row| DocumentToIndex {
                id: row.id,
                source: SearchSource::Notes,
                organization_id,
                title: row.title,
                content: row.content,
                url: None,
                metadata: HashMap::new(),
                created_at: row.created_at,
            })
            .collect();

        Ok(documents)
    }

    pub(super) fn sanitize_query(&self, query: &str) -> String {
        query
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
            .collect::<String>()
            .trim()
            .to_string()
    }

    pub(super) fn build_date_filter(
        &self,
        from_date: &Option<DateTime<Utc>>,
        to_date: &Option<DateTime<Utc>>,
    ) -> String {
        let mut filter = String::new();
        if let Some(from) = from_date {
            filter.push_str(&format!(" AND created_at >= '{}'", from.to_rfc3339()));
        }
        if let Some(to) = to_date {
            filter.push_str(&format!(" AND created_at <= '{}'", to.to_rfc3339()));
        }
        filter
    }

    pub(super) fn parse_source(&self, source: &str) -> SearchSource {
        match source {
            "email" => SearchSource::Email,
            "drive" => SearchSource::Drive,
            "calendar" => SearchSource::Calendar,
            "tasks" => SearchSource::Tasks,
            "transcription" => SearchSource::Transcription,
            "chat" => SearchSource::Chat,
            "contacts" => SearchSource::Contacts,
            "notes" => SearchSource::Notes,
            _ => SearchSource::Drive,
        }
    }
}
