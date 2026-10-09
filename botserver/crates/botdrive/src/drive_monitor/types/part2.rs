use super::*;

impl DriveMonitor {
    pub async fn start_monitoring(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::trace!("DriveMonitor monitoring started for bucket: {}", self.bucket_name);

        loop {
            if self.is_processing.load(Ordering::Relaxed) {
                log::trace!("DriveMonitor still processing, skipping iteration");
            } else {
                self.is_processing.store(true, Ordering::Relaxed);
                if let Err(e) = self.scan_bucket().await {
                    log::error!("Failed to scan bucket {}: {}", self.bucket_name, e);
                    self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.consecutive_failures.store(0, Ordering::Relaxed);
                }
                self.is_processing.store(false, Ordering::Relaxed);
            }
            let backoff = self.calculate_backoff();
            tokio::time::sleep(backoff).await;
        }
    }

    async fn scan_bucket(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::trace!("DriveMonitor: Starting scan of bucket {}", self.bucket_name);
        let start = std::time::Instant::now();

        // TODO(#506): Auto-create bot here — se o bucket {bot}.gbai existe no MinIO
        // mas não há registro na tabela `bots`, criar automaticamente.
        // Usar: ensure_bot_exists(bot_name) → INSERT INTO bots (id, name, ...)
        // Isto garante que bots criados via upload direto ao MinIO são auto-registrados.

        if let Some(s3) = &self.state.drive {
            let list_prefix = self.s3_prefix.as_deref();
            match s3.list_objects_with_metadata(&self.bucket_name, list_prefix).await {
                Ok(objects) => {
                    log::trace!("Found {} objects in bucket {} (prefix: {:?})", objects.len(), self.bucket_name, list_prefix);

                    let bot_name = &self.bot_name;

                    // Auto-create bot in database if not exists (Issue #506)
                    // First check if bot already exists by name
                    let already_exists = if let Ok(mut check_conn) = self.state.conn.get() {
                        use diesel::RunQueryDsl;
                        diesel::sql_query("SELECT 1 FROM bots WHERE name = $1")
                            .bind::<diesel::sql_types::Text, _>(bot_name)
                            .execute(&mut check_conn)
                            .unwrap_or(0) > 0
                    } else {
                        false
                    };

                    if !already_exists {
                        if let Ok(mut conn) = self.state.conn.get() {
                            use diesel::RunQueryDsl;
                            // #1289 — org scope: bots.org_id is NOT NULL. Resolve
                            // the default org like the bootstrap path (any existing
                            // bot's org → bootstrap default UUID) so a monitor for a
                            // drive-only bot does not crash on every scan.
                            #[derive(diesel::QueryableByName)]
                            #[diesel(check_for_backend(diesel::pg::Pg))]
                            struct OrgId {
                                #[diesel(sql_type = diesel::sql_types::Uuid)]
                                org_id: uuid::Uuid,
                            }
                            let org_id: uuid::Uuid = diesel::sql_query(
                                "SELECT org_id FROM bots WHERE org_id IS NOT NULL ORDER BY created_at ASC LIMIT 1",
                            )
                            .get_result::<OrgId>(&mut conn)
                            .map(|r| r.org_id)
                            .unwrap_or_else(|_| uuid::Uuid::from_u128(0xf47ac10b58cc4372a5670e02b2c3d479));
                            if let Err(e) = diesel::sql_query(
                                "INSERT INTO bots (id, name, org_id, llm_provider, llm_config, context_provider, context_config, is_public, created_at, updated_at) \
                                 VALUES ($1, $2, $3, 'openai', '{}', 'openai', '{}', true, NOW(), NOW()) ON CONFLICT (id) DO NOTHING"
                            )
                            .bind::<diesel::sql_types::Uuid, _>(self.bot_id)
                            .bind::<diesel::sql_types::Text, _>(bot_name)
                            .bind::<diesel::sql_types::Uuid, _>(org_id)
                            .execute(&mut conn) {
                                log::error!("Failed to auto-create bot {} in database: {}", bot_name, e);
                            } else {
                                log::trace!("DriveMonitor checked/created bot {} (id: {})", bot_name, self.bot_id);
                            }
                        }
                    } else {
                        log::trace!("DriveMonitor: bot {} already exists in database, skipping auto-create", bot_name);
                    }

                    let current_keys: Vec<String> = objects.iter().map(|o| self.strip_prefix(&o.key).to_string()).collect();

        for obj in &objects {
            if obj.key.ends_with('/') {
                log::debug!("Skipping directory entry: {}", obj.key);
                continue;
            }

            let file_type = classify_file(&obj.key);
                        let relative_key = self.strip_prefix(&obj.key);
                        let full_key = format!("{}.gbai/{}", bot_name, relative_key);
                        let etag = obj.etag.as_deref().map(normalize_etag);

            let existing = self.file_repo.get_file_state(&full_key);

            // Skip re-indexing files that have failed too many times (prevent infinite loops)
            // Automatically retry after FAIL_RETRY_COOLDOWN hours so transient issues (e.g., disk full) self-heal.
            const MAX_FAIL_COUNT: i32 = 5;
            const FAIL_RETRY_COOLDOWN_HOURS: i64 = 1;
            if let Some(prev) = &existing {
                if prev.fail_count >= MAX_FAIL_COUNT {
                    if let Some(last_failed) = prev.last_failed_at {
                        let elapsed = Utc::now() - last_failed;
                        if elapsed < chrono::Duration::hours(FAIL_RETRY_COOLDOWN_HOURS) {
                            log::debug!(
                                "Skipping {} — failed {} times, last retry in {} min.",
                                full_key, prev.fail_count,
                                (chrono::Duration::hours(FAIL_RETRY_COOLDOWN_HOURS) - elapsed).num_minutes()
                            );
                            continue;
                        }
                        log::info!(
                            "Retrying {} after cooldown (last failed {} min ago).",
                            full_key, elapsed.num_minutes()
                        );
                        let _ = self.file_repo.reset_file_fail_count(&full_key);
                    }
                }
            }

            let needs_reindex = match &existing {
                Some(prev) if prev.indexed && prev.etag.as_deref() == etag.as_deref() => false,
                Some(prev) if prev.indexed && prev.etag.as_deref() != etag.as_deref() => {
                    log::info!("ETag changed for {}, will reindex", full_key);
                    true
                }
                Some(prev) if !prev.indexed && prev.etag.as_deref() == etag.as_deref() => {
                    log::debug!("{} unchanged but not yet indexed, will index", full_key);
                    true
                }
                Some(_) => true,
                None => true,
            };

            let etag_changed = existing.as_ref().is_none_or(|prev| prev.etag.as_deref() != etag.as_deref());

        let is_unchanged = existing.is_some() && !etag_changed && !needs_reindex;
        if !is_unchanged {
            let branch_id = self.resolve_branch_id();
            let was_new = existing.is_none();
            match self.file_repo.upsert_file(
                &full_key,
                file_type,
                etag.clone(),
                None,
                branch_id,
            ) {
                Ok(_) => {
                    if etag_changed || was_new {
                        log::info!("DriveMonitor: {} drive_files for: {} ({})",
                            if was_new { "Created" } else { "Updated" },
                            full_key, file_type
                        );
                    }
                    // #1474 — every bot is git-owned. A bot's `.gbdialog` (scripts) and
                    // `.gbot` (config + channel prompts) live in its Forgejo
                    // repository, which is the single source of truth, and
                    // `git_bot_monitor` is the only producer of script sources
                    // and the only compile trigger. Drive still walks the
                    // bucket so the inventory (search, KB indexing) and
                    // `.gbot` config stay current, but it no longer downloads a
                    // `.bas` into the work dir and no longer enqueues one for
                    // compilation. `.gbkb` and `.gbdrive` are untouched.
                    //
                    // Previously the `bas` arm also acted as an operational
                    // fallback for a repo that lags Drive (#1501: "a media-filing
                    // bot froze for hours"). That is exactly what this epic
                    // retires; a lagging repo is now surfaced as a missing
                    // compile rather than silently papered over from Drive.
                    if file_type == "bas" {
                        log::debug!(
                            "DriveMonitor: inventorying {} (git-owned source; not downloaded or compiled from Drive)",
                            full_key
                        );
                    } else if file_type == "prompt" {
                        self.sync_gbot_to_work(bot_name, &obj.key, etag.clone()).await;
                    } else if file_type != "kb" && file_type != "config" {
                        let _ = self.file_repo.mark_indexed(&full_key, etag.clone());
                    }
                }
                Err(e) => log::error!("Failed to upsert {}: {}", full_key, e),
            }
        }

            if needs_reindex && file_type == "kb" {
                #[cfg(any(feature = "research", feature = "llm"))]
                {
                    if let Some(parsed) = parse_kb_path(&obj.key) {
                        let folder_key = format!("kb_{}", parsed.kb_name);
                        let already_indexed = self.kb_indexed_folders.read().await.contains(&folder_key);
                        if !already_indexed {
                            self.pending_kb_index.write().await.insert(folder_key);
                        }
                    }
                    self.index_kb_file(bot_name, &full_key, &obj.key, etag.clone()).await;
                }
            }

        if file_type == "config" && needs_reindex {
            self.sync_bot_config(bot_name, &obj.key, etag.clone()).await;
        }
                    }

        self.handle_deleted_files(bot_name, &current_keys);
        }
        Err(e) => {
            log::error!("Failed to list objects in {}: {}", self.bucket_name, e);
        }
        }
        } else {
            log::warn!("S3 client not available for bucket scan");
        }

        let elapsed = start.elapsed();
        log::trace!("DriveMonitor: Completed scan of {} in {:.2?}", self.bucket_name, elapsed);
        Ok(())
    }

