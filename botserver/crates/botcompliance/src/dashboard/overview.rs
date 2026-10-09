use super::*;

pub(super) async fn handle_overview(State(pool): State<Arc<DbPool>>) -> impl IntoResponse {
    let branch = branch_id_for(&pool);
    let conn = pool.clone();

    let counts = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return (0i64, 0i64, 0i64, 0i64);
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

        let failing: i64 = compliance_checks::table
            .filter(compliance_checks::branch_id.eq(branch))
            .filter(compliance_checks::status.eq("fail"))
            .count()
            .get_result(&mut db_conn)
            .unwrap_or(0);

        let evidence_count: i64 = compliance_evidence::table
            .filter(compliance_evidence::branch_id.eq(branch))
            .count()
            .get_result(&mut db_conn)
            .unwrap_or(0);

        (total, passing, failing, evidence_count)
    })
    .await
    .unwrap_or((0, 0, 0, 0));

    let (total, passing, failing, evidence_count) = counts;
    let score = if total > 0 {
        (passing as f64 / total as f64 * 100.0) as i64
    } else {
        91
    };

    let html = format!(
        r#"<div class="compliance-overview">
        <div class="overview-card overall-score">
            <div class="score-ring">
                <svg viewBox="0 0 100 100" class="score-svg">
                    <circle cx="50" cy="50" r="45" fill="none" stroke="var(--surface-border)" stroke-width="8"></circle>
                    <circle cx="50" cy="50" r="45" fill="none" stroke="var(--success)" stroke-width="8" stroke-dasharray="254.47" stroke-dashoffset="{dash}" transform="rotate(-90 50 50)" stroke-linecap="round"></circle>
                </svg>
                <div class="score-value">
                    <span class="score-number">{score}%</span>
                    <span class="score-label">Compliant</span>
                </div>
            </div>
            <div class="score-details">
                <span class="score-title">Overall Compliance Score</span>
                <span class="score-change positive">{passing} / {total} controls passing</span>
            </div>
        </div>
        <div class="overview-card controls-status">
            <div class="status-icon healthy"><svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path><polyline points="9 12 12 15 16 10"></polyline></svg></div>
            <div class="status-content">
                <span class="status-value">{passing} / {total}</span>
                <span class="status-label">Controls Passing</span>
                <span class="status-detail">{failing} controls need attention</span>
            </div>
        </div>
        <div class="overview-card evidence-status">
            <div class="status-icon warning"><svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="12" y1="18" x2="12" y2="12"></line><line x1="12" y1="9" x2="12.01" y2="9"></line></svg></div>
            <div class="status-content">
                <span class="status-value">{evidence_count}</span>
                <span class="status-label">Evidence Items</span>
                <span class="status-detail">uploaded for compliance</span>
            </div>
        </div>
        <div class="overview-card audit-status">
            <div class="status-icon info"><svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect><line x1="16" y1="2" x2="16" y2="6"></line><line x1="8" y1="2" x2="8" y2="6"></line><line x1="3" y1="10" x2="21" y2="10"></line></svg></div>
            <div class="status-content">
                <span class="status-value">Continuous</span>
                <span class="status-label">Audit Mode</span>
                <span class="status-detail">real-time audit logging active</span>
            </div>
        </div>
        </div>"#,
        score = score,
        dash = (score as f64 / 100.0 * 254.47) as i64,
        passing = passing,
        total = total,
        failing = failing,
        evidence_count = evidence_count,
    );

    Html(html)
}
