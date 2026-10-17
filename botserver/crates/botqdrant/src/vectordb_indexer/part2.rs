use super::*;

impl VectorDBIndexer {
    pub fn new(
        db_pool: DbPool,
        work_root: PathBuf,
        qdrant_url: String,
        llm_endpoint: String,
    ) -> Self {
        Self {
            db_pool,
            work_root,
            qdrant_url,
            embedding_generator: Arc::new(EmbeddingGenerator::new(llm_endpoint)),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            interval_seconds: 300,
            batch_size: 10,
        }
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            warn!("Vector DB indexer already running");
            return Ok(());
        }
        *running = true;
        drop(running);

        info!(" Starting Vector DB Indexer background service");

        let indexer = Arc::clone(&self);
        tokio::spawn(async move {
            indexer.run_indexing_loop().await;
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("🛑 Stopping Vector DB Indexer");
    }

    async fn run_indexing_loop(self: Arc<Self>) {
        loop {
            {
                let running = self.running.read().await;
                if !*running {
                    break;
                }
            }

            info!(" Running vector DB indexing cycle...");

            match self.get_active_users().await {
                Ok(users) => {
                    info!("Found {} active users to index", users.len());

                    for (user_id, bot_id) in users {
                        if let Err(e) = self.index_user_data(user_id, bot_id).await {
                            log::error!("Failed to index user {}: {}", user_id, e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get active users: {}", e);
                }
            }

            info!(" Indexing cycle complete");

            sleep(Duration::from_secs(self.interval_seconds)).await;
        }

        info!("Vector DB Indexer stopped");
    }

    async fn get_active_users(&self) -> Result<Vec<(Uuid, Uuid)>> {
        let pool = self.db_pool.clone();

        tokio::task::spawn_blocking(move || {
            use botcore::shared::models::schema::user_sessions::dsl::*;
            use diesel::prelude::*;

            let mut db_conn = pool.get()?;

            // `user_sessions.user_id` is VARCHAR since 6.5.30-user-sessions-schema-fix;
            // the indexer only handles UUID-shaped ids (the Zitadel users).
            let results: Vec<(String, Uuid)> = user_sessions
                .select((user_id, bot_id))
                .distinct()
                .load(&mut db_conn)?;

            let results = results
                .into_iter()
                .filter_map(|(uid, bid)| match Uuid::parse_str(&uid) {
                    Ok(uid) => Some((uid, bid)),
                    Err(e) => {
                        warn!("Skipping session user_id {uid:?}: {e}");
                        None
                    }
                })
                .collect::<Vec<(Uuid, Uuid)>>();

            Ok::<_, anyhow::Error>(results)
        })
        .await?
    }

    pub(crate) async fn index_user_data(&self, user_id: Uuid, bot_id: Uuid) -> Result<()> {
        info!("Indexing user: {} (bot: {})", user_id, bot_id);

        let mut jobs = self.jobs.write().await;
        let job = jobs.entry(user_id).or_insert_with(|| {
            let workspace = UserWorkspace::new(self.work_root.clone(), &bot_id, &user_id);
            info!("User workspace path: {}", workspace.get_path().display());

        UserIndexingJob {
            user_id,
            bot_id,
            workspace,
            #[cfg(feature = "mail")]
            email_db: None,
            drive_db: None,
                stats: IndexingStats {
                    emails_indexed: 0,
                    files_indexed: 0,
                    emails_pending: 0,
                    files_pending: 0,
                    last_run: None,
                    errors: 0,
                },
                status: IndexingStatus::Idle,
            }
        });

        if job.status == IndexingStatus::Running {
            warn!(
                "Job already running for user {} (bot: {})",
                job.user_id, job.bot_id
            );
            return Ok(());
        }

        job.status = IndexingStatus::Running;

    #[cfg(feature = "mail")]
    if job.email_db.is_none() {
            let mut email_db =
                UserEmailVectorDB::new(user_id, bot_id);
            if let Err(e) = email_db.initialize(&self.qdrant_url).await {
                warn!(
                    "Failed to initialize email vector DB for user {}: {}",
                    user_id, e
                );
            } else {
                job.email_db = Some(email_db);
            }
        }

        if job.drive_db.is_none() {
            let mut drive_db =
                UserDriveVectorDB::new(user_id, bot_id, job.workspace.drive_vectordb().into());
            if let Err(e) = drive_db.initialize(&self.qdrant_url).await {
                warn!(
                    "Failed to initialize drive vector DB for user {}: {}",
                    user_id, e
                );
            } else {
                job.drive_db = Some(drive_db);
            }
        }

        drop(jobs);

        #[cfg(feature = "mail")]
        if let Err(e) = self.index_user_emails(user_id).await {
            log::error!("Failed to index emails for user {}: {}", user_id, e);
        }

        if let Err(e) = self.index_user_files(user_id).await {
            log::error!("Failed to index files for user {}: {}", user_id, e);
        }

        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&user_id) {
            job.status = IndexingStatus::Idle;
            job.stats.last_run = Some(Utc::now());
        }

        Ok(())
    }

    #[cfg(feature = "mail")]
    async fn index_user_emails(&self, user_id: Uuid) -> Result<()> {
        let jobs = self.jobs.read().await;
        let job = jobs
            .get(&user_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found"))?;

        let Some(email_db) = &job.email_db else {
            warn!("Email vector DB not initialized for user {}", user_id);
            return Ok(());
        };

        let accounts = self.get_user_email_accounts(user_id).await?;

        info!(
            "Found {} email accounts for user {}",
            accounts.len(),
            user_id
        );

        for account_id in accounts {
            match self.get_unindexed_emails(user_id, &account_id).await {
                Ok(emails) => {
                    if emails.is_empty() {
                        continue;
                    }

                    info!(
                        "Indexing {} emails for account {}",
                        emails.len(),
                        account_id
                    );

                    for chunk in emails.chunks(self.batch_size) {
                        for email in chunk {
                            let text = format!(
                                "From: {} <{}>\nSubject: {}\n\n{}",
                                email.from_name, email.from_email, email.subject, email.body_text
                            );
                            // Indexed mail is arbitrary text; cut on a char boundary.
                            let text = botlib::text::truncate_at_char_boundary(&text, 8000);

                            match self.embedding_generator.generate_text_embedding(text).await {
                                Ok(embedding) => {
                                    if let Err(e) = email_db.index_email(email, embedding).await {
                                        log::error!("Failed to index email {}: {}", email.id, e);
                                    } else {
                                        info!(" Indexed email: {}", email.subject);
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to generate embedding for email {}: {}",
                                        email.id, e
                                    );
                                }
                            }
                        }

                        sleep(Duration::from_millis(100)).await;
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to get unindexed emails for account {}: {}",
                        account_id, e
                    );
                }
            }
        }

        Ok(())
    }
}
