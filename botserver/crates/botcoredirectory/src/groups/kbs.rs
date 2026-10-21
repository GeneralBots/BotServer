use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;

/// GET /groups/{group_id}/kbs — returns an HTML fragment for the Knowledge Bases tab
pub async fn get_group_kbs(
    State(state): State<Arc<AppState>>,
    Path(group_id_str): Path<String>,
) -> Html<String> {
    let group_id = match Uuid::parse_str(&group_id_str) {
        Ok(uid) => uid,
        Err(_) => {
            return Html(format!(
                "<div class='alert alert-danger m-3 text-center'><div class='fs-3 mb-2'>🚫</div>Invalid Group ID Format: {}</div>",
                html_escape(&group_id_str)
            ));
        }
    };

    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;

        // 1. Get all KBs from kb_collections
        use botcore::shared::schema::kb_collections;
        let all_kbs = kb_collections::table
            .select((
                kb_collections::id,
                kb_collections::name,
                kb_collections::folder_path,
            ))
            .load::<(Uuid, String, String)>(&mut db_conn)
            .map_err(|e| format!("KB query error: {e}"))?;

        // 2. Get associated KB IDs for this group
        use botcore::shared::schema::kb_group_associations;
        let associated_ids: Vec<Uuid> = kb_group_associations::table
            .filter(kb_group_associations::group_id.eq(group_id))
            .select(kb_group_associations::kb_id)
            .load::<Uuid>(&mut db_conn)
            .map_err(|e| format!("Association query error: {e}"))?;

        Ok::<_, String>((all_kbs, associated_ids))
    })
    .await;

    match result {
        Ok(Ok((kbs, associated))) => {
            let mut html = String::from(
                r##"
<div class="kb-management">
    <div class="premium-header mb-4 p-4 border-bottom bg-white shadow-sm" style="border-radius: 16px 16px 0 0;">
        <h5 class="mb-1 text-primary d-flex align-items-center fw-bold">
            <span class="me-3 p-2 bg-primary bg-opacity-10 rounded-3">📚</span> Knowledge Base Permissions
        </h5>
        <p class="text-secondary small mb-0 mt-2">Specify which knowledge repositories are accessible to members of this group during interactive AI sessions.</p>
    </div>
    
    <div class="px-4 pb-4">
        <div class="kb-glass-container overflow-hidden">
            <table class="table table-hover align-middle mb-0 custom-kb-table">
                <thead>
                    <tr>
                        <th style="width: 100px;" class="text-center">Active</th>
                        <th>Knowledge Source</th>
                        <th>File Management Path</th>
                        <th class="text-end pe-4">Catalog ID</th>
                    </tr>
                </thead>
                <tbody>"##,
            );

            if kbs.is_empty() {
                html.push_str("<tr><td colspan='4' class='text-center py-5'><div class='text-muted opacity-50'><div style='font-size: 4rem' class='mb-3'>📂</div><p class='fs-5'>No Knowledge Bases indexed yet.</p><p class='small'>Mark folders as 'KB' in the Drive application to see them here.</p></div></td></tr>");
            } else {
                for (id, name, path) in kbs {
                    let is_checked = associated.contains(&id);
                    let checked_attr = if is_checked { "checked" } else { "" };
                    let status_badge = if is_checked { 
                        "<span class='badge bg-success-soft text-success border border-success border-opacity-10 px-2'>Active</span>" 
                    } else { 
                        "<span class='badge bg-light text-muted border px-2'>Inactive</span>" 
                    };
                    
                    html.push_str(&format!(
                        r##"
                    <tr class="{row_class}">
                        <td class="text-center">
                            <div class="form-check form-switch kb-switch-wrapper">
                                <input class="form-check-input premium-switch" type="checkbox" role="switch" 
                                       id="kb-sw-{id}" {checked_attr}
                                       hx-post="/groups/{group_id_str}/kbs/toggle/{id}"
                                       hx-trigger="change"
                                       hx-swap="none">
                            </div>
                        </td>
                        <td>
                            <div class="d-flex align-items-center">
                                <div class="kb-abstract-icon me-3">KB</div>
                                <div>
                                    <div class="fw-bold text-dark mb-0">{name}</div>
                                    <div class="status-indicator mt-1" id="st-badge-{id}">{status_badge}</div>
                                </div>
                            </div>
                        </td>
                        <td>
                            <div class="path-display d-flex align-items-center">
                                <span class=" me-2 small">📍</span>
                                <code class="text-primary-emphasis bg-primary bg-opacity-10 border-0">{path}</code>
                            </div>
                        </td>
                        <td class="text-end pe-4">
                            <span class="text-monospace text-muted small">{id_short}</span>
                        </td>
                    </tr>"##,
                        id = id,
                        id_short = id.to_string().chars().take(8).collect::<String>(),
                        name = name,
                        path = path,
                        checked_attr = checked_attr,
                        group_id_str = group_id_str,
                        status_badge = status_badge,
                        row_class = if is_checked { "kb-row-active" } else { "" }
                    ));
                }
            }

            html.push_str(
                r##"
                </tbody>
            </table>
        </div>
    </div>
</div>

<style>
.kb-management { animation: shiftUp 0.5s cubic-bezier(0.16, 1, 0.3, 1); }
.kb-glass-container { border-radius: 12px; border: 1px solid rgba(0,0,0,0.08); background: white; box-shadow: 0 4px 20px -5px rgba(0,0,0,0.05); }
.custom-kb-table thead th { background: #fcfcfd; border-bottom: 2px solid #f1f5f9; padding: 1.2rem 1rem; color: #475569; font-size: 0.75rem; letter-spacing: 0.05em; text-transform: uppercase; }
.custom-kb-table tbody tr { transition: all 0.2s ease; border-bottom: 1px solid #f1f5f9; }
.custom-kb-table tbody tr:last-child { border-bottom: none; }
.custom-kb-table tbody tr:hover { background-color: #f8fafc; }
.kb-row-active { background-color: #f0fdf4 !important; }

.premium-switch { width: 3.2rem !important; height: 1.6rem !important; cursor: pointer; transition: 0.3s; box-shadow: none !important; border-color: #cbd5e1; }
.premium-switch:checked { background-color: #10b981 !important; border-color: #10b981 !important; }
.kb-switch-wrapper { display: flex; justify-content: center; }

.kb-abstract-icon { width: 36px; height: 36px; background: #3b82f6; color: white; display: flex; align-items: center; justify-content: center; border-radius: 8px; font-size: 0.7rem; font-weight: 800; }
.path-display code { font-size: 0.8rem; padding: 0.25rem 0.6rem; border-radius: 6px; }
.bg-success-soft { background-color: rgba(16, 185, 129, 0.1); }
.text-monospace { font-family: 'JetBrains Mono', 'Fira Code', monospace; }

@keyframes shiftUp { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }
</style>

<script>
    document.querySelectorAll('.premium-switch').forEach(sw => {
        sw.addEventListener('change', function() {
            const id = this.id.replace('kb-sw-', '');
            const badgeEl = document.getElementById('st-badge-' + id);
            const row = this.closest('tr');
            
            if (this.checked) {
                badgeEl.innerHTML = "<span class='badge bg-success-soft text-success border border-success border-opacity-10 px-2'>Active</span>";
                row.classList.add('kb-row-active');
            } else {
                badgeEl.innerHTML = "<span class='badge bg-light text-muted border px-2'>Inactive</span>";
                row.classList.remove('kb-row-active');
            }
        });
    });
</script>"##,
            );
            Html(html)
        }
        Ok(Err(e)) => Html(format!(
            "<div class='alert alert-danger m-4 shadow-sm border-0 d-flex align-items-center'>
                <div class='me-3 fs-2'>⚠️</div>
                <div><strong>System Error:</strong><br><span class='small'>{}</span></div>
            </div>",
            e
        )),
        Err(e) => Html(format!(
            "<div class='alert alert-warning m-4 shadow-sm border-0 d-flex align-items-center'>
                <div class='me-3 fs-2'>⚙️</div>
                <div><strong>Task Interruption:</strong><br><span class='small'>{}</span></div>
            </div>",
            e
        )),
    }
}

/// POST /groups/{group_id}/kbs/toggle/{kb_id} — toggles KB access for a group
pub async fn toggle_group_kb(
    State(state): State<Arc<AppState>>,
    Path((group_id_str, kb_id_str)): Path<(String, String)>,
) -> impl IntoResponse {
    let group_id = match Uuid::parse_str(&group_id_str) {
        Ok(uid) => uid,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let kb_id = match Uuid::parse_str(&kb_id_str) {
        Ok(uid) => uid,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let conn = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<bool, String> {
        let mut db_conn = conn.get().map_err(|e| format!("DB error: {e}"))?;
        use botcore::shared::schema::kb_group_associations;

        let existing = kb_group_associations::table
            .filter(kb_group_associations::kb_id.eq(kb_id))
            .filter(kb_group_associations::group_id.eq(group_id))
            .select(kb_group_associations::id)
            .first::<Uuid>(&mut db_conn)
            .optional()
            .map_err(|e| format!("Query error: {e}"))?;

        if let Some(assoc_id) = existing {
            diesel::delete(
                kb_group_associations::table.filter(kb_group_associations::id.eq(assoc_id)),
            )
            .execute(&mut db_conn)
            .map_err(|e| format!("Delete error: {e}"))?;
            Ok(false) // Removed
        } else {
            diesel::insert_into(kb_group_associations::table)
                .values((
                    kb_group_associations::id.eq(Uuid::new_v4()),
                    kb_group_associations::kb_id.eq(kb_id),
                    kb_group_associations::group_id.eq(group_id),
                    kb_group_associations::granted_at.eq(Utc::now()),
                ))
                .execute(&mut db_conn)
                .map_err(|e| format!("Insert error: {e}"))?;
            Ok(true) // Added
        }
    })
    .await;

    match result {
        Ok(Ok(is_added)) => {
            if is_added {
                StatusCode::CREATED.into_response()
            } else {
                StatusCode::NO_CONTENT.into_response()
            }
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
