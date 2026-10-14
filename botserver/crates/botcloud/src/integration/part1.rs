use super::*;

/// Canonical identifier slug for orgs, branches and bots (issue #779).
///
/// The signup flow and the drive monitor BOTH derive branch slugs from this
/// shape: lowercase, spaces and underscores to dashes, keeping only
/// alphanumerics and dashes. The `.gbai/` prefix the drive monitor scans is
/// built from the bot slug, so the branch slug MUST use the same sanitization
/// or the monitor resolves a different branch and a duplicate
/// `cloud_workspaces` row is created for the same org.
pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .replace([' ', '_'], "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}
/// Creates a contact in CRM (`crm_contacts`) and returns the generated ID.
pub fn create_crm_contact(
    pool: &DbPool,
    branch_id: Uuid,
    name: &str,
    email: &str,
    pass_hash: Option<&str>,
) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let id = Uuid::new_v4();
    let parts: Vec<&str> = name.splitn(2, ' ').collect();
    let first_name = parts.first().unwrap_or(&"");
    let last_name = parts.get(1);

    diesel::sql_query(
        r#"INSERT INTO crm_contacts (id, branch_id, first_name, last_name, email, pass_hash, status, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, 'active', NOW(), NOW())"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(first_name)
    .bind::<Nullable<Text>, _>(last_name.map(|s| s.to_string()))
    .bind::<Text, _>(email)
    .bind::<Nullable<Text>, _>(pass_hash.map(|s| s.to_string()))
    .execute(&mut conn)
    .map_err(|e| format!("Insert crm_contact: {e}"))?;

    Ok(id)
}
/// Creates a deal (opportunity) in CRM (`crm_deals`) linked to contact and invoice.
pub fn create_crm_deal(
    pool: &DbPool,
    branch_id: Uuid,
    contact_id: Uuid,
    _invoice_id: Uuid,
    deal_name: &str,
    value: f64,
    currency: &str,
) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let id = Uuid::new_v4();
    let now = Utc::now();

    diesel::sql_query(
        r#"INSERT INTO crm_deals (id, branch_id, contact_id, name, title, value, currency, stage, probability, owner_id, created_at, updated_at, deal_date)
           VALUES ($1, $2, $3, $4, $5, $6, $7, 'qualification', 10, $8, $9, $9, $9::date)"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<DieselUuid, _>(contact_id)
    .bind::<Text, _>(deal_name)
    .bind::<Text, _>(deal_name)
    .bind::<Numeric, _>(botbilling::api_models::bd(value))
    .bind::<Text, _>(currency)
    .bind::<DieselUuid, _>(Uuid::nil())
    .bind::<Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Insert crm_deal: {e}"))?;

    Ok(id)
}
/// Marks a deal as won and records the closing date.
pub fn win_crm_deal(
    pool: &DbPool,
    branch_id: Uuid,
    deal_name: &str,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let now = Utc::now();

    diesel::sql_query(
        r#"UPDATE crm_deals SET stage = 'won', won = true, probability = 100,
           actual_close_date = $1::date, closed_at = $1, updated_at = $1
           WHERE branch_id = $2 AND name = $3 AND (won IS NULL OR won = false) AND stage <> 'lost'"#,
    )
    .bind::<Timestamptz, _>(now)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(deal_name)
    .execute(&mut conn)
    .map_err(|e| format!("Win crm_deal: {e}"))?;

    Ok(())
}
/// Creates an organization in the `organizations` table and returns the ID.
pub fn create_organization(pool: &DbPool, name: &str, domain: Option<&str>) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
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
    .execute(&mut conn)
    .map_err(|e| format!("Insert organization: {e}"))?;

    Ok(id)
}
/// Creates a general ledger (GL) entry for a paid invoice.
///
/// Structure:
/// - Debit: Accounts Receivable (or Cash, depending on method)
/// - Credit: Subscription Revenue
pub fn create_gl_entry_for_invoice(
    pool: &DbPool,
    invoice_id: Uuid,
    total: f64,
    customer_name: &str,
) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let entry_id = Uuid::new_v4();
    let now = Utc::now();

    // Find or create default GL accounts
    let ar_account_id = get_or_create_gl_account(&mut conn, "1.1.01", "Contas a Receber - Assinaturas", "asset")?;
    let revenue_account_id = get_or_create_gl_account(&mut conn, "4.1.01", "Receita de Assinatura SaaS", "revenue")?;

    let entry_date = now.date_naive();
    let description = format!("Fatura paga: {customer_name}");

    conn.build_transaction().read_write().run::<_, diesel::result::Error, _>(|tx| {
        diesel::sql_query(
            "INSERT INTO gl_journal_entries (id, bot_id, entry_date, description, reference_type, reference_id, status, created_at) \
             VALUES ($1, '00000000-0000-0000-0000-000000000000', $2, $3, 'invoice', $4, 'posted', $5)",
        )
        .bind::<DieselUuid, _>(entry_id)
        .bind::<Date, _>(entry_date)
        .bind::<Text, _>(&description)
        .bind::<DieselUuid, _>(invoice_id)
        .bind::<Timestamptz, _>(now)
        .execute(tx)?;

        // Debit: Accounts Receivable
        diesel::sql_query(
            "INSERT INTO gl_journal_lines (id, entry_id, account_id, debit, credit, description) \
             VALUES ($1, $2, $3, $4, 0, $5)",
        )
        .bind::<DieselUuid, _>(Uuid::new_v4())
        .bind::<DieselUuid, _>(entry_id)
        .bind::<DieselUuid, _>(ar_account_id)
        .bind::<Numeric, _>(botbilling::api_models::bd(total))
        .bind::<Text, _>("Recebimento assinatura SaaS")
        .execute(tx)?;

        // Credit: Subscription Revenue
        diesel::sql_query(
            "INSERT INTO gl_journal_lines (id, entry_id, account_id, debit, credit, description) \
             VALUES ($1, $2, $3, 0, $4, $5)",
        )
        .bind::<DieselUuid, _>(Uuid::new_v4())
        .bind::<DieselUuid, _>(entry_id)
        .bind::<DieselUuid, _>(revenue_account_id)
        .bind::<Numeric, _>(botbilling::api_models::bd(total))
        .bind::<Text, _>("Reconhecimento de receita SaaS")
        .execute(tx)?;

        Ok(())
    })
    .map_err(|e| format!("Transaction: {e}"))?;

    Ok(entry_id)
}
pub(super) fn get_or_create_gl_account(
    conn: &mut diesel::PgConnection,
    code: &str,
    name: &str,
    account_type: &str,
) -> Result<Uuid, String> {
    // Try to find existing account
    let existing: Option<Uuid> = diesel::sql_query(
        "SELECT id FROM gl_accounts WHERE code = $1",
    )
    .bind::<Text, _>(code)
    .get_result::<IdRow>(conn)
    .ok()
    .map(|r| r.id);

    if let Some(id) = existing {
        return Ok(id);
    }

    // Create new account
    let id = Uuid::new_v4();
    diesel::sql_query(
        "INSERT INTO gl_accounts (id, bot_id, code, name, account_type, is_active, created_at) \
         VALUES ($1, '00000000-0000-0000-0000-000000000000', $2, $3, $4, true, NOW())",
    )
    .bind::<DieselUuid, _>(id)
    .bind::<Text, _>(code)
    .bind::<Text, _>(name)
    .bind::<Text, _>(account_type)
    .execute(conn)
    .map_err(|e| format!("Create gl_account: {e}"))?;

    Ok(id)
}
/// Creates a recurring subscription record (`billing_recurring`) for the bot.
pub fn create_billing_subscription(
    pool: &DbPool,
    branch_id: Uuid,
    customer_name: &str,
    customer_email: &str,
    plan: &str,
    total: f64,
    currency: &str,
    invoice_id: Uuid,
    period: &str,
) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    let frequency = if period == "yearly" { "yearly" } else { "monthly" };

    let interval_count = 1i32;
    let next_invoice = if frequency == "monthly" {
        now.date_naive() + chrono::Duration::days(30)
    } else {
        now.date_naive() + chrono::Duration::days(365)
    };

    diesel::sql_query(
        r#"INSERT INTO billing_recurring
           (id, branch_id, customer_name, customer_email, status, frequency, interval_count,
            amount, currency, description, next_invoice_date, start_date, last_invoice_id,
            invoices_generated, created_at, updated_at)
           VALUES ($1, $2, $3, $4, 'active', $5, $6, $7, $8, $9, $10, $11, $12, 1, $13, $13)"#,
    )
    .bind::<DieselUuid, _>(id)
    .bind::<DieselUuid, _>(branch_id)
    .bind::<Text, _>(customer_name)
    .bind::<Nullable<Text>, _>(Some(customer_email.to_string()))
    .bind::<Text, _>(frequency)
    .bind::<diesel::sql_types::Int4, _>(interval_count)
    .bind::<Numeric, _>(botbilling::api_models::bd(total))
    .bind::<Text, _>(currency)
    .bind::<Nullable<Text>, _>(Some(format!("Assinatura {plan} - {frequency}")))
    .bind::<Date, _>(next_invoice)
    .bind::<Date, _>(now.date_naive())
    .bind::<DieselUuid, _>(invoice_id)
    .bind::<Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Insert billing_recurring: {e}"))?;

    Ok(id)
}
/// Returns the default tenant ID, creating it if it doesn't exist.
pub fn get_or_create_default_tenant(pool: &DbPool) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("DB pool: {e}"))?;

    let existing: Option<Uuid> = diesel::sql_query(
        "SELECT id FROM tenants WHERE slug = 'default' LIMIT 1",
    )
    .get_result::<IdRow>(&mut conn)
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
    .execute(&mut conn)
    .map_err(|e| format!("Insert tenant: {e}"))?;

    Ok(id)
}
