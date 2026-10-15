use super::*;

/// Process a .gborg bucket (tenant) during initial discovery
pub(super) async fn process_org_bucket(
    state: &Arc<AppState>,
    pool: &botcore::shared::utils::DbPool,
    bucket_name: &str,
) {
    let tenant_slug = bucket_name.strip_suffix(".gborg").unwrap_or(bucket_name).to_string();


    // Get S3 client to list prefixes
    let (tenant_id, org_id) = if let Ok(result) = ensure_tenant_and_org(pool, &tenant_slug) {
        result
    } else {
        return;
    };

    // Discover .gbai/ prefixes within the .gborg bucket
    if let Some(s3) = &state.drive {
        match s3.list_common_prefixes(bucket_name, "/").await {
            Ok(prefixes) => {
                for prefix in prefixes {
                    if !prefix.ends_with(".gbai/") {
                        continue;
                    }
                    let branch_slug = prefix.strip_suffix(".gbai/").unwrap_or(&prefix).to_string();

                    // Ensure branch record + cloud workspace row (mirrors Drive in cloud UI)
                    match ensure_branch_exists(pool, &branch_slug, org_id, tenant_id) {
                        Ok(branch_id) => {
                            if let Err(e) = ensure_cloud_workspace(pool, org_id, branch_id, &branch_slug) {
                                warn!("Failed to ensure cloud workspace for branch '{}': {}", branch_slug, e);
                            }
                        }
                        Err(e) => warn!("Failed to ensure branch '{}': {}", branch_slug, e),
                    }

                    // Discover bot names within this branch
                    discover_and_create_bots(
                        state, pool, bucket_name, &prefix,
                        &branch_slug, tenant_id, org_id,
                    ).await;

                    // Start monitors for bots in this branch immediately
                    let bot_names = get_bot_names_in_branch(pool, &branch_slug, org_id).await;
                    for bot_name in bot_names {
                        start_org_bot_monitor(
                            state, pool, bucket_name, &bot_name,
                            &branch_slug, &tenant_slug, &prefix,
                        ).await;
                    }
                }
            }
            Err(e) => error!("Failed to list prefixes in .gborg bucket '{}': {}", bucket_name, e),
        }
    }
}
/// Discover bot names within a branch prefix and create them
pub(super) async fn discover_and_create_bots(
    state: &Arc<AppState>,
    pool: &botcore::shared::utils::DbPool,
    bucket_name: &str,
    branch_prefix: &str,
    branch_slug: &str,
    tenant_id: Uuid,
    org_id: Uuid,
) {
    if let Some(s3) = &state.drive {
        match s3.list_objects(bucket_name, Some(branch_prefix)).await {
            Ok(objects) => {
                // Extract unique bot names from object keys.
                // Key format: {branch}.gbai/{bot}.gbdialog/file.bas
                //
                // Reform #1501 — a bot's sources move to its repository, so
                // discovery must not require `.gbdialog` objects to be present
                // in Drive: any `{bot}.gb*` directory of the branch identifies
                // the bot. Only a top-level `{bot}.gb*` counts (a nested key
                // would name a path, not a bot).
                const BOT_MARKERS: [&str; 4] = [".gbdialog", ".gbot", ".gbkb", ".gbdrive"];
                let mut bot_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
                for key in &objects {
                    let relative = key.strip_prefix(branch_prefix).unwrap_or(key);
                    for marker in BOT_MARKERS {
                        if let Some(dot_idx) = relative.find(marker) {
                            let bot_name = &relative[..dot_idx];
                            if !bot_name.is_empty() && !bot_name.contains('/') {
                                bot_names.insert(bot_name.to_string());
                            }
                            break;
                        }
                    }
                }

                if bot_names.is_empty() {
                    // Create default bot (branch name == bot name)
                    bot_names.insert(branch_slug.to_string());
                }

                for bot_name in &bot_names {
                    info!("Ensuring bot '{}' in branch '{}' (org: {}, tenant: {})",
                          bot_name, branch_slug, org_id, tenant_id);

                    let create_state = state.clone();
                    let bn = bot_name.clone();
                    let bs = branch_slug.to_string();
                    let bs_is_default = bn == bs;
                    let pool_create = pool.clone();
                    let created = match tokio::task::spawn_blocking(move || {
                        create_bot_from_drive(
                            &create_state, &pool_create, &bn,
                            Some((tenant_id, org_id, bs)),
                            Some(bs_is_default),
                        )
                    }).await {
                        Ok(Ok(_)) => {
                            info!("Bot '{}' created in org", bot_name);
                            true
                        }
                        Ok(Err(e)) => {
                            error!("Failed to create bot '{}' in org: {}", bot_name, e);
                            false
                        }
                        Err(e) => {
                            error!("Task failed for bot '{}' in org: {}", bot_name, e);
                            false
                        }
                    };

                    if created {
                        if let Err(e) = sync_tables_for_org_bot(
                            state, pool, bot_name, bucket_name, branch_prefix,
                        ).await {
                            warn!("Failed to sync tables for new org bot '{}': {}", bot_name, e);
                        }
                    }
                }
            }
            Err(e) => error!("Failed to list objects in '{}/{}': {}", bucket_name, branch_prefix, e),
        }
    }
}
/// Periodic re-scan for a .gborg bucket
pub(super) async fn scan_org_bucket(
    state: &Arc<AppState>,
    pool: &botcore::shared::utils::DbPool,
    bucket_name: &str,
    monitored_bots: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
    let tenant_slug = bucket_name.strip_suffix(".gborg").unwrap_or(bucket_name).to_string();

    let (tenant_id, org_id) = ensure_tenant_and_org(pool, &tenant_slug)?;

    if let Some(s3) = &state.drive {
        let prefixes = s3.list_common_prefixes(bucket_name, "/")
            .await
            .map_err(|e| format!("Failed to list prefixes: {}", e))?;

        for prefix in prefixes {
            if !prefix.ends_with(".gbai/") {
                continue;
            }
            let branch_slug = prefix.strip_suffix(".gbai/").unwrap_or(&prefix).to_string();
            let branch_key = format!("{}/", branch_slug);

            if monitored_bots.contains(&branch_key) {
                continue;
            }

            // Ensure branch + cloud workspace row mirror this Drive branch
            let branch_id = ensure_branch_exists(pool, &branch_slug, org_id, tenant_id)?;
            if let Err(e) = ensure_cloud_workspace(pool, org_id, branch_id, &branch_slug) {
                warn!("Failed to ensure cloud workspace for branch '{}': {}", branch_slug, e);
            }

            // Discover bots within branch
            discover_and_create_bots(
                state, pool, bucket_name, &prefix,
                &branch_slug, tenant_id, org_id,
            ).await;

            // Mark branch as monitored (track via prefix)
            monitored_bots.insert(branch_key);

            // Start monitors for newly created bots in this branch
            let bot_names = get_bot_names_in_branch(pool, &branch_slug, org_id).await;
            for bot_name in bot_names {
                let mon_key = format!("{}/{}", branch_slug, bot_name);
                if !monitored_bots.contains(&mon_key) {
                    monitored_bots.insert(mon_key);
                    start_org_bot_monitor(state, pool, bucket_name, &bot_name, &branch_slug,
                                          &tenant_slug, &prefix).await;
                }
            }
        }
    }

    Ok(())
}
/// Start a DriveMonitor for a bot within a .gborg bucket
pub(super) async fn start_org_bot_monitor(
    state: &Arc<AppState>,
    pool: &botcore::shared::utils::DbPool,
    bucket_name: &str,
    bot_name: &str,
    branch_slug: &str,
    tenant_slug: &str,
    s3_prefix: &str,
) {
    let mon_pool = pool.clone();
    let mn = bot_name.to_string();
    let bot_id = tokio::task::spawn_blocking(move || {
        use diesel::prelude::*;
        use botcore::shared::models::schema::bots;
        let mut conn = match mon_pool.get() {
            Ok(c) => c,
            Err(_) => return None,
        };
        bots::dsl::bots
            .filter(bots::dsl::name.eq(&mn))
            .select(bots::dsl::id)
            .first::<Uuid>(&mut conn)
            .ok()
    }).await.ok().flatten();

    if let Some(bot_id) = bot_id {
        let mon_state = state.clone();
        let bucket = bucket_name.to_string();
        let prefix = s3_prefix.to_string();
        let bn = bot_name.to_string();
        let bs = branch_slug.to_string();
        let ts = tenant_slug.to_string();
        let org_slug_for_spawn = format!("{}-org", ts);
        let bn_for_log = bn.clone();
        let bucket_for_log = bucket.clone();
        tokio::spawn(async move {
            use crate::drive::drive_monitor::DriveMonitor;
            use botcore::shared::memory_monitor::register_thread;
            register_thread(&format!("drive-monitor-{}", bn_for_log), "drive");
            let monitor = DriveMonitor::new_with_params(botdrive::drive_monitor::DriveMonitorParams {
                state: mon_state,
                bucket_name: bucket,
                bot_id,
                bot_name: bn,
                branch_slug: bs,
                s3_prefix: Some(prefix),
                tenant_slug: ts,
                org_slug: org_slug_for_spawn,
            });
            info!("Starting DriveMonitor for org bot: {} (bucket: {})", bn_for_log, bucket_for_log);
            if let Err(e) = monitor.start_monitoring().await {
                error!("DriveMonitor failed for org bot {}: {}", bn_for_log, e);
            }
        });
    }
}
/// Get bot names in a branch from the database
pub(super) async fn get_bot_names_in_branch(
    pool: &botcore::shared::utils::DbPool,
    branch_slug: &str,
    org_id: Uuid,
) -> Vec<String> {
    let pool_clone = pool.clone();
    let bs = branch_slug.to_string();
    tokio::task::spawn_blocking(move || {
        use diesel::prelude::*;
        use botcore::shared::models::schema::{branches, bots};
        let mut conn = match pool_clone.get() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let branch_id: Uuid = match branches::dsl::branches
            .filter(branches::dsl::slug.eq(&bs))
            .filter(branches::dsl::org_id.eq(org_id))
            .select(branches::dsl::id)
            .first::<Uuid>(&mut conn)
        {
            Ok(id) => id,
            Err(_) => return Vec::new(),
        };
        bots::dsl::bots
            .filter(bots::dsl::branch_id.eq(branch_id))
            .filter(bots::dsl::is_active.eq(true))
            .select(bots::dsl::name)
            .load::<String>(&mut conn)
            .unwrap_or_default()
    }).await.unwrap_or_default()
}
