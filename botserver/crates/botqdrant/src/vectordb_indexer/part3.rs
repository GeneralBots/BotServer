use super::*;

impl VectorDBIndexer {
    async fn index_user_files(&self, user_id: Uuid) -> Result<()> {
        let jobs = self.jobs.read().await;
        let job = jobs
            .get(&user_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found"))?;

        let Some(drive_db) = &job.drive_db else {
            warn!("Drive vector DB not initialized for user {}", user_id);
            return Ok(());
        };

        match self.get_unindexed_files(user_id).await {
            Ok(files) => {
                if files.is_empty() {
                    return Ok(());
                }

                info!("Indexing {} files for user {}", files.len(), user_id);

                for chunk in files.chunks(self.batch_size) {
                    for file in chunk {
                        let mime_type = file.mime_type.as_deref().unwrap_or("");
                        if !FileContentExtractor::should_index(mime_type, file.file_size) {
                            continue;
                        }

                        let text = format!(
                            "File: {}\nType: {}\n\n{}",
                            file.file_name, file.file_type, file.content_text
                        );

                        match self
                            .embedding_generator
                            .generate_text_embedding(&text)
                            .await
                        {
                            Ok(embedding) => {
                                if let Err(e) = drive_db.index_file(file, embedding).await {
                                    log::error!("Failed to index file {}: {}", file.id, e);
                                } else {
                                    info!(" Indexed file: {}", file.file_name);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to generate embedding for file {}: {}", file.id, e);
                            }
                        }
                    }

                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                log::error!("Failed to get unindexed files for user {}: {}", user_id, e);
            }
        }

        Ok(())
    }

    #[cfg(feature = "mail")]
    async fn get_user_email_accounts(&self, user_id: Uuid) -> Result<Vec<String>> {
        let pool = self.db_pool.clone();

        tokio::task::spawn_blocking(move || {
            use diesel::prelude::*;

            let mut db_conn = pool.get()?;

            #[derive(diesel::QueryableByName)]
            struct AccountIdRow {
                #[diesel(sql_type = diesel::sql_types::Text)]
                id: String,
            }

            let results: Vec<String> = diesel::sql_query(
                "SELECT id::text FROM user_email_accounts WHERE user_id = $1 AND is_active = true",
            )
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .load::<AccountIdRow>(&mut db_conn)?
            .into_iter()
            .map(|row| row.id)
            .collect();

            Ok::<_, anyhow::Error>(results)
        })
        .await?
    }

    #[cfg(feature = "mail")]
    async fn get_unindexed_emails(
        &self,
        user_id: Uuid,
        account_id: &str,
    ) -> Result<Vec<EmailDocument>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_pool.clone();
        let account_id = account_id.to_string();

        let results = tokio::task::spawn_blocking(move || {
            use diesel::prelude::*;
            let mut conn = pool.get()?;

            #[derive(diesel::QueryableByName)]
            struct EmailRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                message_id: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                subject: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                from_address: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                to_addresses: String,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                body_text: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                body_html: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                received_at: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Text)]
                folder: String,
            }

            let query = r"
                SELECT e.id, e.message_id, e.subject, e.from_address, e.to_addresses,
                       e.body_text, e.body_html, e.received_at, e.folder
                FROM emails e
                LEFT JOIN email_index_status eis ON e.id = eis.email_id
                WHERE e.user_id = $1
                  AND e.account_id = $2
                  AND (eis.indexed_at IS NULL OR eis.needs_reindex = true)
                ORDER BY e.received_at DESC
                LIMIT 100
            ";

            let rows: Vec<EmailRow> = diesel::sql_query(query)
                .bind::<diesel::sql_types::Uuid, _>(user_id)
                .bind::<diesel::sql_types::Text, _>(&account_id)
                .load(&mut conn)
                .unwrap_or_default();

            let emails: Vec<EmailDocument> = rows
                .into_iter()
                .map(|row| EmailDocument {
                    id: row.id.to_string(),
                    account_id: account_id.clone(),
                    from_email: row.from_address.clone(),
                    from_name: row.from_address,
                    to_email: row.to_addresses,
                    subject: row.subject,
                    body_text: row
                        .body_html
                        .unwrap_or_else(|| row.body_text.unwrap_or_default()),
                    date: row.received_at,
                    folder: row.folder,
                    has_attachments: false,
                    thread_id: Some(row.message_id),
                })
                .collect();

            Ok::<_, anyhow::Error>(emails)
        })
        .await??;