    /// Reform #1501 — true when `git_bot_monitor` feeds this bot's sources from
    /// its repository. Same condition the monitor uses: a bot-kind vibe project
    /// of the branch in git mode.
    pub(super) fn is_git_owned(&self) -> bool {
        let Ok(mut conn) = self.state.conn.get() else {
            return false;
        };
        // `execute` returns the matched row count, so the query needs no
        // row struct (a `QueryableByName` field that is never read in Rust
        // trips the dead-code lint).
        diesel::sql_query(
            "SELECT vp.name \
             FROM vibe_projects vp \
             JOIN branches br ON br.id = vp.branch_id \
             WHERE vp.project_type = 'bot' AND vp.source_control = 'git' \
               AND br.slug = $1 AND vp.name = $2 LIMIT 1",
        )
        .bind::<diesel::sql_types::Text, _>(&self.branch_slug)
        .bind::<diesel::sql_types::Text, _>(&self.bot_name)
        .execute(&mut conn)
        .map(|matched| matched > 0)
        .unwrap_or(false)
    }

    pub(super) fn handle_deleted_files(&self, bot_name: &str, current_keys: &[String]) {
        // Reform #1501 — a git-owned bot's `.gbdialog`/`.gbot` rows are
        // materialized from its repository and intentionally have no Drive
        // object. `git_bot_monitor` inserts those rows to trigger a compile
        // from the work copy, and deleting them here won the race: the `.ast`
        // was never rebuilt and the bot kept running a stale script. Source
        // rows are therefore kept for a git-owned bot; every other prefix
        // (`.gbkb`, `.gbdrive`, …) keeps the deletion.
        let git_owned = self.is_git_owned();
        let db_files = self.file_repo.get_all_files_for_bot();
        for db_file in &db_files {
            let s3_key = match db_file.file_path.strip_prefix(&format!("{}.gbai/", bot_name)) {
                Some(k) => k,
                None => continue,
            };
            if !current_keys.iter().any(|k| k == s3_key) {
                if git_owned && is_source_prefix(s3_key) {
                    continue;
                }
                log::info!("File deleted from S3: {} (was in DB)", db_file.file_path);

                if db_file.file_type == "kb" {
                    #[cfg(any(feature = "research", feature = "llm"))]
                    {
                        self.delete_kb_file_vectors(bot_name, &db_file.file_path, s3_key);
                    }
                }

                if let Err(e) = self.file_repo.delete_file(&db_file.file_path) {
                    log::error!("Failed to delete drive_files entry for {}: {}", db_file.file_path, e);
                }
            }
        }
    }
}
