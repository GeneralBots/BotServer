use super::*;

pub(super) fn seed_people(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let org = s.org_id;
    let bot = s.bot_id;
    let branch = s.branch_id;
    let user = s.user_id;

    for (dept, code) in [("Engineering", "ENG"), ("Marketing", "MKT")] {
        let n = count(conn, "SELECT count(*) AS n FROM people_departments WHERE org_id::text = $1 AND code = $2", &[&s.org_str, code])?;
        if n == 0 {
            sql_query("INSERT INTO people_departments (id, org_id, bot_id, name, description, code, is_active, created_at, updated_at, branch_id)
                       VALUES ($1, $2, $3, $4, $4 || ' dept', $5, true, NOW(), NOW(), $6)")
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(org)
                .bind::<SqlUuid, _>(bot)
                .bind::<Text, _>(dept)
                .bind::<Text, _>(code)
                .bind::<SqlUuid, _>(branch)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }

    let people: &[(&str, &str, &str, &str, &str)] = &[
        ("Alice", "Sample", "alice@sample.com", "Software Engineer", "Engineering"),
        ("Bruno", "Demo", "bruno@sample.com", "Product Manager", "Marketing"),
        ("Carla", "Test", "carla@sample.com", "Designer", "Engineering"),
    ];
    for (first, last, email, title, dept) in people {
        let n = count(conn, "SELECT count(*) AS n FROM people WHERE org_id::text = $1 AND email = $2", &[&s.org_str, email])?;
        if n == 0 {
            sql_query(
                "INSERT INTO people (id, org_id, bot_id, user_id, first_name, last_name, email, phone, job_title, department, office_location, hire_date, timezone, locale, skills, social_links, custom_fields, created_at, updated_at, branch_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, '+1 555 0100', $8, $9, 'Remote', '2024-03-01', 'UTC', 'en-US', ARRAY['rust'], '{}', '{}', NOW(), NOW(), $10)",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(org)
                .bind::<SqlUuid, _>(bot)
                .bind::<Nullable<SqlUuid>, _>(Some(user))
                .bind::<Text, _>(first)
                .bind::<Text, _>(last)
                .bind::<Text, _>(email)
                .bind::<Text, _>(title)
                .bind::<Text, _>(dept)
                .bind::<SqlUuid, _>(branch)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }

    let n = count(conn, "SELECT count(*) AS n FROM people_skills WHERE org_id::text = $1 AND name = 'Rust'", &[&s.org_str])?;
    if n == 0 {
        sql_query("INSERT INTO people_skills (id, org_id, bot_id, name, category, description, is_active, created_at, branch_id)
                   VALUES ($1, $2, $3, 'Rust', 'Engineering', 'Systems programming', true, NOW(), $4)")
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(org)
            .bind::<SqlUuid, _>(bot)
            .bind::<SqlUuid, _>(branch)
            .execute(conn)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn seed_crm(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let bot = s.bot_id;
    let user = s.user_id;

    
    let n = count(conn, "SELECT count(*) AS n FROM crm_accounts WHERE org_id::text = $1 AND name = 'Acme Corp'", &[&s.branch_str])?;
    let account_id = if n == 0 {
        
        sql_query(
            "INSERT INTO crm_accounts (id, org_id, bot_id, name, industry, website, phone, created_at, updated_at, branch_id)
             VALUES ($1, $2, $3, 'Acme Corp', 'Technology', 'https://acme.example.com', '+1 555 1000', NOW(), NOW(), $2)
             RETURNING id",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(branch)
        .bind::<SqlUuid, _>(bot)
        .get_result::<UuidRowNamed>(conn)
        .map(|r| r.id)
        .map_err(|e| e.to_string())?
    } else {
        sql_query("SELECT id FROM crm_accounts WHERE org_id = $1 AND name = 'Acme Corp' LIMIT 1")
            .bind::<SqlUuid, _>(branch)
            .get_result::<UuidRowNamed>(conn)
            .map(|r| r.id)
            .map_err(|e| e.to_string())?
    };

    let n = count(conn, "SELECT count(*) AS n FROM crm_accounts WHERE org_id::text = $1 AND name = 'Globex Inc'", &[&s.branch_str])?;
    if n == 0 {
        sql_query("INSERT INTO crm_accounts (id, org_id, bot_id, name, industry, website, phone, created_at, updated_at, branch_id)
                   VALUES ($1, $2, $3, 'Globex Inc', 'Retail', 'https://globex.example.com', '+1 555 2000', NOW(), NOW(), $2)")
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(branch)
            .bind::<SqlUuid, _>(bot)
            .execute(conn)
            .map_err(|e| e.to_string())?;
    }

    
    let n = count(conn, "SELECT count(*) AS n FROM crm_contacts WHERE org_id::text = $1 AND email = 'john.doe@acme.example.com'", &[&s.branch_str])?;
    let contact_id = if n == 0 {
        
        sql_query(
            "INSERT INTO crm_contacts (id, org_id, bot_id, first_name, last_name, email, phone, company, job_title, source, status, tags, custom_fields, city, state, country, notes, owner_id, created_at, updated_at, branch_id)
             VALUES ($1, $2, $3, 'John', 'Doe', 'john.doe@acme.example.com', '+1 555 1100', 'Acme Corp', 'CTO', 'web', 'active', ARRAY['enterprise','warm'], '{}', 'Austin', 'TX', 'US', 'Primary technical contact', $4, NOW(), NOW(), $2)
             RETURNING id",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(branch)
        .bind::<SqlUuid, _>(bot)
        .bind::<Nullable<SqlUuid>, _>(Some(user))
        .get_result::<UuidRowNamed>(conn)
        .map(|r| r.id)
        .map_err(|e| e.to_string())?
    } else {
        sql_query("SELECT id FROM crm_contacts WHERE org_id = $1 AND email = 'john.doe@acme.example.com' LIMIT 1")
            .bind::<SqlUuid, _>(branch)
            .get_result::<UuidRowNamed>(conn)
            .map(|r| r.id)
            .map_err(|e| e.to_string())?
    };

    let n = count(conn, "SELECT count(*) AS n FROM crm_contacts WHERE org_id::text = $1 AND email = 'jane.roe@globex.example.com'", &[&s.branch_str])?;
    if n == 0 {
        sql_query("INSERT INTO crm_contacts (id, org_id, bot_id, first_name, last_name, email, phone, company, job_title, source, status, tags, custom_fields, city, state, country, owner_id, created_at, updated_at, branch_id)
                   VALUES ($1, $2, $3, 'Jane', 'Roe', 'jane.roe@globex.example.com', '+1 555 2100', 'Globex Inc', 'Head of Ops', 'referral', 'active', ARRAY['retail','new'], '{}', 'Seattle', 'WA', 'US', $4, NOW(), NOW(), $2)")
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(branch)
            .bind::<SqlUuid, _>(bot)
            .bind::<Nullable<SqlUuid>, _>(Some(user))
            .execute(conn)
            .map_err(|e| e.to_string())?;
    }

    let deals: &[(&str, &str, f64, &str, i32)] = &[
        ("Acme expansion", "Acme expansion", 45000.0, "proposal", 60),
        ("Globex pilot", "Globex pilot", 12000.0, "discovery", 20),
    ];
    for (title, name, value, stage, probability) in deals {
        let n = count(conn, "SELECT count(*) AS n FROM crm_deals WHERE org_id::text = $1 AND title = $2", &[&s.branch_str, title])?;
        if n == 0 {
            sql_query(
                "INSERT INTO crm_deals (id, org_id, bot_id, contact_id, account_id, owner_id, title, name, description, value, currency, stage, probability, won, source, created_at, updated_at, branch_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'Demo deal', $9, 'USD', $10, $11, false, 'web', NOW(), NOW(), $2)",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(branch)
                .bind::<SqlUuid, _>(bot)
                .bind::<Nullable<SqlUuid>, _>(Some(contact_id))
                .bind::<Nullable<SqlUuid>, _>(Some(account_id))
                .bind::<Nullable<SqlUuid>, _>(Some(user))
                .bind::<Text, _>(title)
                .bind::<Text, _>(name)
                .bind::<diesel::sql_types::Double, _>(value)
                .bind::<Text, _>(stage)
                .bind::<Integer, _>(*probability)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_tickets(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let org = s.org_id;
    let bot = s.bot_id;
    let branch = s.branch_id;
    let user = s.user_id;

    for (cat, color, icon) in [("Incident", "#ef4444", "alert"), ("Request", "#22c55e", "hand")] {
        let n = count(conn, "SELECT count(*) AS n FROM ticket_categories WHERE org_id::text = $1 AND name = $2", &[&s.org_str, cat])?;
        if n == 0 {
            sql_query("INSERT INTO ticket_categories (id, org_id, bot_id, name, description, color, icon, sort_order, is_active, created_at, branch_id)
                       VALUES ($1, $2, $3, $4, $4 || ' category', $5, $6, 1, true, NOW(), $7)")
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(org)
                .bind::<SqlUuid, _>(bot)
                .bind::<Text, _>(cat)
                .bind::<Text, _>(color)
                .bind::<Text, _>(icon)
                .bind::<SqlUuid, _>(branch)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }

    let tickets: &[(&str, &str, &str, &str, &str, &str)] = &[
        ("TK-1001", "Cannot access dashboard", "Dashboard returns 502 after login", "open", "high", "alice@sample.com"),
        ("TK-1002", "New hire onboarding", "Please provision access for new hire", "in_progress", "medium", "bruno@sample.com"),
        ("TK-1003", "Export report missing columns", "Monthly export is missing created_at", "resolved", "low", "carla@sample.com"),
    ];
    for (num, subject, desc, status, priority, requester) in tickets {
        let n = count(conn, "SELECT count(*) AS n FROM support_tickets WHERE org_id::text = $1 AND ticket_number = $2", &[&s.org_str, num])?;
        if n == 0 {
            sql_query(
                "INSERT INTO support_tickets (id, org_id, bot_id, ticket_number, subject, description, status, priority, category, source, requester_id, requester_email, requester_name, assignee_id, tags, created_at, updated_at, branch_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'Incident', 'web', $9, $10, 'Sample User', $9, ARRAY['sample'], NOW(), NOW(), $11)",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(org)
                .bind::<SqlUuid, _>(bot)
                .bind::<Text, _>(num)
                .bind::<Text, _>(subject)
                .bind::<Text, _>(desc)
                .bind::<Text, _>(status)
                .bind::<Text, _>(priority)
                .bind::<Nullable<SqlUuid>, _>(Some(user))
                .bind::<Text, _>(requester)
                .bind::<SqlUuid, _>(branch)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_billing(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let org = s.org_id;
    let bot = s.bot_id;
    let branch = s.branch_id;

    let products: &[(&str, &str, &str, f64, &str)] = &[
        ("vps-small", "VPS Small", "4 vCPU / 8 GB RAM / 100 GB NVMe", 9.99, "infrastructure"),
        ("gpu-basic", "GPU Basic", "RTX 3060 12 GB VRAM", 39.99, "infrastructure"),
        ("domain-com", "Domain .com", "Annual .com domain", 21.99, "communication"),
        ("print-pla-100g", "PLA Filament 1kg", "PLA filament 1.75mm black", 19.99, "print3d"),
    ];
    for (sku, name, desc, price, category) in products {
        let n = count(conn, "SELECT count(*) AS n FROM products WHERE org_id::text = $1 AND sku = $2", &[&s.org_str, sku])?;
        if n == 0 {
            sql_query(
                "INSERT INTO products (id, org_id, bot_id, sku, name, description, category, product_type, price, cost, currency, tax_rate, unit, stock_quantity, low_stock_threshold, is_active, attributes, created_at, updated_at, branch_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 'service', $8, $8 * 0.4, 'USD', 0, 'unit', -1, 0, true, '{}'::jsonb, NOW(), NOW(), $9)",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(org)
                .bind::<SqlUuid, _>(bot)
                .bind::<Text, _>(sku)
                .bind::<Text, _>(name)
                .bind::<Text, _>(desc)
                .bind::<Text, _>(category)
                .bind::<diesel::sql_types::Double, _>(price)
                .bind::<SqlUuid, _>(branch)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }

    let n = count(conn, "SELECT count(*) AS n FROM billing_invoices WHERE org_id::text = $1 AND invoice_number = 'INV-2026-0001'", &[&s.branch_str])?;
    if n == 0 {
        sql_query(
            "INSERT INTO billing_invoices (id, org_id, bot_id, invoice_number, customer_id, customer_name, customer_email, status, issue_date, due_date, subtotal, tax_rate, tax_amount, total, amount_paid, amount_due, currency, notes, created_at, updated_at, branch_id)
             VALUES ($1, $2, $3, 'INV-2026-0001', NULL, 'Acme Corp', 'billing@acme.example.com', 'paid', '2026-07-01', '2026-07-31', 2500, 0, 0, 2500, 2500, 0, 'USD', 'Sample paid invoice', NOW(), NOW(), $2)",
        )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(branch)
            .bind::<SqlUuid, _>(bot)
            .execute(conn)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn seed_tasks(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let user = s.user_id;

    let tasks: &[(&str, &str, &str)] = &[
        ("Design onboarding flow", "UX flow for new user onboarding", "in_progress"),
        ("Fix login token refresh", "Token refresh loop after idle", "todo"),
    ];
    for (title, desc, status) in tasks {
        let n = count(conn, "SELECT count(*) AS n FROM tasks WHERE branch_id::text = $1 AND title = $2", &[&s.branch_str, title])?;
        if n == 0 {
            sql_query(
                "INSERT INTO tasks (id, title, description, status, priority, assignee_id, reporter_id, due_date, tags, estimated_hours, actual_hours, progress, created_at, updated_at, branch_id)
                 VALUES ($1, $2, $3, $4, 'high', $5, $5, '2026-08-20', ARRAY['demo'], 12, 6, 50, NOW(), NOW(), $6)",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<Text, _>(title)
                .bind::<Text, _>(desc)
                .bind::<Text, _>(status)
                .bind::<Nullable<SqlUuid>, _>(Some(user))
                .bind::<SqlUuid, _>(branch)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
