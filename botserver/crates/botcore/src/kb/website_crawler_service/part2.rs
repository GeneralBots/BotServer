use super::*;

impl WebsiteCrawlerService {
    pub(crate) async fn do_crawl_website(
        website: WebsiteCrawlRecord,
        db_pool: DbPool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        trace!("Starting crawl for website: {}", website.url);

        let config_manager = ConfigManager::new(db_pool.clone());

        let website_max_depth = config_manager
            .get_bot_config_value(&website.bot_id, "website-max-depth")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(website.max_depth as usize);

        let website_max_pages = config_manager
            .get_bot_config_value(&website.bot_id, "website-max-pages")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(website.max_pages as usize);

        // Strict limits to prevent memory exhaustion
        let max_depth = std::cmp::min(website_max_depth, 3); // Max 3 levels deep
        let max_pages = std::cmp::min(website_max_pages, 50); // Max 50 pages

        let mut config = WebsiteCrawlConfig {
            url: website.url.clone(),
            max_depth,
            max_pages,
            crawl_delay_ms: 1000, // Increased delay to be respectful
            expires_policy: website.expires_policy.clone(),
            refresh_policy: website.refresh_policy.clone(),
            last_crawled: None,
            next_crawl: None,
        };

        let mut crawler = WebCrawler::new(config.clone());

        match crawler.crawl().await {
            Ok(pages) => {
                trace!("Crawled {} pages from {}", pages.len(), website.url);

                let mut conn = db_pool.get()?;
                #[derive(QueryableByName)]
                struct BotNameResult {
                    #[diesel(sql_type = diesel::sql_types::Text)]
                    name: String,
                }

                let bot_name: String = diesel::sql_query("SELECT name FROM bots WHERE id = $1")
                    .bind::<diesel::sql_types::Uuid, _>(&website.bot_id)
                    .get_result::<BotNameResult>(&mut conn)
                    .map(|r| r.name)?;

                let kb_name = format!("website_{}", sanitize_url_for_kb(&website.url));

                let work_path = std::path::PathBuf::from(crate::shared::utils::get_work_path())
                    .join(&bot_name)
                    .join(format!("{}.gbkb", bot_name))
                    .join(&kb_name);

                tokio::fs::create_dir_all(&work_path).await?;

                // Load embedding config: DB settings + Vault API key at gbo/{bot_name}.
                let embedding_config = embedding_config_with_vault(&db_pool, &website.bot_id, &bot_name).await;
                info!("Using embedding URL: {} for bot {}", embedding_config.embedding_url, bot_name);

            // Create bot-specific KB indexer with correct embedding config
            let qdrant_config = QdrantConfig::from_config(db_pool.clone(), &website.bot_id);
            let bot_indexer = KbIndexer::new(embedding_config, qdrant_config);

                // Process pages in small batches to prevent memory exhaustion
                const BATCH_SIZE: usize = 5;
                let total_pages = pages.len();

                for (batch_idx, batch) in pages.chunks(BATCH_SIZE).enumerate() {
                    trace!("Processing batch {} of {} pages", batch_idx + 1, total_pages.div_ceil(BATCH_SIZE));

                    for (idx, page) in batch.iter().enumerate() {
                        let global_idx = batch_idx * BATCH_SIZE + idx;
                        let filename = format!("page_{:04}.txt", global_idx);
                        let filepath = work_path.join(&filename);

                        // Limit content size to prevent memory issues
                        let content_preview = if page.content.len() > 10_000 {
                            format!("{}\n\n[Content truncated - original size: {} chars]",
                                   &page.content[..10_000], page.content.len())
                        } else {
                            page.content.clone()
                        };

                        let content = format!(
                            "URL: {}\nTitle: {}\nCrawled: {}\n\n{}",
                            page.url,
                            page.title.as_deref().unwrap_or("Untitled"),
                            page.crawled_at,
                            content_preview
                        );

                        tokio::fs::write(&filepath, content).await?;
                    }

                    // Process this batch immediately to free memory
                    if batch_idx == 0 || (batch_idx + 1) % 2 == 0 {
                        // Index every 2 batches to prevent memory buildup
                        match bot_indexer.index_kb_folder(website.bot_id, &bot_name, &kb_name, &work_path).await {
                            Ok(result) => trace!("Indexed batch {} successfully: {} docs, {} chunks",
                                batch_idx + 1, result.documents_processed, result.chunks_indexed),
                            Err(e) => warn!("Failed to index batch {}: {}", batch_idx + 1, e),
                        }

                        // Force memory cleanup
                        tokio::task::yield_now().await;
                    }
                }

                // Final indexing for any remaining content
                bot_indexer
                    .index_kb_folder(website.bot_id, &bot_name, &kb_name, &work_path)
                    .await?;

                config.calculate_next_crawl();

                diesel::sql_query(
                    "UPDATE website_crawls
                     SET last_crawled = NOW(),
                         next_crawl = $1,
                         crawl_status = 1,
                         pages_crawled = $2,
                         error_message = NULL
                     WHERE id = $3",
                )
                .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(
                    config.next_crawl,
                )
                .bind::<diesel::sql_types::Integer, _>(pages.len() as i32)
                .bind::<diesel::sql_types::Uuid, _>(&website.id)
                .execute(&mut conn)?;

                trace!(
                    "Successfully recrawled {}, next crawl: {:?}",
                    website.url, config.next_crawl
                );
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to crawl {}: {}", website.url, e);

                let mut conn = db_pool.get()?;
                diesel::sql_query(
                    "UPDATE website_crawls
                     SET crawl_status = 3,
                         error_message = $1
                     WHERE id = $2",
                )
                .bind::<diesel::sql_types::Text, _>(&e.to_string())
                .bind::<diesel::sql_types::Uuid, _>(&website.id)
                .execute(&mut conn)?;

                Err(e.into())
            }
        }
    }

