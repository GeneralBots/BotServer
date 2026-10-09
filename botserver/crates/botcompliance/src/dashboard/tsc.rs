use super::*;

pub(super) async fn handle_tsc(State(pool): State<Arc<DbPool>>) -> impl IntoResponse {
    let branch = branch_id_for(&pool);
    let conn = pool.clone();

    let counts = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return (0i64, 0i64);
            }
        };
        let total: i64 = compliance_checks::table
            .filter(compliance_checks::branch_id.eq(branch))
            .count()
            .get_result(&mut db_conn)
            .unwrap_or(0);
        let passing: i64 = compliance_checks::table
            .filter(compliance_checks::branch_id.eq(branch))
            .filter(compliance_checks::status.eq("pass"))
            .count()
            .get_result(&mut db_conn)
            .unwrap_or(0);
        (total, passing)
    })
    .await
    .unwrap_or((0, 0));

    let (total, passing) = counts;
    let pct = if total > 0 { (passing as f64 / total as f64 * 100.0) as i64 } else { 0 };
    let failing = total - passing;

    let html = format!(
        r#"<div class="tsc-grid">
        <div class="tsc-category" onclick="showTscDetails('security')">
            <div class="tsc-header"><div class="tsc-icon security"><svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" stroke-width="2" fill="none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path></svg></div><span class="tsc-name">Security</span></div>
            <div class="tsc-progress"><div class="tsc-bar"><div class="tsc-fill passing" style="width: {pct}%"></div></div><span class="tsc-percent">{pct}%</span></div>
            <div class="tsc-counts"><span class="passing">{passing} passing</span><span class="failing">{failing} failing</span></div>
        </div>
        <div class="tsc-category" onclick="showTscDetails('availability')">
            <div class="tsc-header"><div class="tsc-icon availability"><svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" stroke-width="2" fill="none"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg></div><span class="tsc-name">Availability</span></div>
            <div class="tsc-progress"><div class="tsc-bar"><div class="tsc-fill passing" style="width: {pct}%"></div></div><span class="tsc-percent">{pct}%</span></div>
            <div class="tsc-counts"><span class="passing">{passing} passing</span><span class="failing">{failing} failing</span></div>
        </div>
        <div class="tsc-category" onclick="showTscDetails('confidentiality')">
            <div class="tsc-header"><div class="tsc-icon confidentiality"><svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" stroke-width="2" fill="none"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect><path d="M7 11V7a5 5 0 0 1 10 0v4"></path></svg></div><span class="tsc-name">Confidentiality</span></div>
            <div class="tsc-progress"><div class="tsc-bar"><div class="tsc-fill passing" style="width: {pct}%"></div></div><span class="tsc-percent">{pct}%</span></div>
            <div class="tsc-counts"><span class="passing">{passing} passing</span><span class="failing">{failing} failing</span></div>
        </div>
        <div class="tsc-category" onclick="showTscDetails('integrity')">
            <div class="tsc-header"><div class="tsc-icon integrity"><svg viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" stroke-width="2" fill="none"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg></div><span class="tsc-name">Integrity</span></div>
            <div class="tsc-progress"><div class="tsc-bar"><div class="tsc-fill passing" style="width: {pct}%"></div></div><span class="tsc-percent">{pct}%</span></div>
            <div class="tsc-counts"><span class="passing">{passing} passing</span><span class="failing">{failing} failing</span></div>
        </div>
        </div>"#
    );

    Html(html)
}
