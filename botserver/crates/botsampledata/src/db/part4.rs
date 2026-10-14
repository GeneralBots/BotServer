use super::*;

pub(super) fn seed_social(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let org = s.org_id;
    let user = s.user_id;

    
    let n = count(conn, "SELECT count(*) AS n FROM social_communities WHERE org_id::text = $1 AND slug = 'company-announcements'", &[&s.org_str])?;
    let community_id = if n == 0 {
        sql_query(
            "INSERT INTO social_communities (id, org_id, bot_id, name, slug, description, visibility, join_policy, owner_id, member_count, post_count, is_official, is_featured, settings, created_at, updated_at, branch_id)
             VALUES ($1, $2, $2, 'Company Announcements', 'company-announcements', 'Internal announcements', 'public', 'open', $3, 3, 2, true, false, '{}', NOW(), NOW(), $2) RETURNING id",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(org)
        .bind::<SqlUuid, _>(user)
        .get_result::<UuidRowNamed>(conn)
        .map(|r| r.id)
        .map_err(|e| e.to_string())?
    } else {
        sql_query("SELECT id FROM social_communities WHERE org_id = $1 AND slug = 'company-announcements' LIMIT 1")
            .bind::<SqlUuid, _>(org)
            .get_result::<UuidRowNamed>(conn)
            .map(|r| r.id)
            .map_err(|e| e.to_string())?
    };

    for content in ["Welcome to the new platform! 🎉", "v1.0 is shipping this quarter"] {
        let n = count(conn, "SELECT count(*) AS n FROM social_posts WHERE org_id::text = $1 AND content = $2", &[&s.org_str, content])?;
        if n == 0 {
            sql_query(
                "INSERT INTO social_posts (id, org_id, bot_id, author_id, community_id, content, content_type, hashtags, visibility, comment_count, reaction_counts, created_at, branch_id)
                 VALUES ($1, $2, $2, $3, $4, $5, 'text', ARRAY['demo'], 'public', 1, '{\"like\":1}', NOW(), $2)",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(org)
                .bind::<SqlUuid, _>(user)
                .bind::<Nullable<SqlUuid>, _>(Some(community_id))
                .bind::<Text, _>(content)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub(super) fn seed_marketing(conn: &mut diesel::PgConnection, scopes: &Scopes) -> Result<(), String> {
    let branch = scopes.branch_id;
    let branch_str = &scopes.branch_str;

    let campaigns: &[(&str, &str)] = &[("Q3 Launch Campaign", "email"), ("WhatsApp Promo", "whatsapp")];
    for (name, ctype) in campaigns {
        let n = count(conn, "SELECT count(*) AS n FROM marketing_campaigns WHERE branch_id::text = $1 AND name = $2", &[branch_str, name])?;
        if n == 0 {
            sql_query(
                "INSERT INTO marketing_campaigns (id, branch_id, name, campaign_type, status, starts_at, ends_at, budget, metrics, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, 'draft', NOW(), NOW() + INTERVAL '14 days', 500, '{}', NOW(), NOW())",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(branch)
                .bind::<Text, _>(name)
                .bind::<Text, _>(ctype)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }

    for name in ["All Active Contacts", "Warm Leads"] {
        let n = count(conn, "SELECT count(*) AS n FROM marketing_lists WHERE branch_id::text = $1 AND name = $2", &[branch_str, name])?;
        if n == 0 {
            sql_query(
                "INSERT INTO marketing_lists (id, branch_id, name, list_type, description, query_text, member_count, contact_count, is_dynamic, criteria, created_at, updated_at)
                 VALUES ($1, $2, $3, 'static', 'Sample list', NULL, 0, 0, false, '{}', NOW(), NOW())",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(branch)
                .bind::<Text, _>(name)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub(super) fn seed_m365(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let bot = s.bot_id;
    let org = s.org_id;

    for (site, list, title) in [("site-engineering", "list-docs", "Engineering Docs"), ("site-marketing", "list-assets", "Marketing Assets")] {
        let n = count(conn, "SELECT count(*) AS n FROM m365_sharepoint_items WHERE site_id = $1 AND list_id::text = $2", &[site, list])?;
        if n == 0 {
            sql_query(
                "INSERT INTO m365_sharepoint_items (id, bot_id, organization_id, site_id, list_id, item_id, title, fields, author, modified_at, synced_at)
                 VALUES ($1, $2, $3, $4, $5, 'item-1', $6, '{\"doc_count\": 10}', 'Sample User', NOW(), NOW())",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(bot)
                .bind::<SqlUuid, _>(org)
                .bind::<Text, _>(site)
                .bind::<Text, _>(list)
                .bind::<Text, _>(title)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub(super) fn seed_drive(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let user = s.user_id;

    for (path, ftype, name, mime) in [
        ("/reports/q3-summary.pdf", "pdf", "q3-summary.pdf", "application/pdf"),
        ("/exports/customers.csv", "csv", "customers.csv", "text/csv"),
    ] {
        let n = count(conn, "SELECT count(*) AS n FROM drive_files WHERE branch_id::text = $1 AND user_id::text = $2 AND file_path = $3", &[&s.branch_str, &s.user_str, path])?;
        if n == 0 {
            sql_query(
                "INSERT INTO drive_files (id, file_path, file_type, last_modified, file_size, indexed, user_id, scope, branch_id, path, name, mime_type, created_at, updated_at)
                 VALUES ($1, $2, $3, NOW(), 245760, true, $4, 'user', $5, $6, $7, $8, NOW(), NOW())",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<Text, _>(path)
                .bind::<Text, _>(ftype)
                .bind::<SqlUuid, _>(user)
                .bind::<SqlUuid, _>(branch)
                .bind::<Text, _>(path.rsplit('/').next().map(|p| format!("/{}", p)).unwrap_or_default())
                .bind::<Text, _>(name)
                .bind::<Text, _>(mime)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Seeds fiscal test data for issues #722/#723/#724:
///   * `billing_tax_rates` rows (existing fiscal model) so the tax engine
///     resolves branch-scoped rates instead of defaults;
///   * `bot_configuration` overrides (`tax-*` keys) so the per-bot config
///     path of the tax engine is exercisable;
///   * `drive_files` index rows mirroring the invoice/finance objects that
///     `drive::seed_drive_objects` writes to MinIO.
pub(super) fn seed_fiscal(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let org = s.org_id;
    let branch = s.branch_id;
    let bot = s.bot_id;

    let rates: &[(&str, f64, &str)] = &[
        ("IRPJ", 4.80, "IRPJ over presumed profit (32% x 15%)"),
        ("CSLL", 2.88, "CSLL over presumed profit (32% x 9%)"),
        ("PIS/COFINS", 3.65, "PIS/COFINS cumulativo"),
        ("ISS", 5.00, "ISS sobre servicos"),
    ];
    for (name, rate, description) in rates {
        let n = count(
            conn,
            "SELECT count(*) AS n FROM billing_tax_rates WHERE branch_id::text = $1 AND name = $2",
            &[&s.branch_str, name],
        )?;
        if n == 0 {
            sql_query(
                "INSERT INTO billing_tax_rates (id, org_id, branch_id, bot_id, name, rate, description, region, is_default, is_active, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 'BR', true, true, NOW())",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(org)
            .bind::<SqlUuid, _>(branch)
            .bind::<SqlUuid, _>(bot)
            .bind::<Text, _>(name)
            .bind::<diesel::sql_types::Double, _>(rate)
            .bind::<Text, _>(description)
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }

    for (key, value) in [
        ("tax-irpj", "4.80"),
        ("tax-csll", "2.88"),
        ("tax-pis-cofins", "3.65"),
        ("tax-iss", "5.00"),
    ] {
        let n = count(
            conn,
            "SELECT count(*) AS n FROM bot_configuration WHERE bot_id::text = $1 AND config_key = $2",
            &[&s.bot_str, key],
        )?;
        if n == 0 {
            sql_query(
                "INSERT INTO bot_configuration (id, bot_id, branch_id, config_key, config_value, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, NOW(), NOW())",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(bot)
            .bind::<SqlUuid, _>(branch)
            .bind::<Text, _>(key)
            .bind::<Text, _>(value)
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }

    // Sample service in the registry so the service-tax flow (issue #722)
    // can run live: fixed-price consulting at R$ 10,000.00. tax_rate = 0
    // means the composite rates come from billing_tax_rates (16.33% total).
    // Seeded in the sample scope (dedicated demo tenant).
    {
        let n = count(
            conn,
            "SELECT count(*) AS n FROM services WHERE branch_id::text = $1 AND name = $2",
            &[&s.branch_str, "IT Consulting"],
        )?;
        if n == 0 {
            sql_query(
                "INSERT INTO services (id, org_id, bot_id, branch_id, name, description, category, service_type, fixed_price, hourly_rate, currency, duration_minutes, is_active, tax_rate, attributes, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, 'IT Consulting', 'Consulting and software development services', 'Services', 'fixed', 10000.00, NULL, 'BRL', 60, true, 0, '{}'::jsonb, NOW(), NOW())",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(org)
            .bind::<SqlUuid, _>(bot)
            .bind::<SqlUuid, _>(branch)
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }

    let now = chrono::Utc::now();
    let current_month = now.format("%Y-%m").to_string();
    let prev_month = now
        .checked_sub_months(chrono::Months::new(1))
        .map(|d| d.format("%Y-%m").to_string())
        .unwrap_or_else(|| "2026-07".to_string());
    let invoice_name = format!("telefonia-{current_month}.pdf");
    let finance_name = format!("fluxo-caixa-{current_month}.csv");
    let prev_finance_name = format!("fluxo-caixa-{prev_month}.csv");

    for (path, ftype, name, mime) in [
        (
            format!("/faturas/{invoice_name}"),
            "pdf",
            invoice_name.as_str(),
            "application/pdf",
        ),
        (
            format!("/financeiro/{finance_name}"),
            "csv",
            finance_name.as_str(),
            "text/csv",
        ),
        (
            format!("/financeiro/{prev_finance_name}"),
            "csv",
            prev_finance_name.as_str(),
            "text/csv",
        ),
    ] {
        let n = count(
            conn,
            "SELECT count(*) AS n FROM drive_files WHERE branch_id::text = $1 AND file_path = $2",
            &[&s.branch_str, path.as_str()],
        )?;
        if n == 0 {
            sql_query(
                "INSERT INTO drive_files (id, file_path, file_type, last_modified, file_size, indexed, user_id, scope, branch_id, path, name, mime_type, created_at, updated_at)
                 VALUES ($1, $2, $3, NOW(), 245760, true, $4, 'user', $5, $6, $7, $8, NOW(), NOW())",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<Text, _>(path.clone())
            .bind::<Text, _>(ftype)
            .bind::<SqlUuid, _>(s.user_id)
            .bind::<SqlUuid, _>(branch)
            .bind::<Text, _>(path.rsplit('/').next().map(|p| format!("/{}", p)).unwrap_or_default())
            .bind::<Text, _>(name)
            .bind::<Text, _>(mime)
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
