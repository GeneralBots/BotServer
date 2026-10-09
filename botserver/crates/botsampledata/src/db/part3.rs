use super::*;

pub(super) fn seed_calendar(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let org = s.org_id;
    let user = s.user_id;

    let cal_id;
    let n = count(conn, "SELECT count(*) AS n FROM calendars WHERE org_id::text = $1 AND owner_id::text = $2", &[&s.org_str, &s.user_str])?;
    if n == 0 {
        cal_id = sql_query(
            "INSERT INTO calendars (id, org_id, bot_id, owner_id, name, description, color, timezone, is_primary, is_visible, is_shared, created_at, branch_id)
             VALUES ($1, $2, $2, $3, 'Work', 'Primary work calendar', '#3b82f6', 'UTC', true, true, false, NOW(), $2) RETURNING id",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(org)
        .bind::<SqlUuid, _>(user)
        .get_result::<UuidRowNamed>(conn)
        .map(|r| r.id)
        .map_err(|e| e.to_string())?;
    } else {
        cal_id = sql_query("SELECT id FROM calendars WHERE org_id = $1 AND owner_id = $2 LIMIT 1")
            .bind::<SqlUuid, _>(org)
            .bind::<SqlUuid, _>(user)
            .get_result::<UuidRowNamed>(conn)
            .map(|r| r.id)
            .map_err(|e| e.to_string())?;
    }

    let events: &[&str] = &["Sprint Planning", "Product Review"];
    for title in events {
        let n = count(conn, "SELECT count(*) AS n FROM calendar_events WHERE org_id::text = $1 AND title = $2", &[&s.org_str, title])?;
        if n == 0 {
            sql_query(
                "INSERT INTO calendar_events (id, org_id, bot_id, calendar_id, owner_id, title, description, location, start_time, end_time, status, visibility, busy_status, reminders, attendees, created_at, updated_at, branch_id)
                 VALUES ($1, $2, $2, $3, $4, $5, 'Demo event', 'Virtual', NOW() + INTERVAL '1 day', NOW() + INTERVAL '1 day' + INTERVAL '1 hour', 'confirmed', 'default', 'busy', '[]', '[]', NOW(), NOW(), $2)",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(org)
                .bind::<SqlUuid, _>(cal_id)
                .bind::<SqlUuid, _>(user)
                .bind::<Text, _>(title)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_research(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let bot = s.bot_id;
    let user = s.user_id;

    for (name, folder, count_docs) in [("Market Research", "/kb/market", 12), ("Product Docs", "/kb/product", 24)] {
        let n = count(conn, "SELECT count(*) AS n FROM kb_collections WHERE bot_id::text = $1 AND name = $2", &[&s.bot_str, name])?;
        if n == 0 {
            sql_query(
                "INSERT INTO kb_collections (id, bot_id, name, folder_path, qdrant_collection, document_count, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, 'default_demo_' || $3, $5, NOW(), NOW())",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(bot)
                .bind::<Text, _>(name)
                .bind::<Text, _>(folder)
                .bind::<Integer, _>(count_docs)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }

    // Seed documents inside the collections so the universal search endpoint
    // has indexed knowledge-base content to return for the research app.
    let docs: &[(&str, &str, &str)] = &[
        ("Market Research", "/kb/market/enterprise-rag-2025.md", "Enterprise RAG adoption trends and budgets for 2025"),
        ("Market Research", "/kb/market/on-prem-gpu-inference.md", "Cost analysis of on-premises GPU inference clusters"),
        ("Market Research", "/kb/market/competitor-analysis-q2.md", "Competitor feature comparison for conversational AI platforms"),
        ("Product Docs", "/kb/product/api-reference.md", "Complete REST API reference for the General Bots platform"),
        ("Product Docs", "/kb/product/getting-started.md", "Quick start guide covering installation and first bot creation"),
        ("Product Docs", "/kb/product/whatsapp-integration.md", "Connecting WhatsApp Business API to a bot in five steps"),
    ];
    for (collection, file_path, summary) in docs {
        let n = count(
            conn,
            "SELECT count(*) AS n FROM kb_documents WHERE bot_id::text = $1 AND file_path = $2",
            &[&s.bot_str, file_path],
        )?;
        if n == 0 {
            sql_query(
                "INSERT INTO kb_documents (id, bot_id, collection_name, file_path, file_size, file_hash, first_published_at, last_modified_at, fail_count, metadata, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, 2048, md5($4), NOW(), NOW(), 0, jsonb_build_object('summary', $5), NOW(), NOW())",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(bot)
                .bind::<Text, _>(collection)
                .bind::<Text, _>(file_path)
                .bind::<Text, _>(summary)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }

    for query in ["enterprise rag adoption 2025", "on-prem gpu inference"] {
        let n = count(conn, "SELECT count(*) AS n FROM research_searches WHERE user_id::text = $1 AND query = $2", &[&s.user_str, query])?;
        if n == 0 {
            sql_query("INSERT INTO research_searches (id, user_id, query, created_at) VALUES ($1, $2, $3, NOW())")
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(user)
                .bind::<Text, _>(query)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
pub(super) fn seed_compliance(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let user = s.user_id;

    let checks: &[(&str, &str, i32)] = &[
        ("SOC2", "compliant", 94),
        ("ISO27001", "non_compliant", 40),
    ];
    for (framework, status, score) in checks {
        let n = count(conn, "SELECT count(*) AS n FROM compliance_checks WHERE branch_id::text = $1 AND check_type = $2", &[&s.branch_str, framework])?;
        if n == 0 {
            sql_query(
                "INSERT INTO compliance_checks (id, branch_id, check_type, status, target_type, target_id, result, checked_at, checked_by, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, 'control', NULL, jsonb_build_object('score', $5), NOW(), $6, NOW(), NOW())",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(branch)
                .bind::<Text, _>(framework)
                .bind::<Text, _>(status)
                .bind::<Integer, _>(*score)
                .bind::<Nullable<SqlUuid>, _>(Some(user))
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_goals(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let org = s.org_id;
    let user = s.user_id;

    let obj_id;
    let n = count(conn, "SELECT count(*) AS n FROM okr_objectives WHERE org_id::text = $1 AND title = 'Ship v1.0'", &[&s.org_str])?;
    if n == 0 {
        obj_id = sql_query(
            "INSERT INTO okr_objectives (id, org_id, bot_id, owner_id, title, description, period, period_start, period_end, status, progress, visibility, tags, created_at, updated_at, branch_id)
             VALUES ($1, $2, $2, $3, 'Ship v1.0', 'Deliver the v1 platform', 'Q3-2026', '2026-07-01', '2026-09-30', 'active', 45, 'team', ARRAY['v1','launch'], NOW(), NOW(), $2) RETURNING id",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(org)
        .bind::<SqlUuid, _>(user)
        .get_result::<UuidRowNamed>(conn)
        .map(|r| r.id)
        .map_err(|e| e.to_string())?;
    } else {
        obj_id = sql_query("SELECT id FROM okr_objectives WHERE org_id = $1 AND title = 'Ship v1.0' LIMIT 1")
            .bind::<SqlUuid, _>(org)
            .get_result::<UuidRowNamed>(conn)
            .map(|r| r.id)
            .map_err(|e| e.to_string())?;
    }

    for (title, current) in [("Complete all 12 milestones", 5), ("Reach 95% test coverage", 72)] {
        let n = count(conn, "SELECT count(*) AS n FROM okr_key_results WHERE org_id::text = $1 AND objective_id::text = $2 AND title = $3", &[&s.org_str, &obj_id.to_string(), title])?;
        if n == 0 {
            sql_query(
                "INSERT INTO okr_key_results (id, org_id, bot_id, objective_id, owner_id, title, metric_type, start_value, target_value, current_value, unit, status, due_date, created_at, updated_at, branch_id)
                 VALUES ($1, $2, $2, $3, $4, $5, 'count', 0, 100, $6, 'units', 'in_progress', '2026-09-30', NOW(), NOW(), $2)",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(org)
                .bind::<SqlUuid, _>(obj_id)
                .bind::<SqlUuid, _>(user)
                .bind::<Text, _>(title)
                .bind::<Integer, _>(current)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_workspaces(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let org = s.org_id;
    let bot = s.bot_id;
    let branch = s.branch_id;
    let user = s.user_id;

    let ws_id;
    let n = count(conn, "SELECT count(*) AS n FROM aiworkspaces WHERE branch_id::text = $1 AND name = 'Product Workspace'", &[&s.branch_str])?;
    if n == 0 {
        ws_id = sql_query(
            "INSERT INTO aiworkspaces (id, org_id, bot_id, name, description, icon_type, icon_value, settings, created_by, created_at, updated_at, branch_id)
             VALUES ($1, $2, $3, 'Product Workspace', 'Shared product documentation', 'emoji', '🚀', '{}', $4, NOW(), NOW(), $5) RETURNING id",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(org)
        .bind::<SqlUuid, _>(bot)
        .bind::<SqlUuid, _>(user)
        .bind::<SqlUuid, _>(branch)
        .get_result::<UuidRowNamed>(conn)
        .map(|r| r.id)
        .map_err(|e| e.to_string())?;
    } else {
        ws_id = sql_query("SELECT id FROM aiworkspaces WHERE branch_id = $1 AND name = 'Product Workspace' LIMIT 1")
            .bind::<SqlUuid, _>(branch)
            .get_result::<UuidRowNamed>(conn)
            .map(|r| r.id)
            .map_err(|e| e.to_string())?;
    }

    for (title, icon) in [("Home", "🏠"), ("Roadmap", "🗺️")] {
        let n = count(conn, "SELECT count(*) AS n FROM aiworkspace_pages WHERE workspace_id::text = $1 AND title = $2", &[&ws_id.to_string(), title])?;
        if n == 0 {
            sql_query(
                "INSERT INTO aiworkspace_pages (id, workspace_id, parent_id, title, icon_type, icon_value, content, properties, is_template, is_public, public_edit, position, created_by, last_edited_by, created_at, updated_at)
                 VALUES ($1, $2, NULL, $3, 'emoji', $4, '[]', '{}', false, false, false, 0, $5, $5, NOW(), NOW())",
            )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(ws_id)
                .bind::<Text, _>(title)
                .bind::<Text, _>(icon)
                .bind::<SqlUuid, _>(user)
                .execute(conn)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
