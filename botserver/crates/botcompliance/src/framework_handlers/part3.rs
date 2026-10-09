use super::*;

/// `DELETE /api/compliance/evidence/:evidence_id`
pub async fn handle_delete_evidence(
    State(pool): State<Arc<crate::DbPool>>,
    Path(evidence_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;

        diesel::delete(compliance_control_evidence::table.find(evidence_id))
            .execute(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Evidence not found".to_string()))?;

        audit(
            &mut conn,
            BRANCH_ID_PLACEHOLDER,
            None,
            "evidence.delete",
            "compliance_control_evidence",
            Some(evidence_id),
            None,
        );

        Ok::<_, ComplianceError>(serde_json::json!({ "ok": true, "id": evidence_id }))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
/// `GET /api/compliance/frameworks/:framework_id/export.csv`
///
/// Audit-ready CSV scorecard: control id, title, category, mandatory flag,
/// evidence status and coverage, plus framework header rows.
pub async fn handle_export_framework_csv(
    State(pool): State<Arc<crate::DbPool>>,
    Path(framework_id): Path<Uuid>,
) -> Result<axum::response::Response, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;

        let fw: DbComplianceFramework = compliance_frameworks::table
            .find(framework_id)
            .first(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Framework not found".to_string()))?;

        let controls: Vec<DbComplianceControl> = compliance_controls::table
            .filter(compliance_controls::framework_id.eq(framework_id))
            .order(compliance_controls::control_id.asc())
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let control_ids: Vec<Uuid> = controls.iter().map(|c| c.id).collect();
        let evidence_rows: Vec<DbComplianceControlEvidence> = if control_ids.is_empty() {
            Vec::new()
        } else {
            compliance_control_evidence::table
                .filter(compliance_control_evidence::control_id.eq_any(&control_ids))
                .load(&mut conn)
                .map_err(|e| ComplianceError::Database(e.to_string()))?
        };

        let mut evidence_by_control: std::collections::HashMap<Uuid, Vec<&DbComplianceControlEvidence>> =
            std::collections::HashMap::new();
        for ev in &evidence_rows {
            evidence_by_control.entry(ev.control_id).or_default().push(ev);
        }

        let mut csv = String::new();
        csv.push_str(&format!(
            "Compliance Scorecard,{},{},v{}\n",
            fw.name, fw.framework_key, fw.version
        ));
        csv.push_str("Generated at,");
        csv.push_str(&Utc::now().to_rfc3339());
        csv.push('\n');
        csv.push('\n');
        csv.push_str("control_id,title,category,mandatory,evidence_count,approved,status\n");

        let mut covered = 0usize;
        for c in &controls {
            let evs = evidence_by_control.get(&c.id).cloned().unwrap_or_default();
            let approved = evs.iter().filter(|e| e.status == "approved").count();
            let has_evidence = approved > 0;
            if has_evidence {
                covered += 1;
            }
            let title = c.title.replace(',', " ").replace('\n', " ");
            let category = c.category.clone().unwrap_or_default().replace(',', " ");
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                c.control_id,
                title,
                category,
                c.is_mandatory,
                evs.len(),
                approved,
                c.status
            ));
        }

        let total = controls.len();
        let coverage = if total > 0 {
            format!("{:.2}%", covered as f64 / total as f64 * 100.0)
        } else {
            "0.00%".to_string()
        };
        csv.push('\n');
        csv.push_str(&format!("Total controls,{total}\n"));
        csv.push_str(&format!("Controls with approved evidence,{covered}\n"));
        csv.push_str(&format!("Coverage,{coverage}\n"));

        let fname = format!("compliance-{}-{}.csv", fw.framework_key, fw.version);
        let headers = [
            (
                axum::http::header::CONTENT_TYPE,
                "text/csv; charset=utf-8".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{fname}\""),
            ),
        ];

        Ok::<_, ComplianceError>((
            headers,
            csv,
        ))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(result.into_response())
}
/// Server-rendered framework detail view, reachable from the suite Compliance app.
/// Resolves the framework by name (falling back to its `framework_key`) and lists
/// every control together with its collected evidence, so agents get an audit-ready
/// page without leaving the desktop shell.
pub async fn handle_framework_detail_page(
    State(pool): State<Arc<crate::DbPool>>,
    Path(name): Path<String>,
) -> Html<String> {
    let result = tokio::task::spawn_blocking(move || -> Result<String, ComplianceError> {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let fw: DbComplianceFramework = compliance_frameworks::table
            .filter(compliance_frameworks::name.eq(&name))
            .first(&mut conn)
            .or_else(|_| -> Result<DbComplianceFramework, diesel::result::Error> {
                compliance_frameworks::table
                    .filter(compliance_frameworks::framework_key.eq(&name))
                    .first(&mut conn)
            })
            .map_err(|_| ComplianceError::NotFound("Framework not found".to_string()))?;

        let controls: Vec<DbComplianceControl> = compliance_controls::table
            .filter(compliance_controls::framework_id.eq(fw.id))
            .order(compliance_controls::control_id.asc())
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let control_ids: Vec<Uuid> = controls.iter().map(|c| c.id).collect();

        let evidence_rows: Vec<DbComplianceControlEvidence> = if control_ids.is_empty() {
            Vec::new()
        } else {
            compliance_control_evidence::table
                .filter(compliance_control_evidence::control_id.eq_any(&control_ids))
                .load(&mut conn)
                .map_err(|e| ComplianceError::Database(e.to_string()))?
        };

        let mut evidence_count: std::collections::HashMap<Uuid, usize> = std::collections::HashMap::new();
        for ev in &evidence_rows {
            *evidence_count.entry(ev.control_id).or_insert(0) += 1;
        }

        let total = controls.len();
        let covered = controls
            .iter()
            .filter(|c| evidence_count.get(&c.id).copied().unwrap_or(0) > 0)
            .count();
        let coverage = if total > 0 {
            (covered as f64 / total as f64 * 100.0).round()
        } else {
            0.0
        };

        let mut rows = String::new();
        for c in &controls {
            let cnt = evidence_count.get(&c.id).copied().unwrap_or(0);
            let badge = if cnt > 0 {
                "<span class=\"cv-badge ok\">Covered</span>"
            } else {
                "<span class=\"cv-badge missing\">No evidence</span>"
            };
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&c.control_id),
                escape_html(&c.title),
                escape_html(c.category.as_deref().unwrap_or("")),
                cnt,
                badge
            ));
        }

        let html = format!(
            "<div class=\"framework-detail\">\
                <div class=\"framework-detail-header\">\
                    <h2>{}</h2>\
                    <p class=\"framework-detail-desc\">{}</p>\
                    <div class=\"framework-detail-coverage\">Coverage: {}% ({}/{} controls with evidence)</div>\
                </div>\
                <table class=\"framework-controls-table\">\
                    <thead><tr><th>Control</th><th>Title</th><th>Category</th><th>Evidence</th><th>Status</th></tr></thead>\
                    <tbody>{}</tbody>\
                </table>\
            </div>",
            escape_html(&fw.name),
            escape_html(fw.description.as_deref().unwrap_or("")),
            coverage,
            covered,
            total,
            rows
        );
        Ok(html)
    });

    match result.await {
        Ok(Ok(html)) => Html(html),
        Ok(Err(e)) => Html(format!(
            "<div class=\"framework-detail-error\">{}</div>",
            escape_html(&e.to_string())
        )),
        Err(e) => Html(format!(
            "<div class=\"framework-detail-error\">{}</div>",
            escape_html(&e.to_string())
        )),
    }
}
pub(super) fn escape_html(text: &str) -> String {
    let mut s = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '"' => s.push_str("&quot;"),
            '\'' => s.push_str("&#39;"),
            _ => s.push(ch),
        }
    }
    s
}
