use super::*;

pub(super) fn ensure_tenant_and_org(pool: &botcore::shared::utils::DbPool, tenant_slug: &str) -> Result<(Uuid, Uuid), String> {
    use diesel::sql_query;
    let mut conn = pool.get().map_err(|e| e.to_string())?;

    // Upsert tenant
    let tenant_id = Uuid::new_v4();
    sql_query(format!(
        "INSERT INTO tenants (id, name, slug, created_at) \
         VALUES ('{tid}', '{slug}', '{slug}', NOW()) \
         ON CONFLICT (slug) DO UPDATE SET name = '{slug}'",
        tid = tenant_id,
        slug = tenant_slug
    ))
    .execute(&mut conn)
    .map_err(|e| format!("Failed to upsert tenant '{}': {}", tenant_slug, e))?;

    // Get tenant_id
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct TenantId { #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid }
    let tid: Uuid = diesel::sql_query(
        "SELECT id FROM tenants WHERE slug = $1 LIMIT 1"
    )
    .bind::<diesel::sql_types::Text, _>(tenant_slug)
    .get_result::<TenantId>(&mut conn)
    .map_err(|e| format!("Failed to get tenant_id: {}", e))?
    .id;

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct OrgRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        org_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
        tenant_id: Option<Uuid>,
    }

    // Reuse an existing org matching this tenant: signup creates organizations
    // with slug = {name}, drive_monitor's legacy naming appends "-org". Both
    // must resolve to the SAME org/branch so a bucket -> signup -> workspace
    // does not produce a duplicated cloud_workspaces row (issue #779).
    let legacy_org_slug = format!("{}-org", tenant_slug);
    let reuse_ids: [&str; 2] = [tenant_slug, &legacy_org_slug];
    let mut reusable: Option<OrgRow> = None;
    for slug in &reuse_ids {
        match diesel::sql_query(
            "SELECT org_id, tenant_id FROM organizations WHERE slug = $1 LIMIT 1"
        )
        .bind::<diesel::sql_types::Text, _>(slug)
        .get_result::<OrgRow>(&mut conn)
        {
            Ok(row) => { reusable = Some(row); break; }
            Err(_) => continue,
        }
    }

    let resolved_org_id: Uuid;
    let effective_tenant_id: Uuid = match reusable {
        // Reuse the org as-is, keeping ITS tenant so branch/workspace scopes
        // stay consistent with whatever created it (signup or default tenant).
        Some(OrgRow { org_id, tenant_id: Some(org_tenant) }) => {
            resolved_org_id = org_id;
            org_tenant
        }
        Some(OrgRow { org_id, tenant_id: None }) => {
            resolved_org_id = org_id;
            tid
        }
        None => {
            // No existing org — create the legacy "{slug}-org" naming.
            let org_id = Uuid::new_v4();
            sql_query(format!(
                "INSERT INTO organizations (org_id, tenant_id, name, slug, created_at) \
                 VALUES ('{oid}', '{tid}', '{slug}', '{orgslug}', NOW()) \
                 ON CONFLICT (slug) DO NOTHING",
                oid = org_id,
                tid = tid,
                slug = tenant_slug,
                orgslug = format!("{}-org", tenant_slug)
            ))
            .execute(&mut conn)
            .ok();

            let row: OrgRow = diesel::sql_query(
                "SELECT org_id, tenant_id FROM organizations WHERE tenant_id = $1 AND slug = $2 LIMIT 1"
            )
            .bind::<diesel::sql_types::Uuid, _>(tid)
            .bind::<diesel::sql_types::Text, _>(format!("{}-org", tenant_slug))
            .get_result::<OrgRow>(&mut conn)
            .map_err(|e| format!("Failed to get org_id: {}", e))?;
            resolved_org_id = row.org_id;
            tid
        }
    };

    Ok((effective_tenant_id, resolved_org_id))
}
pub(super) fn ensure_branch_exists(pool: &botcore::shared::utils::DbPool, branch_slug: &str, org_id: Uuid, tenant_id: Uuid) -> Result<Uuid, String> {
    use diesel::sql_query;
    let mut conn = pool.get().map_err(|e| e.to_string())?;

    let branch_id = Uuid::new_v4();
    sql_query(format!(
        "INSERT INTO branches (id, org_id, tenant_id, slug, name, created_at) \
         VALUES ('{bid}', '{oid}', '{tid}', '{slug}', '{slug}', NOW()) \
         ON CONFLICT (org_id, slug) DO NOTHING",
        bid = branch_id,
        oid = org_id,
        tid = tenant_id,
        slug = branch_slug
    ))
    .execute(&mut conn)
    .map_err(|e| format!("Failed to upsert branch '{}': {}", branch_slug, e))?;

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct BranchId { #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid }
    let resolved_bid: Uuid = diesel::sql_query(
        "SELECT id FROM branches WHERE org_id = $1 AND slug = $2 LIMIT 1"
    )
    .bind::<diesel::sql_types::Uuid, _>(org_id)
    .bind::<diesel::sql_types::Text, _>(branch_slug)
    .get_result::<BranchId>(&mut conn)
    .map_err(|e| format!("Failed to get branch_id: {}", e))?
    .id;

    Ok(resolved_bid)
}
pub(super) fn create_bot_from_drive(
    _state: &Arc<botcore::shared::state::AppState>,
    pool: &botcore::shared::utils::DbPool,
    bot_name: &str,
    org_info: Option<(Uuid, Uuid, String)>, // (tenant_id, org_id, branch_slug)
    is_default: Option<bool>,
) -> Result<(), String> {
    use diesel::sql_query;

    let mut conn = pool.get().map_err(|e| e.to_string())?;

    let (tenant_id, org_id, branch_slug) = match org_info {
        Some((tid, oid, bs)) => (tid, oid, bs),
        None => {
            // Legacy mode: resolve actual default tenant/org from database
            #[derive(diesel::QueryableByName)]
            #[diesel(check_for_backend(diesel::pg::Pg))]
            struct OrgId { #[diesel(sql_type = diesel::sql_types::Uuid)] org_id: Uuid }

            #[derive(diesel::QueryableByName)]
            #[diesel(check_for_backend(diesel::pg::Pg))]
            struct TenantId { #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid }

            let actual_tenant_id: Uuid = diesel::sql_query(
                "SELECT id FROM tenants WHERE slug = 'default' LIMIT 1"
            ).get_result::<TenantId>(&mut conn)
             .map(|r| r.id)
             .unwrap_or(DEFAULT_TENANT_ID);

            let actual_org_id: Uuid = diesel::sql_query(
                "SELECT org_id FROM organizations WHERE slug = 'default' LIMIT 1"
            ).get_result::<OrgId>(&mut conn)
             .map(|r| r.org_id)
             .unwrap_or(DEFAULT_ORG_ID);

            let bs = bot_name.to_string();
            (actual_tenant_id, actual_org_id, bs)
        }
    };

    let branch_id = ensure_branch_exists(pool, &branch_slug, org_id, tenant_id)?;
    let bot_id = Uuid::new_v4();
    let is_default = is_default.unwrap_or(true);
    let isdef = if is_default { "true" } else { "false" };

    // Drive bucket names keep their original case, so a bot already recorded
    // from Drive can hold a mixed-case slug (`PragmatismoGB`). The INSERT below
    // only dedups on an exact `slug` match, so without this lookup a bucket
    // scanned after a side rewrote the slug's case would insert a *second* bot
    // instead of updating the existing row. Resolve case-insensitively first and
    // update that row in place, preserving its id and `database_name`.
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct ExistingBotId {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }

    let existing: Option<Uuid> = sql_query(format!(
        "SELECT id FROM bots WHERE lower(slug) = lower('{name}') OR lower(name) = lower('{name}') \
         ORDER BY created_at ASC LIMIT 1",
        name = bot_name,
    ))
    .get_result::<ExistingBotId>(&mut conn)
    .ok()
    .map(|r| r.id);

    if let Some(existing_id) = existing {
        sql_query(format!(
            "UPDATE bots SET is_active = true, org_id = '{oid}', branch_id = '{branchid}', \
             is_default_for_branch = {isdef}, updated_at = NOW() WHERE id = '{existing_id}'",
            oid = org_id,
            branchid = branch_id,
            isdef = isdef,
            existing_id = existing_id,
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update existing bot '{bot_name}': {e}"))?;
        info!("create_bot_from_drive: adopted existing bot {existing_id} for '{bot_name}'");
        return Ok(());
    }

    let result = sql_query(format!(
        "INSERT INTO bots (id, name, slug, org_id, branch_id, is_default_for_branch, \
                          is_active, created_at, updated_at, llm_provider, llm_config, \
                          context_provider, context_config, is_public) \
         VALUES ('{bid}', '{name}', '{name}', '{oid}', '{branchid}', {isdef}, \
                 true, NOW(), NOW(), 'openai', '{{}}', 'openai', '{{}}', false) \
         ON CONFLICT (slug) DO UPDATE SET is_active = true, org_id = '{oid}', \
         branch_id = '{branchid}', updated_at = NOW()",
        bid = bot_id,
        name = bot_name,
        oid = org_id,
        branchid = branch_id,
        isdef = isdef,
    ))
    .execute(&mut conn);

    if result.is_err() {
        sql_query(format!(
            "UPDATE bots SET is_active = true, org_id = '{oid}', branch_id = '{branchid}', \
             is_default_for_branch = {isdef}, updated_at = NOW(), \
             llm_provider = 'openai', llm_config = '{{}}', \
             context_provider = 'openai', context_config = '{{}}' \
             WHERE name = '{name}'",
            oid = org_id,
            branchid = branch_id,
            isdef = isdef,
            name = bot_name
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update bot '{}': {}", bot_name, e))?;
    }

    let db_name = format!("bot_{}", bot_name.replace('-', "_"));
    let _ = sql_query(format!(
        "DO $$ BEGIN IF NOT EXISTS (SELECT FROM pg_database WHERE datname = '{db}') THEN EXECUTE 'CREATE DATABASE {db}'; END IF; END $$;",
        db = db_name
    ))
    .execute(&mut conn);

    info!("Bot '{}' created (org: {}, branch: {})", bot_name, org_id, branch_id);
    Ok(())
}
pub(crate) async fn start_drive_compiler(app_state: Arc<AppState>) {
    use crate::drive::drive_compiler::DriveCompiler;
    let compiler = DriveCompiler::new(app_state.clone());
    if let Err(e) = compiler.start_compiling().await {
        error!("Failed to start DriveCompiler: {}", e);
    } else {
        trace!("DriveCompiler started - compiling .bas files from drive_files");
    }
}