    pub(super) fn scan_and_register_websites_from_scripts(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        trace!("Scanning .bas files for USE WEBSITE commands");

        // Use the correct work directory path instead of plain "work"
        let work_dir = std::path::PathBuf::from(crate::shared::utils::get_work_path());

        if !work_dir.exists() {
            return Ok(());
        }

        let mut conn = self.db_pool.get()?;

        for entry in std::fs::read_dir(work_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if path.is_dir() && file_name_str.ends_with(".gbai") {
                    let bot_name = file_name_str.replace(".gbai", "");

                // Get bot_id from database
                #[derive(QueryableByName)]
                struct BotIdResult {
                    #[diesel(sql_type = diesel::sql_types::Uuid)]
                    id: uuid::Uuid,
                }

                let bot_id_result: Result<BotIdResult, _> = diesel::sql_query("SELECT id FROM bots WHERE name = $1")
                    .bind::<diesel::sql_types::Text, _>(&bot_name)
                    .get_result(&mut conn);

                let bot_id = match bot_id_result {
                    Ok(result) => result.id,
                    Err(_) => continue, // Skip if bot not found
                };

                // Scan {bot_name}.gbdialog directory for .bas files
                let dialog_dir = path.join(format!("{}.gbdialog", bot_name));
                if dialog_dir.exists() {
                    self.scan_directory_for_websites(&dialog_dir, bot_id, &mut conn)?;
                }
                }
            }
        }

        Ok(())
    }

    pub async fn crawl_single_website(
        &self,
        website: WebsiteCrawlRecord,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::crawl_website(website, self.db_pool.clone(), Arc::clone(&self.active_crawls)).await
    }

    pub(super) fn scan_directory_for_websites(
        &self,
        dir: &std::path::Path,
        bot_id: uuid::Uuid,
        conn: &mut diesel::PgConnection,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let website_regex = regex::Regex::new(r#"(?i)(?:USE\s+WEBSITE\s+"([^"]+)"(?:\s+REFRESH\s+"([^"]+)")?)|(?:USE_WEBSITE\s*\(\s*"([^"]+)"\s*(?:,\s*"([^"]+)"\s*)?\))"#)?;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "bas") {
                let content = std::fs::read_to_string(&path)?;

                for cap in website_regex.captures_iter(&content) {
                    // Extract URL from either capture group 1 (space syntax) or group 3 (function syntax)
                    let url_str = if let Some(url) = cap.get(1) {
                        url.as_str()
                    } else if let Some(url) = cap.get(3) {
                        url.as_str()
                    } else {
                        continue;
                    };

                    // Extract refresh from either capture group 2 (space syntax) or group 4 (function syntax)
                    let refresh_str = if let Some(refresh) = cap.get(2) {
                        refresh.as_str()
                    } else if let Some(refresh) = cap.get(4) {
                        refresh.as_str()
                    } else {
                        "1m"
                    };

                    // Check if already registered
                    let exists = diesel::sql_query(
                        "SELECT COUNT(*) as count FROM website_crawls WHERE bot_id = $1 AND url = $2"
                    )
                    .bind::<diesel::sql_types::Uuid, _>(&bot_id)
                    .bind::<diesel::sql_types::Text, _>(url_str)
                    .get_result::<CountResult>(conn)
                    .map(|r| r.count)
                    .unwrap_or(0);

                    if exists == 0 {
                        trace!("Auto-registering website {} for bot {} with refresh: {}", url_str, bot_id, refresh_str);

                        // TODO: register_website_for_crawling_with_refresh stays in botserver
                        log::trace!("Would register website {} for bot {} with refresh: {} (requires botserver layer)", url_str, bot_id, refresh_str);
                    }
                }
            }
        }

        Ok(())
    }
}
