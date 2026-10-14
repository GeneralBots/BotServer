use super::*;

pub(crate) async fn start_drive_monitors(
    app_state: Arc<AppState>,
    _pool: &botcore::shared::utils::DbPool,
) {
    use botcore::shared::memory_monitor::register_thread;

    let pool_clone = _pool.clone();
    let state_for_scan = app_state.clone();
    let scan_pool = _pool.clone();

    // Bootstrap: ensure branches table schema (migration 9.15) if diesel pipeline was blocked
    ensure_org_branches_schema(_pool);

    // Bootstrap: ensure cloud_workspaces unique index (migration 6.5.31) if pipeline was blocked
    ensure_cloud_workspaces_schema(_pool);

    // Bootstrap: ensure default tenant/org/branch/bot exist
    if let Err(e) = ensure_bootstrap_defaults(_pool) {
        error!("Bootstrap defaults failed: {}", e);
    } else {
        info!("Bootstrap defaults ensured (tenant/org/branch/bot)");
    }

    tokio::spawn(async move {
        register_thread("drive-monitor", "drive");

        // Step 1: Discover bots from S3 .gborg buckets
        log::info!("Drive client status: {:?}", state_for_scan.drive.is_some());
        if let Some(s3_client) = &state_for_scan.drive {
            match s3_client.list_all_buckets().await {
                Ok(buckets) => {
                    for bucket in buckets {
                        if bucket.ends_with(".gborg") {
                            process_org_bucket(
                                &state_for_scan, &pool_clone, &bucket,
                            ).await;
                        }
                    }
                }
                Err(e) => warn!("Failed to list S3 buckets for bot discovery: {}", e),
            }
        }

        // Step 2: Periodic bucket re-scan (bots are discovered via process_org_bucket)
        let mut monitored_bots: std::collections::HashSet<String> = std::collections::HashSet::new();

        let mut last_list_error: Option<std::time::Instant> = None;
        let mut consecutive_failures: u32 = 0;
        let mut unhealthy_logged = false;
        let scan_state = app_state.clone();
        tokio::spawn(async move {
            register_thread("drive-scan", "drive");
            // Every cycle performs a full ListBuckets plus a walk (and
            // drive_files upsert) of every `.gborg` bucket, so a 5s cadence
            // saturated a core on a busy instance and pushed every endpoint —
            // Drive's file list included — into multi-second responses. The
            // per-bot monitors still pick up config.csv / .gbdialog changes on
            // their own cadence, so this slower discovery loop costs freshness
            // only for brand-new buckets. Override with DRIVE_SCAN_INTERVAL.
            let scan_interval = std::env::var("DRIVE_SCAN_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30u64);
            const MAX_BACKOFF_SECS: u64 = 300; // 5 minutes

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(scan_interval)).await;

                if let Some(s3_client) = &scan_state.drive {
                    match s3_client.list_all_buckets().await {
                        Ok(buckets) => {
                            if last_list_error.is_some() {
                                info!("Drive scan recovered — S3 buckets now accessible");
                            }
                            if consecutive_failures > 0 {
                                info!(
                                    "Drive scan recovered after {} consecutive failures",
                                    consecutive_failures
                                );
                            }
                            consecutive_failures = 0;
                            unhealthy_logged = false;
                            last_list_error = None;
                            for bucket in buckets {
                                if bucket.ends_with(".gborg") {
                                    if let Err(e) = scan_org_bucket(
                                        &scan_state, &scan_pool, &bucket,
                                        &mut monitored_bots,
                                    ).await {
                                        warn!("Periodic .gborg scan failed for {}: {}", bucket, e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            consecutive_failures += 1;
                            // Exponential backoff: grow the sleep after repeated failures
                            // so a dead/unreachable drive does not burn CPU forever.
                            if consecutive_failures > 1 {
                                let backoff = scan_interval.saturating_mul(
                                    1u64 << consecutive_failures.min(6)
                                ).min(MAX_BACKOFF_SECS);
                                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                            }
                            if consecutive_failures >= 10 && !unhealthy_logged {
                                error!(
                                    "Drive is UNHEALTHY after {} consecutive ListBuckets failures: {}. \
                                     Check MinIO credentials (VAULT_TOKEN / MINIO_ACCESS_KEY / \
                                     MINIO_SECRET_KEY). No further alerts until recovery.",
                                    consecutive_failures, e
                                );
                                unhealthy_logged = true;
                            }
                            let now = std::time::Instant::now();
                            let should_log = last_list_error.is_none_or(|t| now.duration_since(t).as_secs() > 120);
                            if should_log {
                                warn!("Periodic bucket re-scan failed: {}", e);
                                last_list_error = Some(now);
                            }
                        }
                    }
                }
            }
        });
    });
}
/// Bootstrap: ensure default tenant/org/branch/bot exist for fresh installs
pub(super) fn ensure_bootstrap_defaults(pool: &botcore::shared::utils::DbPool) -> Result<(), String> {
    use diesel::sql_query;

    let mut conn = pool.get().map_err(|e| e.to_string())?;

    if let Err(e) = sql_query(format!(
        "INSERT INTO tenants (id, name, slug, created_at) \
         VALUES ('{tid}', 'Default Tenant', 'default', NOW()) \
         ON CONFLICT (slug) DO NOTHING",
        tid = DEFAULT_TENANT_ID
    ))
    .execute(&mut conn) {
        warn!("Failed to insert default tenant: {}", e);
    }

    if let Err(e) = sql_query(format!(
        "INSERT INTO organizations (org_id, tenant_id, name, slug, created_at) \
         VALUES ('{oid}', '{tid}', 'Default Organization', 'default', NOW()) \
         ON CONFLICT (slug) DO NOTHING",
        oid = DEFAULT_ORG_ID,
        tid = DEFAULT_TENANT_ID
    ))
    .execute(&mut conn) {
        warn!("Failed to insert default organization: {}", e);
    }

    // Look up the actual org_id and tenant_id from the database
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct OrgId { #[diesel(sql_type = diesel::sql_types::Uuid)] org_id: Uuid }

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct TenantId { #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid }

    let actual_org_id: Uuid = sql_query(
        "SELECT org_id FROM organizations WHERE slug = 'default' LIMIT 1"
    ).get_result::<OrgId>(&mut conn)
     .map(|r| r.org_id)
     .unwrap_or(DEFAULT_ORG_ID);

    let actual_tenant_id: Uuid = sql_query(
        "SELECT id FROM tenants WHERE slug = 'default' LIMIT 1"
    ).get_result::<TenantId>(&mut conn)
     .map(|r| r.id)
     .unwrap_or(DEFAULT_TENANT_ID);

    if let Err(e) = sql_query(format!(
        "INSERT INTO branches (id, org_id, tenant_id, slug, name, created_at) \
         VALUES ('{bid}', '{oid}', '{tid}', 'default', 'Default Branch', NOW()) \
         ON CONFLICT (org_id, slug) DO NOTHING",
        bid = DEFAULT_BRANCH_ID,
        oid = actual_org_id,
        tid = actual_tenant_id
    ))
    .execute(&mut conn) {
        warn!("Failed to insert default branch: {}", e);
    }

    // Look up the actual branch_id
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct BranchId { #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid }

    let actual_branch_id: Uuid = sql_query(
        "SELECT id FROM branches WHERE org_id = $1 AND slug = 'default' LIMIT 1"
    ).bind::<diesel::sql_types::Uuid, _>(actual_org_id)
     .get_result::<BranchId>(&mut conn)
     .map(|r| r.id)
     .unwrap_or(DEFAULT_BRANCH_ID);

    if let Err(e) = sql_query(format!(
        "INSERT INTO bots (id, name, slug, org_id, branch_id, is_default_for_branch, \
                           is_active, created_at, updated_at, llm_provider, llm_config, \
                           context_provider, context_config, is_public) \
         VALUES ('{botid}', 'default', 'default', '{oid}', '{bid}', true, \
                 true, NOW(), NOW(), 'openai', '{{}}', 'openai', '{{}}', false) \
         ON CONFLICT (slug) DO NOTHING",
        botid = DEFAULT_BOT_ID,
        oid = actual_org_id,
        bid = actual_branch_id
    ))
    .execute(&mut conn) {
        warn!("Failed to insert default bot: {}", e);
    }

    info!("Default tenant, organization, branch, and bot ensured");
    Ok(())
}
