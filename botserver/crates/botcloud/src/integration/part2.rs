use super::*;

/// Links an organization to a tenant.
pub fn link_org_to_tenant(pool: &DbPool, org_id: Uuid, tenant_id: Uuid) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;

    diesel::sql_query(
        "UPDATE organizations SET tenant_id = $1 WHERE org_id = $2",
    )
    .bind::<DieselUuid, _>(tenant_id)
    .bind::<DieselUuid, _>(org_id)
    .execute(&mut conn)
    .map_err(|e| format!("Link org to tenant: {e}"))?;

    Ok(())
}
/// Creates a branch under an organization and tenant.
pub fn create_branch(pool: &DbPool, org_id: Uuid, tenant_id: Uuid, name: &str) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let id = Uuid::new_v4();
    let slug = slugify(name);

    diesel::sql_query(
        r#"INSERT INTO branches (id, org_id, tenant_id, slug, name, is_active, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, true, NOW(), NOW())"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<DieselUuid, _>(org_id)
    .bind::<DieselUuid, _>(tenant_id)
    .bind::<Text, _>(&slug)
    .bind::<Text, _>(name)
    .execute(&mut conn)
    .map_err(|e| format!("Insert branch: {e}"))?;

    Ok(id)
}
/// Creates a bot record in the database and returns (bot_id, org_slug).
pub fn create_bot(pool: &DbPool, org_id: Uuid, branch_id: Uuid, _tenant_id: Uuid, name: &str) -> Result<(Uuid, String), String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let bot_id = Uuid::new_v4();
    let slug = slugify(name);
    let org_slug = slug.clone();
    let now = Utc::now();

    diesel::sql_query(
        r#"INSERT INTO bots
           (id, name, slug, org_id, branch_id, tenant_id, is_default_for_branch,
            is_active, created_at, updated_at, llm_provider, llm_config,
            context_provider, context_config, is_public, database_name)
           VALUES ($1, $2, $3, $4, $5, $6, true,
                   true, $7, $7, 'openai', '{}'::jsonb,
                    'openai', '{}'::jsonb, true, $8)"#,
    )
    .bind::<DieselUuid, _>(bot_id)
    .bind::<Text, _>(name)
    .bind::<Text, _>(&slug)
    .bind::<DieselUuid, _>(org_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<DieselUuid, _>(Uuid::nil())
    .bind::<Timestamptz, _>(now)
    .bind::<Nullable<Text>, _>(Some(format!("cloud_{}", slug)))
    .execute(&mut conn)
    .map_err(|e| format!("Insert bot: {e}"))?;

    Ok((bot_id, org_slug))
}

