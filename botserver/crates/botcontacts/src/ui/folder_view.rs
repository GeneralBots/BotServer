//! CRM lead detail + embedded Folder view (issue #741).
//!
//! Deals whose `custom_fields.folder` points at a drive prefix (e.g.
//! `pragmatismo.gbai/pragmatismo.gbdrive/Proposals/2020/202007-BayerBot`)
//! render a live, read-only Folder tab that lists the objects under that
//! prefix. Drive stays the persistence; the view mirrors it — no duplication,
//! no re-import. Proposals without a folder render the plain detail.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{Html, IntoResponse},
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::CrmDeal;
use crate::schema::crm_deals;
use crate::CrateState;

pub async fn handle_lead_detail(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html("<div class='crm-detail-empty'>No detail available</div>".to_string());
    };
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| state.get_bot_context());
    let deal: Result<CrmDeal, _> = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn);
    let Ok(deal) = deal else {
        return Html("<div class='crm-detail-empty'>Deal not found</div>".to_string());
    };

    let title = deal.name.clone().unwrap_or_default();
    let folder = deal
        .custom_fields
        .as_ref()
        .and_then(|v| v.get("folder"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut html = String::new();
    html.push_str(&format!(
        r##"<div class="lead-detail">
<h3 class="lead-detail-title">{}</h3>
<div class="lead-detail-meta">
<span class="badge">{}</span>
<span class="text-muted">Branch scoped</span>
</div>
<div class="lead-detail-tabs">
<button class="crm-tab active" data-dtab="overview">Overview</button>
"##,
        crate::models::html_escape(&title),
        crate::models::html_escape(&deal.stage.clone().unwrap_or_else(|| "open".to_string())),
    ));

    html.push_str(r#"<div id="crm-detail-overview" class="crm-dtab-view active">"#);
    html.push_str(&detail_fields(&deal));
    html.push_str("</div>");

    // #1441 A5 — per-record activity timeline (calls, emails, meetings, notes).
    // Server-rendered from crm_activities so the detail page is self-contained.
    let activities: Vec<crate::models::CrmActivity> = crate::schema::crm_activities::table
        .filter(crate::schema::crm_activities::lead_id.eq(deal.id))
        .order(crate::schema::crm_activities::created_at.desc())
        .limit(20)
        .load(&mut conn)
        .unwrap_or_default();
    html.push_str(r#"<div class="lead-detail-timeline"><h4 class="lead-detail-subtitle">Activities</h4>"#);
    if activities.is_empty() {
        html.push_str(r#"<div class="lead-detail-empty-timeline">No activities recorded yet</div>"#);
    } else {
        html.push_str("<ul class=\"lead-timeline-list\">");
        for a in &activities {
            let due = a
                .due_date
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            html.push_str(&format!(
                r#"<li class="lead-timeline-item"><span class="lead-timeline-type">{}</span><span class="lead-timeline-subject">{}</span><span class="lead-timeline-date">{}</span></li>"#,
                crate::models::html_escape(&a.activity_type),
                crate::models::html_escape(a.subject.as_deref().unwrap_or("")),
                crate::models::html_escape(&due),
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str("</div>");

    if let Some(folder) = folder {
        let safe_folder = crate::models::html_escape(&folder);
        html.push_str(&format!(
            r##"
            <button class="crm-tab-btn" data-dtab="folder">Folder</button>
            </div>
            <div id="crm-detail-folder" class="crm-dtab-view" hidden>
            <div class="folder-view-head">
            <span class="folder-view-path" id="crm-folder-path">{}</span>
            </div>
            <div id="crm-folder-list" class="folder-view-list" data-folder-prefix="{}">
            <div class="folder-view-loading">Loading folder…</div>
            </div>
            </div>
            <script>
            (function(){{
              var box = document.getElementById('crm-folder-list');
              if (!box) return;
              var prefix = box.getAttribute('data-folder-prefix') || '';
              if (!prefix) {{ box.innerHTML = '<div class="folder-view-empty">No folder prefix configured</div>'; return; }}
              // Derive bucket from the prefix: <bucket>/<path>
              var parts = prefix.split('/');
              var extra = (window.CRM_BOT_BUCKET || '');
              var path = parts.slice(1).join('/');
              fetch('/api/files/list?bucket=' + encodeURIComponent(extra || parts[0]) +
                    '&path=' + encodeURIComponent(path) + '&scope=bot',
                    {{ headers: {{ 'Content-Type': 'application/json' }} }})
                .then(function (r) {{ return r.json(); }})
                .then(function (items) {{
                  if (!items || !items.length) {{ box.innerHTML = '<div class="empty-view-empty">No documents here yet</div>'; return; }}
                  var html = '<ul class="folder-view-items">';
                  items.forEach(function (it) {{
                    var icon = it.is_dir ? '&#128193;' : '&#128196;';
                    html += '<li class="folder-view-item' + (it.is_dir ? ' is-dir' : '') + '">' +
                      '<span class="folder-view-icon">' + icon + '</span>' +
                      '<span class="folder-view-name">' + (it.name || it.path || '') + '</span>' +
                      '<span class="folder-view-size">' + (it.size || it.size===0 ? it.size + ' B' : '') + '</span>' +
                      '</li>';
                  }});
                  html += '</ul>';
                  box.innerHTML = html;
                }})
                .catch(function (e) {{
                  box.innerHTML = '<div class="empty-view-empty">Failed to load folder: ' + e.message + '</div>';
                }});
            }})();
            </script>
            "##,
            safe_folder,
            crate::models::html_escape(&folder),
        ));
    } else {
        html.push_str("</div>");
    }

    html.push_str("</div>");
    Html(html)
}

fn deal_fields(deal: &CrmDeal) -> Vec<(&'static str, String)> {
    vec![
        ("Stage", deal.stage.clone().unwrap_or_else(|| "-".to_string())),
        ("Value", deal.value.map(|v| format!("{v}")).unwrap_or_else(|| "-".to_string())),
        ("Source", deal.source.clone().unwrap_or_default()),
    ]
}

fn detail_fields(deal: &CrmDeal) -> String {
    let mut html = String::new();
    for (label, value) in deal_fields(deal) {
        html.push_str(&format!(
            "<div class='detail-row'><span class='detail-label'>{}</span><span class='detail-value'>{}</span></div>",
            label,
            crate::models::html_escape(&value)
        ));
    }
    html
}