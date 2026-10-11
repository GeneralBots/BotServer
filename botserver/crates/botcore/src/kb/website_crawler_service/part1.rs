use super::*;

impl WebsiteCrawlerService {
    pub fn new(db_pool: DbPool) -> Self {
        Self {
            db_pool,
            check_interval: Duration::from_secs(60),
            running: Arc::new(tokio::sync::RwLock::new(false)),
            active_crawls: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
        }
    }

    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let service = Arc::clone(&self);

        tokio::spawn(async move {
            trace!("Website crawler service started");

            // Reset any rows stuck at crawl_status=2 (in-progress) from a previous
            // crash or restart — they are no longer actually being crawled.
            if let Ok(mut conn) = service.db_pool.get() {
                let reset = diesel::sql_query(
                    "UPDATE website_crawls SET crawl_status = 0 WHERE crawl_status = 2"
                )
                .execute(&mut conn);
                match reset {
                    Ok(n) if n > 0 => info!("Reset {} stale in-progress crawl(s) to pending", n),
                    Ok(_) => {}
                    Err(e) => info!("Could not reset stale crawl statuses: {}", e),
                }
            }

            let mut ticker = interval(service.check_interval);

            loop {
                ticker.tick().await;

                if *service.running.read().await {
                    info!("Website crawler is already running, skipping this cycle");
                    continue;
                }

                *service.running.write().await = true;

                if let Err(e) = service.check_and_crawl_websites().await {
                    log::error!("Error in website crawler service: {}", e);
                }

                *service.running.write().await = false;
            }
        })
    }

    async fn check_and_crawl_websites(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        trace!("Checking for websites that need recrawling");

        // First, scan for new USE WEBSITE commands in .bas files
        self.scan_and_register_websites_from_scripts()?;

        let mut conn = self.db_pool.get()?;

        // Debug: Log all websites in database
        let all_websites: Vec<WebsiteCrawlRecord> = diesel::sql_query(
            "SELECT id, bot_id, url, expires_policy, refresh_policy, max_depth, max_pages, next_crawl, crawl_status
             FROM website_crawls
             ORDER BY id DESC
             LIMIT 10"
        )
        .load(&mut conn)?;

        trace!("Total websites in database: {}", all_websites.len());
        for ws in &all_websites {
            trace!("  - URL: {}, status: {:?}, refresh: {:?}", ws.url, ws.crawl_status, ws.refresh_policy);
        }

        let websites = diesel::sql_query(
            "SELECT id, bot_id, url, expires_policy, refresh_policy, max_depth, max_pages, next_crawl, crawl_status
             FROM website_crawls
             WHERE next_crawl <= NOW()
             AND crawl_status != 2
             ORDER BY next_crawl ASC
             LIMIT 3"
        )
        .load::<WebsiteCrawlRecord>(&mut conn)?;

        trace!("Found {} websites to recrawl (next_crawl <= NOW())", websites.len());

        // Process websites sequentially to prevent memory exhaustion
        for website in websites {
            // Skip if already being crawled
            let should_skip = {
                let active = self.active_crawls.read().await;
                active.contains(&website.url)
            };

            if should_skip {
                info!("Skipping {} - already being crawled", website.url);
                continue;
            }

            // Update status to "in progress"
            diesel::sql_query("UPDATE website_crawls SET crawl_status = 2 WHERE id = $1")
                .bind::<diesel::sql_types::Uuid, _>(&website.id)
                .execute(&mut conn)?;

            // Process one website at a time to control memory usage
            let db_pool = self.db_pool.clone();
            let active_crawls = Arc::clone(&self.active_crawls);

            trace!("Processing website: {}", website.url);

            match Self::crawl_website(website, db_pool, active_crawls).await {
                Ok(_) => {
                    trace!("Successfully processed website crawl");
                }
                Err(e) => {
                    log::error!("Failed to crawl website: {}", e);
                }
            }

            // Force memory cleanup between websites
            tokio::task::yield_now().await;

            // Add delay between crawls to prevent overwhelming the system
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }

        Ok(())
    }

    pub(crate) async fn crawl_website(
        website: WebsiteCrawlRecord,
        db_pool: DbPool,
        active_crawls: Arc<tokio::sync::RwLock<HashSet<String>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check if already crawling this URL
        {
            let mut active = active_crawls.write().await;
            if active.contains(&website.url) {
                info!("Crawl already in progress for {}, skipping", website.url);
                return Ok(());
            }
            active.insert(website.url.clone());
        }

        let url_for_cleanup = website.url.clone();
        let active_crawls_cleanup = Arc::clone(&active_crawls);

        // Always remove from active_crawls at the end, regardless of success or error.
        let result = Self::do_crawl_website(website, db_pool).await;

        active_crawls_cleanup.write().await.remove(&url_for_cleanup);

        result
    }
}
