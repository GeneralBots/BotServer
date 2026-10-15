use super::*;

impl Scopes {
    pub(super) fn new(conn: &mut diesel::PgConnection) -> Result<Self, String> {
        let sample: SampleScope = ensure_sample_scope(conn)?;
        Ok(Scopes {
            org_id: sample.org_id,
            branch_id: sample.branch_id,
            bot_id: sample.bot_id,
            user_id: sample.user_id,
            org_str: sample.org_str,
            branch_str: sample.branch_str,
            bot_str: sample.bot_str,
            user_str: sample.user_str,
        })
    }
}
/// A seed domain: its name and the function that populates it.
type SeedDomain = (
    &'static str,
    fn(&mut diesel::PgConnection, &Scopes) -> Result<(), String>,
);

/// Seed every domain. Each domain runs independently: a failure in one app
/// (e.g. a legacy/missing table on an un-migrated prod schema) is logged and
/// the remaining domains still seed.
pub fn seed(conn: &mut diesel::PgConnection) -> Result<(), String> {
    let scopes = Scopes::new(conn)?;

    let domains: Vec<SeedDomain> = vec![
        ("people", seed_people),
        ("crm", seed_crm),
        ("tickets", seed_tickets),
        ("billing", seed_billing),
        ("tasks", seed_tasks),
        ("calendar", seed_calendar),
        ("research", seed_research),
        ("compliance", seed_compliance),
        ("goals", seed_goals),
        ("workspaces", seed_workspaces),
        ("social", seed_social),
        ("marketing", seed_marketing),
        ("o365", seed_m365),
        ("drive", seed_drive),
        ("fiscal", seed_fiscal),
    ];

    for (name, seed_fn) in domains {
        match seed_fn(conn, &scopes) {
            Ok(()) => log::info!("botsampledata: {name} seeded"),
            Err(e) => log::error!("botsampledata: {name} seeding failed: {e}"),
        }
    }

    // Extended surface (products, dashboards, meet, learn, project, canvas,
    // attendant, OKR, database, integrations, sources, monitoring).
    match crate::seed_all_apps::seed(conn, &scopes) {
        Ok(()) => log::info!("botsampledata: extended app surface seeded"),
        Err(e) => log::error!("botsampledata: extended app seeding failed: {e}"),
    }

    Ok(())
}
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
        _ => return Err("count: too many binds".to_string()),
    };
    r.map(|r| r.n).map_err(|e| e.to_string())
}
