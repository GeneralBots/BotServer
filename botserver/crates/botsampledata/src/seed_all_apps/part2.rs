use super::*;

pub(super) fn seed_sales(conn: &mut diesel::PgConnection, _s: &Scopes) -> Result<(), String> {
    if !table_exists(conn, "sales_deals") {
        log::info!("botsampledata: sales_deals table not yet created; skipping sales seed");
        return Ok(());
    }
    let deals: &[(&str, f64, &str, f64)] = &[
        ("Acme Corporation expansion", 25000.0, "proposal", 0.6),
        ("Beta Inc onboarding", 12000.0, "negotiation", 0.8),
        ("Gamma renewal", 8000.0, "closed_won", 1.0),
    ];
    for (title, value, stage, prob) in deals {
        let n = count(conn, "SELECT count(*) AS n FROM sales_deals WHERE title = $1", &[title])?;
        if n == 0 {
            sql_query(
                "INSERT INTO sales_deals (id, title, value, stage, status, probability, created_at)
                 VALUES ($1, $2, $3, $4, 'open', $5, NOW())",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<Text, _>(title)
            .bind::<diesel::sql_types::Double, _>(value)
            .bind::<Text, _>(stage)
            .bind::<diesel::sql_types::Double, _>(prob)
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_hr(conn: &mut diesel::PgConnection, _s: &Scopes) -> Result<(), String> {
    if !table_exists(conn, "hr_employees") {
        log::info!("botsampledata: hr_employees table not yet created; skipping hr seed");
        return Ok(());
    }
    let n = count(conn, "SELECT count(*) AS n FROM hr_employees WHERE email = 'joana.hr@example.com'", &[])?;
    if n == 0 {
        sql_query(
            "INSERT INTO hr_employees (id, name, email, department, role, status, hired_at)
             VALUES ($1, 'Joana Lima', 'joana.hr@example.com', 'People', 'HR Analyst', 'active', NOW() - INTERVAL '200 days')",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    if table_exists(conn, "hr_recruitment") {
        let n2 = count(conn, "SELECT count(*) AS n FROM hr_recruitment WHERE position = 'Backend Engineer'", &[])?;
        if n2 == 0 {
            sql_query(
                "INSERT INTO hr_recruitment (id, position, department, status, candidates, opened_at)
                 VALUES ($1, 'Backend Engineer', 'Engineering', 'open', 4, NOW())",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    if table_exists(conn, "hr_review_cycles") {
        let nc = count(conn, "SELECT count(*) AS n FROM hr_review_cycles WHERE name = 'H1 Performance Review'", &[])?;
        if nc == 0 {
            sql_query(
                "INSERT INTO hr_review_cycles (id, name, start_date, end_date, status, completed, total)
                 VALUES ($1, 'H1 Performance Review', CURRENT_DATE - INTERVAL '90 days', CURRENT_DATE - INTERVAL '30 days', 'completed', 2, 3),
                        ($2, 'H2 Performance Review', CURRENT_DATE, CURRENT_DATE + INTERVAL '120 days', 'active', 1, 3)",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    if table_exists(conn, "hr_goals") {
        let ng = count(conn, "SELECT count(*) AS n FROM hr_goals WHERE title = 'Ship onboarding revamp'", &[])?;
        if ng == 0 {
            let emp_id = first_uuid(conn, "SELECT id FROM hr_employees WHERE email = 'joana.hr@example.com' LIMIT 1")?;
            let goal1 = Uuid::new_v4();
            let goal2 = Uuid::new_v4();
            sql_query(
                "INSERT INTO hr_goals (id, employee_id, title, completion, due_date)
                 VALUES ($1, $2, 'Ship onboarding revamp', 70, CURRENT_DATE + INTERVAL '60 days'),
                        ($3, $2, 'Reduce hiring cycle to 30 days', 40, CURRENT_DATE + INTERVAL '90 days')",
            )
            .bind::<SqlUuid, _>(goal1)
            .bind::<SqlUuid, _>(emp_id)
            .bind::<SqlUuid, _>(goal2)
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_tax(conn: &mut diesel::PgConnection, _s: &Scopes) -> Result<(), String> {
    if table_exists(conn, "brazil_nfe") {
        let n = count(conn, "SELECT count(*) AS n FROM brazil_nfe WHERE number = 'NFe-2026-0001'", &[])?;
        if n == 0 {
            sql_query(
                "INSERT INTO brazil_nfe (id, number, series, emitter_cnpj, recipient_cnpj, total, status, created_at)
                 VALUES ($1, 'NFe-2026-0001', '1', '00000000000191', '00000000000283', 1250.00, 'authorized', NOW() - INTERVAL '5 days')",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    if table_exists(conn, "brazil_nfse") {
        let n2 = count(conn, "SELECT count(*) AS n FROM brazil_nfse WHERE number = 'NFS-e-2026-001'", &[])?;
        if n2 == 0 {
            sql_query(
                "INSERT INTO brazil_nfse (id, number, service_code, provider_cnpj, total, status, created_at)
                 VALUES ($1, 'NFS-e-2026-001', '0101', '00000000000191', 480.00, 'issued', NOW() - INTERVAL '3 days')",
            )
            .bind::<SqlUuid, _>(Uuid::new_v4())
            .execute(conn)
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub(super) fn seed_kyc(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    // identity_kyc_workflows has a NOT NULL profile_id FK -> identity_profiles.
    let mut profile_id = Uuid::new_v4();
    let profile_n = count(conn, "SELECT count(*) AS n FROM identity_profiles WHERE legal_name = 'Carlos Demo'", &[])?;
    if profile_n == 0 {
        sql_query(
            "INSERT INTO identity_profiles (id, bot_id, person_id, branch_id, legal_name, tax_id, kyc_status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'Carlos Demo', '000.000.000-00', 'pending', NOW(), NOW())",
        )
        .bind::<SqlUuid, _>(profile_id)
        .bind::<SqlUuid, _>(s.bot_id)
        .bind::<SqlUuid, _>(s.user_id)
        .bind::<SqlUuid, _>(s.branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    } else {
        // Reuse an existing profile id.
        #[derive(diesel::QueryableByName)]
        struct P {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
        }
        let existing: Option<P> = sql_query(
            "SELECT id FROM identity_profiles WHERE legal_name = 'Carlos Demo' LIMIT 1",
        )
        .get_result(conn)
        .ok();
        if let Some(p) = existing {
            profile_id = p.id;
        }
    }

    let n = count(conn, "SELECT count(*) AS n FROM identity_kyc_workflows WHERE workflow_name = 'Standard Onboarding'", &[])?;
    if n == 0 {
        sql_query(
            "INSERT INTO identity_kyc_workflows (id, bot_id, profile_id, branch_id, workflow_name, current_step, steps_completed, total_steps, status, started_at)
             VALUES ($1, $2, $3, $4, 'Standard Onboarding', 'document_upload', '[]'::jsonb, 4, 'in_progress', NOW() - INTERVAL '1 day')",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(s.bot_id)
        .bind::<SqlUuid, _>(profile_id)
        .bind::<SqlUuid, _>(s.branch_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn seed_fraud(conn: &mut diesel::PgConnection, s: &Scopes) -> Result<(), String> {
    if !table_exists(conn, "fraud_transactions") {
        log::info!("botsampledata: fraud_transactions table not yet; skipping fraud seed");
        return Ok(());
    }
    let n = count(conn, "SELECT count(*) AS n FROM fraud_transactions WHERE amount = 350.00", &[])?;
    if n == 0 {
        sql_query(
            "INSERT INTO fraud_transactions (id, user_id, amount, currency, status, risk_score, created_at)
             VALUES ($1, $2, 350.00, 'BRL', 'flagged', 82, NOW() - INTERVAL '2 hours')",
        )
        .bind::<SqlUuid, _>(Uuid::new_v4())
        .bind::<SqlUuid, _>(s.user_id)
        .execute(conn)
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
