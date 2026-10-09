use super::*;

/// Ensure the 9.15-org-branches migration schema exists (branches table + bots columns).
/// Runs independently of the diesel migration pipeline to handle environments where
/// pre-existing migrations block the pipeline from reaching 9.15.
pub(super) fn ensure_org_branches_schema(pool: &botcore::shared::utils::DbPool) {
    use diesel::sql_query;
    if let Ok(mut conn) = pool.get() {
        // Ensure tenants table exists (pre-requisite for branches)
        if let Err(e) = sql_query(
            "CREATE TABLE IF NOT EXISTS tenants (
                id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                name        VARCHAR(255) NOT NULL,
                slug        VARCHAR(255) NOT NULL UNIQUE,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to create tenants table: {}", e);
        }

        // Ensure tenant_id column exists on organizations (pre-requisite for branches)
        if let Err(e) = sql_query(
            "ALTER TABLE organizations ADD COLUMN IF NOT EXISTS tenant_id UUID REFERENCES tenants(id)"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to add tenant_id to organizations: {}", e);
        }

        if let Err(e) = sql_query(
            "CREATE TABLE IF NOT EXISTS branches (
                id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                org_id      UUID NOT NULL REFERENCES organizations(org_id) ON DELETE CASCADE,
                tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                slug        VARCHAR(255) NOT NULL,
                name        VARCHAR(255) NOT NULL,
                description TEXT,
                is_active   BOOLEAN DEFAULT true,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(org_id, slug)
            )"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to create branches table: {}", e);
        }

        if let Err(e) = sql_query(
            "ALTER TABLE bots ADD COLUMN IF NOT EXISTS branch_id UUID REFERENCES branches(id)"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to add branch_id to bots: {}", e);
        }

        if let Err(e) = sql_query(
            "ALTER TABLE bots ADD COLUMN IF NOT EXISTS is_default_for_branch BOOLEAN DEFAULT false"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to add is_default_for_branch to bots: {}", e);
        }

        if let Err(e) = sql_query(
            "CREATE INDEX IF NOT EXISTS idx_branches_org ON branches(org_id)"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to create idx_branches_org: {}", e);
        }

        if let Err(e) = sql_query(
            "CREATE INDEX IF NOT EXISTS idx_branches_tenant ON branches(tenant_id)"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to create idx_branches_tenant: {}", e);
        }

        if let Err(e) = sql_query(
            "CREATE INDEX IF NOT EXISTS idx_branches_slug ON branches(slug)"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to create idx_branches_slug: {}", e);
        }

        if let Err(e) = sql_query(
            "CREATE INDEX IF NOT EXISTS idx_bots_branch ON bots(branch_id)"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to create idx_bots_branch: {}", e);
        }

        if let Err(e) = sql_query(
            "UPDATE bots SET org_id = '00000000-0000-0000-0000-000000000001' WHERE org_id IS NULL"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to update null org_ids: {}", e);
        }

        if let Err(e) = sql_query(
            "ALTER TABLE bots ALTER COLUMN org_id SET NOT NULL"
        ).execute(&mut conn) {
            warn!("ensure_org_branches_schema: failed to set org_id NOT NULL: {}", e);
        }

        trace!("ensure_org_branches_schema: branches table and bot columns ensured");
    }
}
/// Ensure the cloud_workspaces unique index on branch_id exists (migration 6.5.31).
/// One cloud workspace per Drive branch (.gbai prefix), so drive_monitor can
/// upsert workspace rows and keep the cloud UI equal to Drive.
pub(super) fn ensure_cloud_workspaces_schema(pool: &botcore::shared::utils::DbPool) {
    use diesel::sql_query;
    if let Ok(mut conn) = pool.get() {
        if let Err(e) = sql_query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_cloud_workspaces_branch \
             ON cloud_workspaces(branch_id)"
        ).execute(&mut conn) {
            warn!("ensure_cloud_workspaces_schema: failed to create unique index: {}", e);
        }
        trace!("ensure_cloud_workspaces_schema: unique index on cloud_workspaces(branch_id) ensured");
    }
}
/// Upsert a cloud_workspaces row for a Drive branch (.gbai prefix) so the
/// SaaS cloud UI lists every workspace that exists in Drive.
/// Prefers ON CONFLICT (branch_id); falls back to select-then-upsert when the
/// database has no unique constraint on branch_id (older deployments).
pub(super) fn ensure_cloud_workspace(
    pool: &botcore::shared::utils::DbPool,
    org_id: Uuid,
    branch_id: Uuid,
    branch_slug: &str,
) -> Result<(), String> {
    use diesel::sql_query;
    let mut conn = pool.get().map_err(|e| e.to_string())?;

    let ws_id = Uuid::new_v4();
    let insert_result = sql_query(
        "INSERT INTO cloud_workspaces (id, org_id, branch_id, name, description, icon, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, 'default', NOW(), NOW()) \
         ON CONFLICT (branch_id) DO UPDATE SET name = EXCLUDED.name, updated_at = NOW()",
    )
    .bind::<diesel::sql_types::Uuid, _>(ws_id)
    .bind::<diesel::sql_types::Uuid, _>(org_id)
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .bind::<diesel::sql_types::Text, _>(branch_slug)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
        Some(format!("Default workspace for {}", branch_slug)),
    )
    .execute(&mut conn);

    match insert_result {
        Ok(_) => {
            info!("Cloud workspace '{}' ensured (org: {}, branch: {})", branch_slug, org_id, branch_id);
            return Ok(());
        }
        Err(e) => {
            trace!("ON CONFLICT upsert unavailable for branch '{}' ({}), falling back to select-then-upsert", branch_slug, e);
        }
    }

    // Fallback for databases without a unique constraint on branch_id
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct WsIdRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }
    let existing: Option<Uuid> = sql_query(
        "SELECT id FROM cloud_workspaces WHERE branch_id = $1 LIMIT 1"
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .get_result::<WsIdRow>(&mut conn)
    .ok()
    .map(|r| r.id);

    match existing {
        Some(existing_id) => {
            sql_query("UPDATE cloud_workspaces SET name = $2, updated_at = NOW() WHERE id = $1")
                .bind::<diesel::sql_types::Uuid, _>(existing_id)
                .bind::<diesel::sql_types::Text, _>(branch_slug)
                .execute(&mut conn)
                .map_err(|e| format!("Failed to update cloud workspace '{}': {}", branch_slug, e))?;
            info!("Cloud workspace '{}' updated (org: {}, branch: {})", branch_slug, org_id, branch_id);
        }
        None => {
            sql_query(
                "INSERT INTO cloud_workspaces (id, org_id, branch_id, name, description, icon, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, 'default', NOW(), NOW())",
            )
            .bind::<diesel::sql_types::Uuid, _>(ws_id)
            .bind::<diesel::sql_types::Uuid, _>(org_id)
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .bind::<diesel::sql_types::Text, _>(branch_slug)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
                Some(format!("Default workspace for {}", branch_slug)),
            )
            .execute(&mut conn)
            .map_err(|e| format!("Failed to insert cloud workspace '{}': {}", branch_slug, e))?;
            info!("Cloud workspace '{}' ensured (org: {}, branch: {})", branch_slug, org_id, branch_id);
        }
    }

    Ok(())
}
/// True when the bot's `.gbdialog` is owned by its git repository (reform
/// #1501/#1502), i.e. the git-pull monitor feeds it and Drive must not.
pub(super) async fn is_git_owned_bot(pool: &botcore::shared::utils::DbPool, branch_slug: &str, bot_name: &str) -> bool {
    let pool = pool.clone();
    let branch = branch_slug.trim().to_string();
    let bot = bot_name.to_string();
    tokio::task::spawn_blocking(move || {
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            n: i64,
        }
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return false,
        };
        diesel::sql_query(
            "SELECT count(*) AS n FROM vibe_projects vp \
             JOIN branches br ON br.id = vp.branch_id \
             WHERE vp.project_type = 'bot' AND vp.source_control = 'git' \
               AND vp.name = $1 AND br.slug = $2",
        )
        .bind::<diesel::sql_types::Text, _>(&bot)
        .bind::<diesel::sql_types::Text, _>(&branch)
        .get_result::<Row>(&mut conn)
        .map(|r| r.n > 0)
        .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}