// ── _inner variants for use inside transactions ──
pub fn get_or_create_default_tenant_inner(conn: &mut PgConnection) -> Result<Uuid, String> {
    let existing: Option<Uuid> = diesel::sql_query(
        "SELECT id FROM tenants WHERE slug = 'default' LIMIT 1",
    )
    .get_result::<IdRow>(conn)
    .ok()
    .map(|r| r.id);

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = Uuid::new_v4();
    diesel::sql_query(
        "INSERT INTO tenants (id, name, slug, is_active, created_at, updated_at) \
         VALUES ($1, 'Default Tenant', 'default', true, NOW(), NOW())",
    )
    .bind::<DieselUuid, _>(id)
    .execute(conn)
    .map_err(|e| format!("Insert tenant: {e}"))?;

    Ok(id)
}
pub fn create_organization_inner(conn: &mut PgConnection, name: &str, domain: Option<&str>) -> Result<Uuid, String> {
    let id = Uuid::new_v4();
    let slug = slugify(name);

    diesel::sql_query(
        r#"INSERT INTO organizations (org_id, name, slug, domain, created_at, updated_at)
           VALUES ($1, $2, $3, $4, NOW(), NOW())"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<Text, _>(name)
    .bind::<Text, _>(&slug)
    .bind::<Nullable<Text>, _>(domain)
    .execute(conn)
    .map_err(|e| format!("Insert organization: {e}"))?;

    Ok(id)
}
pub fn link_org_to_tenant_inner(conn: &mut PgConnection, org_id: Uuid, tenant_id: Uuid) -> Result<(), String> {
    diesel::sql_query(
        "UPDATE organizations SET tenant_id = $1 WHERE org_id = $2",
    )
    .bind::<DieselUuid, _>(tenant_id)
    .bind::<DieselUuid, _>(org_id)
    .execute(conn)
    .map_err(|e| format!("Link org to tenant: {e}"))?;

    Ok(())
}
pub fn create_branch_inner(conn: &mut PgConnection, org_id: Uuid, tenant_id: Uuid, name: &str) -> Result<Uuid, String> {
    let id = Uuid::new_v4();
    let slug = slugify(name);

    diesel::sql_query(
        r#"INSERT INTO branches (id, org_id, tenant_id, slug, name, is_active, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, true, NOW(), NOW())"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<DieselUuid, _>(org_id)
    .bind::<DieselUuid, _>(tenant_id)
    .bind::<Text, _>(&slug)
    .bind::<Text, _>(name)
    .execute(conn)
    .map_err(|e| format!("Insert branch: {e}"))?;

    Ok(id)
}
pub fn create_bot_inner(conn: &mut PgConnection, org_id: Uuid, branch_id: Uuid, name: &str) -> Result<(Uuid, String), String> {
    let bot_id = Uuid::new_v4();
    let slug = slugify(name);
    let org_slug = slug.clone();
    let now = Utc::now();

    diesel::sql_query(
        r#"INSERT INTO bots
           (id, name, slug, org_id, branch_id, tenant_id, is_default_for_branch,
            is_active, created_at, updated_at, llm_provider, llm_config,
            context_provider, context_config, is_public, database_name)
           VALUES ($1, $2, $3, $4, $5, $6, true,
                   true, $7, $7, 'openai', '{}'::jsonb,
                    'openai', '{}'::jsonb, true, $8)"#,
    )
    .bind::<DieselUuid, _>(bot_id)
    .bind::<Text, _>(name)
    .bind::<Text, _>(&slug)
    .bind::<DieselUuid, _>(org_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<DieselUuid, _>(Uuid::nil())
    .bind::<Timestamptz, _>(now)
    .bind::<Nullable<Text>, _>(Some(format!("cloud_{}", slug)))
    .execute(conn)
    .map_err(|e| format!("Insert bot: {e}"))?;

    Ok((bot_id, org_slug))
}
pub fn create_crm_contact_inner(
    conn: &mut PgConnection,
    branch_id: Uuid,
    bot_id: Uuid,
    name: &str,
    email: &str,
    pass_hash: Option<&str>,
) -> Result<Uuid, String> {
    let id = Uuid::new_v4();
    let parts: Vec<&str> = name.splitn(2, ' ').collect();
    let first_name = parts.first().unwrap_or(&"");
    let last_name = parts.get(1);

    diesel::sql_query(
        r#"INSERT INTO crm_contacts (id, org_id, bot_id, branch_id, first_name, last_name, email, pass_hash, status, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active', NOW(), NOW())"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<DieselUuid, _>(bot_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(first_name)
    .bind::<Nullable<Text>, _>(last_name.map(|s| s.to_string()))
    .bind::<Text, _>(email)
    .bind::<Nullable<Text>, _>(pass_hash.map(|s| s.to_string()))
    .execute(conn)
    .map_err(|e| format!("Insert crm_contact: {e}"))?;

    Ok(id)
}
pub fn create_free_subscription_inner(
    conn: &mut PgConnection,
    branch_id: Uuid,
    bot_id: Uuid,
    customer_name: &str,
    customer_email: &str,
) -> Result<Uuid, String> {
    let sub_id = Uuid::new_v4();
    let now = Utc::now();

    diesel::sql_query(
        r#"INSERT INTO billing_recurring
           (id, org_id, bot_id, branch_id, customer_name, customer_email, status, frequency, interval_count,
            amount, currency, description, next_invoice_date, start_date, last_invoice_id,
            invoices_generated, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, 'active', 'monthly', 1, 0.0, 'USD',
                   'Free Plan', $9, $7, NULL, 0, $8, $8)"#,
    )
    .bind::<DieselUuid, _>(sub_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<DieselUuid, _>(bot_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(customer_name)
    .bind::<Nullable<Text>, _>(Some(customer_email.to_string()))
    .bind::<Date, _>(now.date_naive())
    .bind::<Timestamptz, _>(now)
    .bind::<Date, _>(now.date_naive())
    .execute(conn)
    .map_err(|e| format!("Insert free subscription: {e}"))?;

    Ok(sub_id)
}
pub fn create_trial_subscription_inner(
    conn: &mut PgConnection,
    branch_id: Uuid,
    bot_id: Uuid,
    customer_name: &str,
    customer_email: &str,
    plan: &str,
    trial_days: i32,
) -> Result<Uuid, String> {
    let sub_id = Uuid::new_v4();
    let now = Utc::now();
    let trial_end = now.date_naive() + Duration::days(trial_days as i64);

    diesel::sql_query(
        r#"INSERT INTO billing_recurring
           (id, org_id, bot_id, branch_id, customer_name, customer_email, status, frequency, interval_count,
            amount, currency, description, next_invoice_date, start_date, last_invoice_id,
            invoices_generated, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, 'trialing', 'monthly', 1, 0.0, 'USD',
                   $7, $8, $9, NULL, 0, $10, $10)"#,
    )
    .bind::<DieselUuid, _>(sub_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<DieselUuid, _>(bot_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(customer_name)
    .bind::<Nullable<Text>, _>(Some(customer_email.to_string()))
    .bind::<Nullable<Text>, _>(Some(format!("{plan} - {trial_days} Day Trial")))
    .bind::<Date, _>(trial_end)
    .bind::<Date, _>(now.date_naive())
    .bind::<Timestamptz, _>(now)
    .execute(conn)
    .map_err(|e| format!("Insert trial subscription: {e}"))?;

    Ok(sub_id)
}
pub fn create_cloud_workspace_inner(conn: &mut PgConnection, branch_id: Uuid, name: &str) -> Result<Uuid, String> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    diesel::sql_query(
        r#"INSERT INTO cloud_workspaces (id, org_id, branch_id, name, description, icon, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, 'default', $6, $6)"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(name)
    .bind::<Nullable<Text>, _>(Some(format!("Default workspace for {}", name)))
    .bind::<Timestamptz, _>(now)
    .execute(conn)
    .map_err(|e| format!("Insert cloud_workspace: {e}"))?;

    Ok(id)
}