        Ok(results)
    }

    async fn get_unindexed_files(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<FileDocument>, Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.db_pool.clone();

        let results = tokio::task::spawn_blocking(move || {
            use diesel::prelude::*;
            let mut conn = pool.get()?;

            #[derive(diesel::QueryableByName)]
            struct FileRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                file_path: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                file_name: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                file_type: String,
                #[diesel(sql_type = diesel::sql_types::BigInt)]
                file_size: i64,
                #[diesel(sql_type = diesel::sql_types::Text)]
                bucket: String,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                mime_type: Option<String>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                created_at: DateTime<Utc>,
                #[diesel(sql_type = diesel::sql_types::Timestamptz)]
                modified_at: DateTime<Utc>,
            }

            let query = r"
                SELECT f.id, f.file_path, f.file_name, f.file_type, f.file_size,
                       f.bucket, f.mime_type, f.created_at, f.modified_at
                FROM user_files f
                LEFT JOIN file_index_status fis ON f.id = fis.file_id
                WHERE f.user_id = $1
                  AND (fis.indexed_at IS NULL OR fis.needs_reindex = true)
                ORDER BY f.modified_at DESC
                LIMIT 100
            ";

            let rows: Vec<FileRow> = diesel::sql_query(query)
                .bind::<diesel::sql_types::Uuid, _>(user_id)
                .load(&mut conn)
                .unwrap_or_default();

            let files: Vec<FileDocument> = rows
                .into_iter()
                .map(|row| FileDocument {
                    id: row.id.to_string(),
                    file_path: row.file_path,
                    file_name: row.file_name,
                    file_type: row.file_type,
                    file_size: row.file_size as u64,
                    bucket: row.bucket,
                    content_text: String::new(),
                    content_summary: None,
                    created_at: row.created_at,
                    modified_at: row.modified_at,
                    indexed_at: Utc::now(),
                    mime_type: row.mime_type,
                    tags: Vec::new(),
                })
                .collect();

            Ok::<_, anyhow::Error>(files)
        })
        .await??;

        Ok(results)
    }

    pub async fn get_user_stats(&self, user_id: Uuid) -> Option<IndexingStats> {
        let jobs = self.jobs.read().await;
        jobs.get(&user_id).map(|job| job.stats.clone())
    }

    pub async fn get_overall_stats(&self) -> IndexingStats {
        let jobs = self.jobs.read().await;

        let mut total_stats = IndexingStats {
            emails_indexed: 0,
            files_indexed: 0,
            emails_pending: 0,
            files_pending: 0,
            last_run: None,
            errors: 0,
        };

        for job in jobs.values() {
            total_stats.emails_indexed += job.stats.emails_indexed;
            total_stats.files_indexed += job.stats.files_indexed;
            total_stats.emails_pending += job.stats.emails_pending;
            total_stats.files_pending += job.stats.files_pending;
            total_stats.errors += job.stats.errors;

            if let Some(last_run) = job.stats.last_run {
                if total_stats.last_run.is_none_or(|lr| lr < last_run) {
                    total_stats.last_run = Some(last_run);
                }
            }
        }

        total_stats
    }

    pub async fn pause_user_indexing(&self, user_id: Uuid) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&user_id) {
            job.status = IndexingStatus::Paused;
            info!("⏸️  Paused indexing for user {}", user_id);
        }
        Ok(())
    }

    pub async fn resume_user_indexing(&self, user_id: Uuid) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&user_id) {
            job.status = IndexingStatus::Idle;
            info!("▶️  Resumed indexing for user {}", user_id);
        }
        Ok(())
    }

    pub async fn trigger_user_indexing(&self, user_id: Uuid, bot_id: Uuid) -> Result<()> {
        info!(" Triggering immediate indexing for user {}", user_id);
        self.index_user_data(user_id, bot_id).await
    }
}