/// Sync tables.bas for a bot within a .gborg bucket structure.
///
/// Reform #1501/#1502 — a git-owned bot's schema travels with its sources: the
/// monitor materializes `.gbdialog/tables.bas` and compiling it runs
/// `process_table_definitions` (wired as a compiler callback). Reading Drive
/// here would apply a stale schema over the repository's.
pub(super) async fn sync_tables_for_org_bot(
    state: &Arc<AppState>,
    pool: &botcore::shared::utils::DbPool,
    bot_name: &str,
    bucket_name: &str,
    s3_prefix: &str,
) -> Result<(), String> {
    let branch_slug = s3_prefix
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .strip_suffix(".gbai")
        .unwrap_or("");
    if is_git_owned_bot(pool, branch_slug, bot_name).await {
        trace!(
            "tables.bas for git-owned bot '{}' comes from its repository, skipping Drive sync",
            bot_name
        );
        return Ok(());
    }
    let bot_id = get_bot_id(pool, bot_name).await?;

    let content = match &state.drive {
        Some(s3) => {
            match s3.get_object_direct(bucket_name, &format!("{}{}.gbdialog/tables.bas", s3_prefix, bot_name)).await {
                Ok(data) => String::from_utf8(data).map_err(|e| format!("UTF-8 error: {e}"))?,
                Err(_) => {
                    trace!("tables.bas not found in S3 for org bot {}, skipping", bot_name);
                    return Ok(());
                }
            }
        }
        None => {
            trace!("S3 client not available, skipping table sync for org bot {}", bot_name);
            return Ok(());
        }
    };

    let runtime: Arc<dyn botbasic_types::BasicRuntime> = Arc::new(AppStateBasicRuntime(state.clone()));
    match process_table_definitions(runtime, bot_id, &content) {
        Ok(tables) => info!("Synced {} table definitions for org bot '{}'", tables.len(), bot_name),
        Err(e) => warn!("Failed to sync table definitions for org bot '{}': {}", bot_name, e),
    }
    Ok(())
}
pub(super) async fn get_bot_id(
    pool: &botcore::shared::utils::DbPool,
    bot_name: &str,
) -> Result<Uuid, String> {
    let bn = bot_name.to_string();
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        use diesel::prelude::*;
        use botcore::shared::models::schema::bots;
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return None,
        };
        bots::dsl::bots
            .filter(bots::dsl::name.eq(&bn))
            .select(bots::dsl::id)
            .first::<Uuid>(&mut conn)
            .ok()
    })
    .await
    .map_err(|e| format!("Task error: {e}"))?
    .ok_or_else(|| format!("Bot '{}' not found in database", bot_name))
}
