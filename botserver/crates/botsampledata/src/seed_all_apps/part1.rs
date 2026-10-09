use super::*;

/// Count helper reusing the parent module's semantics.
pub(super) fn count(conn: &mut diesel::PgConnection, sql: &str, binds: &[&str]) -> Result<i64, String> {
    #[derive(diesel::QueryableByName)]
    struct C {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        n: i64,
    }
    let r = match binds.len() {
        0 => sql_query(sql).get_result::<C>(conn),
        1 => sql_query(sql)
            .bind::<Text, _>(binds[0])
            .get_result::<C>(conn),
        2 => sql_query(sql)
            .bind::<Text, _>(binds[0])
            .bind::<Text, _>(binds[1])
            .get_result::<C>(conn),
        3 => sql_query(sql)
            .bind::<Text, _>(binds[0])
            .bind::<Text, _>(binds[1])
            .bind::<Text, _>(binds[2])
            .get_result::<C>(conn),
        4 => sql_query(sql)
            .bind::<Text, _>(binds[0])
            .bind::<Text, _>(binds[1])
            .bind::<Text, _>(binds[2])
            .bind::<Text, _>(binds[3])
            .get_result::<C>(conn),
        _ => return Err("count: too many binds".to_string()),
    };
    r.map(|r| r.n).map_err(|e| e.to_string())
}
/// Whether a table exists in the public schema (prevents seeding crate-owned
/// tables that have not been created yet).
pub(super) fn table_exists(conn: &mut diesel::PgConnection, name: &str) -> bool {
    #[derive(diesel::QueryableByName)]
    struct Exists {
        #[diesel(sql_type = diesel::sql_types::Bool)]
        exists: bool,
    }
    let r: Result<Exists, _> = sql_query("SELECT to_regclass($1) IS NOT NULL AS exists")
        .bind::<Text, _>(format!("public.{name}"))
        .get_result(conn);
    r.map(|r| r.exists).unwrap_or(false)
}
/// Returns the first UUID of a single-column query (empty/not-found → nil).
pub(super) fn first_uuid(conn: &mut diesel::PgConnection, sql: &str) -> Result<Uuid, String> {
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }
    sql_query(sql).get_result::<Row>(conn).map(|r| r.id).map_err(|e| e.to_string())
}
/// Seeds every additional app. Failures are logged and isolated per domain.
pub fn seed(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let domains: Vec<(&str, fn(&mut diesel::PgConnection, &Scopes) -> Result<(), String>)> = vec![
        ("products", seed_products),
        ("dashboards", seed_dashboards),
        ("meet", seed_meet),
        ("learn", seed_learn),
        ("project", seed_project),
        ("canvas", seed_canvas),
        ("attendant", seed_attendant),
        ("okr", seed_okr),
        ("database", seed_database),
        ("sales", seed_sales),
        ("hr", seed_hr),
        ("tax", seed_tax),
        ("kyc", seed_kyc),
        ("fraud", seed_fraud),
    ];

    for (name, seed_fn) in domains {
        match seed_fn(conn, s) {
            Ok(()) => log::info!("botsampledata: {name} seeded"),
            Err(e) => log::error!("botsampledata: {name} seeding failed: {e}"),
        }
    }
    Ok(())
}
pub(super) fn seed_products(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    // Products are scoped to the dedicated sample branch so the demo catalog
    // never pollutes a real tenant.
    let scope_branch = s.branch_id;
    let scope_str = s.branch_str.clone();
    let catalog: &[(&str, &str, &str, &str, f64, i32)] = &[
        ("hosting-basic", "Hosting Basic", "Shared web hosting plan", "plan", 9.99, 100),
        ("hosting-pro", "Hosting Pro", "VPS hosting with dedicated resources", "infrastructure", 19.99, 50),
        ("ssl-cert", "SSL Certificate", "Annual SSL certificate", "communication", 21.99, 200),
        ("support-hour", "Support Hour", "Specialized support hour", "service", 49.90, -1),
    ];
    for (sku, name, desc, ptype, price, stock) in catalog {
        let n = count(
            conn,
            "SELECT count(*) AS n FROM products WHERE branch_id::text = $1 AND sku = $2",
            &[&scope_str, sku],
        )?;
        if n == 0 {
            sql_query(
                "INSERT INTO products (id, branch_id, sku, name, description, category, product_type, price, currency, tax_rate, unit, stock_quantity, low_stock_threshold, is_active, images, attributes, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'BRL', 16.33, 'unit', $9, 10, true, '{}'::jsonb, '{}'::jsonb, NOW(), NOW())",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(scope_branch)
            .bind::<Text, _>(sku)
            .bind::<Text, _>(name)
            .bind::<Text, _>(desc)
            .bind::<Text, _>(ptype)
            .bind::<Text, _>(ptype)
            .bind::<diesel::sql_types::Double, _>(price)
            .bind::<Integer, _>(*stock)
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_dashboards(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let user = s.user_id;
    let n = count(conn, "SELECT count(*) AS n FROM dashboards WHERE branch_id::text = $1 AND name = 'Executive Overview'", &[&s.branch_str])?;
    if n == 0 {
        sql_query(
            "INSERT INTO dashboards (id, org_id, bot_id, branch_id, owner_id, name, description, layout, refresh_interval, is_public, is_template, tags, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, 'Executive Overview', 'Company KPIs', '{}'::jsonb, 300, true, false, ARRAY['kpi','finance'], NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(s.org_id)
        .bind::<SqlUuid, _>(s.bot_id)
        .bind::<SqlUuid, _>(branch)
        .bind::<SqlUuid, _>(user)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn seed_meet(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let user = s.user_id;
    let n = count(conn, "SELECT count(*) AS n FROM meeting_rooms WHERE branch_id::text = $1 AND name = 'Weekly Sync'", &[&s.branch_str])?;
    if n == 0 {
        let room = Uuid::new_v4();
        sql_query(
            "INSERT INTO meeting_rooms (id, org_id, bot_id, branch_id, room_code, name, description, created_by, max_participants, is_recording, is_transcribing, status, settings, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'WEEKLY', 'Weekly Sync', 'Team standup', $5, 12, true, true, 'active', '{}'::jsonb, NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(room)
        .bind::<SqlUuid, _>(s.org_id)
        .bind::<SqlUuid, _>(s.bot_id)
        .bind::<SqlUuid, _>(branch)
        .bind::<SqlUuid, _>(user)
        .execute(conn)
        .map_err(|e| e.to_string())?;

        let rec_n = count(conn, "SELECT count(*) AS n FROM meeting_recordings WHERE room_id::text = $1", &[&room.to_string()])?;
        if rec_n == 0 {
            sql_query(
                "INSERT INTO meeting_recordings (id, room_id, org_id, bot_id, branch_id, recording_type, file_url, file_size, duration_seconds, status, started_at, metadata, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, 'video', 'drive://meet/weekly.mp4', 52428800, 1800, 'completed', NOW() - INTERVAL '1 day', '{}'::jsonb, NOW(), NOW())",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(room)
            .bind::<SqlUuid, _>(s.org_id)
            .bind::<SqlUuid, _>(s.bot_id)
            .bind::<SqlUuid, _>(branch)
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_learn(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let n = count(conn, "SELECT count(*) AS n FROM learn_courses WHERE title = 'Onboarding Fundamentals'", &[])?;
    if n == 0 {
        sql_query(
            "INSERT INTO learn_courses (id, organization_id, title, description, category, difficulty, duration_minutes, is_mandatory, due_days, is_published, created_by, created_at, updated_at)
             VALUES ($1, $2, 'Onboarding Fundamentals', 'Company onboarding course', 'Onboarding', 'beginner', 45, true, 14, true, $3, NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<Nullable<SqlUuid>, _>(Some(s.org_id))
        .bind::<Nullable<SqlUuid>, _>(Some(s.user_id))
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn seed_project(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let n = count(conn, "SELECT count(*) AS n FROM projects WHERE name = 'Website Redesign'", &[])?;
    if n == 0 {
        let project_id = Uuid::new_v4();
        // Live schema: `projects.org` is varchar; project_type/deploy_target/
        // environment are NOT NULL.
        sql_query(
            "INSERT INTO projects (id, org, name, project_type, deploy_target, environment, status, description, created_at, updated_at)
             VALUES ($1, $2, 'Website Redesign', 'web', 'production', 'dev', 'active', 'Refresh the corporate website', NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(project_id)
        .bind::<Text, _>(&s.org_str)
        .execute(conn)
        .map_err(|e| e.to_string())?;

        let t = count(conn, "SELECT count(*) AS n FROM project_tasks WHERE project_id::text = $1", &[&project_id.to_string()])?;
        if t == 0 {
            for (idx, name) in ["Design mockups", "Content migration", "QA pass"].iter().enumerate() {
                sql_query(
                    "INSERT INTO project_tasks (id, project_id, name, task_type, start_date, end_date, duration_days, percent_complete, status, priority, assigned_to, wbs, created_at, updated_at)
                     VALUES ($1, $2, $3, 'milestone', CURRENT_DATE, CURRENT_DATE + INTERVAL '7 days', 7, 0, 'not_started', 'medium', ARRAY[]::uuid[], $4, NOW(), NOW())",
                )
                .bind::<SqlUuid, _>(Uuid::new_v4())
                .bind::<SqlUuid, _>(project_id)
                .bind::<Text, _>(name)
                .bind::<Text, _>(&format!("1.{}", idx + 1))
                .execute(conn)
                .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}
pub(super) fn seed_canvas(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let user = s.user_id;
    let n = count(conn, "SELECT count(*) AS n FROM canvases WHERE branch_id::text = $1 AND name = 'Product Roadmap Board'", &[&s.branch_str])?;
    if n == 0 {
        sql_query(
            "INSERT INTO canvases (id, org_id, bot_id, branch_id, name, description, width, height, is_public, is_template, created_by, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'Product Roadmap Board', 'Planning whiteboard', 1920, 1080, true, false, $5, NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(s.org_id)
        .bind::<SqlUuid, _>(s.bot_id)
        .bind::<SqlUuid, _>(branch)
        .bind::<SqlUuid, _>(user)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn seed_attendant(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let n = count(conn, "SELECT count(*) AS n FROM attendant_queues WHERE branch_id::text = $1 AND name = 'Support'", &[&s.branch_str])?;
    if n == 0 {
        sql_query(
            "INSERT INTO attendant_queues (id, org_id, bot_id, branch_id, name, description, priority, max_wait_minutes, auto_assign, working_hours, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'Support', 'Customer support queue', 1, 10, true, '{}'::jsonb, true, NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(s.org_id)
        .bind::<SqlUuid, _>(s.bot_id)
        .bind::<SqlUuid, _>(branch)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn seed_okr(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let branch = s.branch_id;
    let user = s.user_id;
    let n = count(conn, "SELECT count(*) AS n FROM okr_objectives WHERE branch_id::text = $1 AND title = 'Grow revenue 20%'", &[&s.branch_str])?;
    if n == 0 {
        sql_query(
            "INSERT INTO okr_objectives (id, org_id, bot_id, branch_id, owner_id, title, description, period, period_start, period_end, status, progress, visibility, weight, tags, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, 'Grow revenue 20%', 'Increase quarterly revenue', 'Q3', CURRENT_DATE, CURRENT_DATE + INTERVAL '90 days', 'on_track', 0, 'company', 1.0, ARRAY['growth'], NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(s.org_id)
        .bind::<SqlUuid, _>(s.bot_id)
        .bind::<SqlUuid, _>(branch)
        .bind::<SqlUuid, _>(user)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn seed_database(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    let user = s.user_id;
    let n = count(conn, "SELECT count(*) AS n FROM database_saved_queries WHERE name = 'Recent customers'", &[])?;
    if n == 0 {
        sql_query(
            "INSERT INTO database_saved_queries (id, bot_id, branch_id, user_id, name, query_text, description, is_shared, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'Recent customers', 'SELECT * FROM crm_contacts ORDER BY created_at DESC LIMIT 10;', 'Most recently added contacts', true, NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(s.bot_id)
        .bind::<SqlUuid, _>(s.branch_id)
        .bind::<SqlUuid, _>(user)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
