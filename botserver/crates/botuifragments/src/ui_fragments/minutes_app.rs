/* =============================================================================
 * ui_fragments/minutes_app.rs — meetings, transcripts, documents
 * =============================================================================*/
use super::*;
use axum::Json;
use axum::http::HeaderMap;
use crate::db;
use botminutes::handlers as minutes;

use super::minutes_app_forms::{complete_action, ensure_actions_table, update_document};

pub fn configure<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/suite/minutes/fragments/upcoming", get(upcoming))
        .route("/suite/minutes/fragments/live", get(live))
        .route("/suite/minutes/fragments/transcripts", get(transcripts))
        .route("/suite/minutes/fragments/documents", get(documents))
        .route("/suite/minutes/fragments/actions", get(actions))
        .route("/suite/minutes/fragments/templates", get(templates))
        .route("/suite/minutes/fragments/signatures", get(signatures))
        .route("/suite/minutes/fragments/attendance/:id", get(attendance))
        .route("/api/minutes/forms/action/:id/done", post(complete_action))
        .route("/api/minutes/forms/document/:id", post(update_document))
}

async fn upcoming() -> Result<Html<String>, (StatusCode, String)> {
    match minutes::list_meetings(HeaderMap::new()).await {
        Ok(Json(value)) => {
            let items = value.get("items").cloned().unwrap_or(serde_json::Value::Array(vec![]));
            if let Some(arr) = items.as_array() {
                if arr.is_empty() {
                    return Ok(Html(render_meetings_empty()));
                }
                let cards = arr.iter().filter_map(|m| {
                    let id = m.get("id")?.as_str()?.to_string();
                    let title = m.get("title")?.as_str().unwrap_or("").to_string();
                    let date = m.get("date")?.as_str().unwrap_or("").to_string();
                    let time = m.get("time")?.as_str().unwrap_or("").to_string();
                    let dur = m.get("duration_minutes").and_then(|v| v.as_u64()).unwrap_or(30);
                    let loc = m.get("location").and_then(|v| v.as_str()).unwrap_or("Online").to_string();
                    let status = m.get("status").and_then(|v| v.as_str()).unwrap_or("scheduled").to_string();
                    let parts: Vec<String> = m.get("participants")
                        .and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|p| p.get("name").and_then(|n| n.as_str()).map(String::from)).collect())
                        .unwrap_or_default();
                    Some(render_meeting_card(&MeetingCard {
                        id: &id,
                        title: &title,
                        date: &date,
                        time: &time,
                        duration_minutes: dur,
                        location: &loc,
                        status: &status,
                        participants: &parts,
                    }))
                }).collect::<String>();
                Ok(Html(format!(
                    r##"<div class="min-upcoming" hx-get="/suite/minutes/fragments/upcoming" hx-trigger="every 60s" hx-swap="outerHTML">
  <header class="gb-toolbar">
<button class="gb-btn gb-btn-primary" hx-get="/suite/minutes/forms/meeting-modal" hx-target="#min-meeting-modal" hx-swap="innerHTML">+ Agendar reunião</button>
  </header>
  <div class="min-meeting-grid">{cards}</div>
</div>"##,
                    cards = cards,
                )))
            } else {
                Ok(Html(render_meetings_empty()))
            }
        }
        Err((_, e)) => Ok(Html(err_fragment(&format!("meetings: {e}")))),
    }
}

fn render_meetings_empty() -> String {
    r##"<div class="gb-empty">
  <div class="gb-empty-icon">📅</div>
  <h3>Nenhuma reunião agendada</h3>
  <p>Use o botão acima para criar uma.</p>
</div>"##.to_string()
}

/// One meeting as the minutes dashboard renders it.
#[derive(Clone, Copy)]
struct MeetingCard<'a> {
    id: &'a str,
    title: &'a str,
    date: &'a str,
    time: &'a str,
    duration_minutes: u64,
    location: &'a str,
    status: &'a str,
    participants: &'a [String],
}

fn render_meeting_card(card: &MeetingCard<'_>) -> String {
    let MeetingCard {
        id,
        title,
        date,
        time,
        duration_minutes: dur,
        location: loc,
        status,
        participants: parts,
    } = *card;
    let participants = if parts.is_empty() {
        String::new()
    } else {
        format!(
            r#"<div class="min-participant-list">{}</div>"#,
            parts.iter().map(|p| format!(r#"<span class="min-participant">{}</span>"#, htmx_escape(p))).collect::<String>()
        )
    };
    let cls = match status { "active" => "active", "completed" => "completed", _ => "scheduled" };
    format!(
        r##"<article class="min-card" data-id="{id}">
  <header><h4>{title}</h4><span class="min-badge {cls}">{status}</span></header>
  <dl>
<dt>Data</dt><dd>{date}</dd>
<dt>Hora</dt><dd>{time} ({dur} min)</dd>
<dt>Local</dt><dd>{loc}</dd>
  </dl>
  {participants}
  <footer class="min-card-actions">
<button class="gb-btn gb-btn-primary" hx-post="/api/minutes/forms/meeting/start/{id}" hx-swap="none">Iniciar</button>
<button class="gb-btn" hx-get="/suite/minutes/fragments/attendance/{id}" hx-target="#min-attendance-modal" hx-swap="innerHTML">Presença</button>
  </footer>
</article>"##,
        id = htmx_escape(id),
        title = htmx_escape(title),
        cls = cls,
        status = htmx_escape(status),
        date = htmx_escape(date),
        time = htmx_escape(time),
        dur = dur,
        loc = htmx_escape(loc),
        participants = participants,
    )
}

async fn live() -> Result<Html<String>, (StatusCode, String)> {
    Ok(Html(r##"<section class="min-live" hx-get="/suite/minutes/fragments/live" hx-trigger="every 5s" hx-swap="outerHTML">
  <header class="min-live-header">
<h3>Reunião em andamento</h3>
<span class="min-rec-dot"></span>
<span id="min-timer">00:00</span>
<button class="gb-btn" hx-post="/api/minutes/forms/recording/toggle" hx-swap="none">Pausar</button>
  </header>
  <div class="min-live-grid">
<article class="min-live-transcript">
  <p><time>14:02</time> Alice: Vamos começar pela revisão do Q2.</p>
  <p><time>14:03</time> Bruno: Os números de maio ficaram 12% acima da meta.</p>
  <p class="min-live-placeholder">… transcrição em tempo real</p>
</article>
<aside class="min-live-side">
  <h4>Participantes</h4>
  <ul class="min-participants-list">
    <li class="online">Alice Silva</li>
    <li class="online">Bruno Costa</li>
    <li class="muted">Carla Souza</li>
  </ul>
  <h4>Resumo IA</h4>
  <ul class="min-ai-actions">
    <li>Definir meta de Q2</li>
    <li>Revisar budget de marketing</li>
    <li>Escalar caso PR-1234</li>
  </ul>
</aside>
  </div>
</section>"##.to_string()))
}

async fn transcripts() -> Result<Html<String>, (StatusCode, String)> {
    match minutes::list_transcripts(HeaderMap::new()).await {
        Ok(Json(value)) => {
            let items = value.get("items").cloned().unwrap_or(serde_json::Value::Array(vec![]));
            let arr = items.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                return Ok(Html(r#"<div class="gb-empty">Nenhuma transcrição disponível.</div>"#.to_string()));
            }
            let cards = arr.iter().filter_map(|t| {
                let meeting = t.get("meeting_id")?.as_str()?.to_string();
                let content = t.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let created = t.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let wc = t.get("word_count").and_then(|v| v.as_u64()).unwrap_or(0);
                Some(format!(
                    r##"<article class="min-card">
  <header><h4>Reunião {meeting}</h4><span class="min-badge active">{wc} palavras</span></header>
  <p class="min-muted">{created}</p>
  <pre class="min-transcript">{content}</pre>
</article>"##,
                    meeting = htmx_escape(&meeting),
                    wc = wc,
                    created = htmx_escape(&created),
                    content = htmx_escape(&content),
                ))
            }).collect::<String>();
            Ok(Html(format!(
                r##"<div class="min-transcripts" hx-get="/suite/minutes/fragments/transcripts" hx-trigger="every 60s" hx-swap="outerHTML">
  <header class="gb-toolbar">
<input type="search" name="q" placeholder="Buscar nas transcrições…" hx-get="/suite/minutes/fragments/transcripts" hx-trigger="keyup delay:300ms changed" hx-target="#min-transcripts-body" hx-swap="innerHTML" class="gb-search">
<button class="gb-btn">Exportar CSV</button>
  </header>
  <div id="min-transcripts-body" class="min-card-grid">{cards}</div>
</div>"##,
                cards = cards,
            )))
        }
        Err((_, e)) => Ok(Html(err_fragment(&format!("transcripts: {e}")))),
    }
}

async fn documents() -> Result<Html<String>, (StatusCode, String)> {
    match minutes::list_documents(HeaderMap::new()).await {
        Ok(Json(value)) => {
            let items = value.get("items").cloned().unwrap_or(serde_json::Value::Array(vec![]));
            let arr = items.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                return Ok(Html(r#"<div class="gb-empty">Nenhuma ata gerada.</div>"#.to_string()));
            }
            let rows = arr.iter().filter_map(|d| {
                let id = d.get("id")?.as_str()?.to_string();
                let title = d.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let status = d.get("status").and_then(|v| v.as_str()).unwrap_or("draft").to_string();
                let v = d.get("version").and_then(|x| x.as_u64()).unwrap_or(1);
                let updated = d.get("updated_at").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let cls = if status == "approved" { "ok" } else if status == "signed" { "purple" } else { "warn" };
                Some(format!(
                    r##"<tr>
<td>{title}</td><td>v{v}</td><td><span class="gb-badge {cls}">{status}</span></td><td>{updated}</td>
<td>
  <button class="gb-btn" hx-get="/suite/minutes/forms/edit/{id}" hx-target="#min-doc-modal" hx-swap="innerHTML">Editar</button>
  <button class="gb-btn gb-btn-success" hx-post="/api/minutes/forms/document/{id}/approve" hx-swap="none" hx-trigger="click" hx-on::after-request="htmx.ajax('GET', '/suite/minutes/fragments/documents', '#min-documents-body')">Aprovar</button>
</td>
</tr>"##,
                    id = htmx_escape(&id),
                    title = htmx_escape(&title),
                    v = v,
                    status = htmx_escape(&status),
                    cls = cls,
                    updated = htmx_escape(&updated),
                ))
            }).collect::<String>();
            Ok(Html(format!(
                r##"<div class="min-documents" hx-get="/suite/minutes/fragments/documents" hx-trigger="every 30s" hx-swap="outerHTML">
  <header class="gb-toolbar">
<input type="search" name="q" placeholder="Buscar atas…" class="gb-search" hx-get="/suite/minutes/fragments/documents" hx-trigger="keyup delay:300ms changed" hx-target="#min-documents-body" hx-swap="innerHTML">
  </header>
  <div id="min-documents-body" class="gb-table-wrapper"><table class="gb-table">
<thead><tr><th>Título</th><th>Versão</th><th>Status</th><th>Atualizada</th><th>Ações</th></tr></thead>
<tbody>{rows}</tbody>
  </table></div>
</div>"##,
                rows = rows,
            )))
        }
        Err((_, e)) => Ok(Html(err_fragment(&format!("documents: {e}")))),
    }
}

async fn actions() -> Result<Html<String>, (StatusCode, String)> {
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool: {e}")))?;
    ensure_actions_table(&mut conn).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] title: String,
        #[diesel(sql_type = diesel::sql_types::Text)] owner: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Date>)] due: Option<chrono::NaiveDate>,
        #[diesel(sql_type = diesel::sql_types::Text)] priority: String,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, title, owner, due, priority, status FROM minutes_actions ORDER BY created_at DESC LIMIT 200",
    )
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    if rows.is_empty() {
        return Ok(Html(r##"<div class="min-actions" hx-get="/suite/minutes/fragments/actions" hx-trigger="every 30s" hx-swap="outerHTML">
  <header class="gb-toolbar">
<button class="gb-btn gb-btn-primary" hx-get="/suite/minutes/forms/action-modal" hx-target="#min-action-modal" hx-swap="innerHTML">+ Nova ação</button>
  </header>
  <p class="gb-empty">Nenhuma ação cadastrada.</p>
</div>"##.to_string()));
    }
    let body = rows.iter().map(|r| {
        let due = r.due.map(|d| d.to_string()).unwrap_or_else(|| "—".into());
        let cls = if r.status == "done" { "ok" } else if r.priority == "critical" { "err" } else { "warn" };
        let action = if r.status != "done" {
            format!(
                r#"<button class="gb-btn" hx-post="/api/minutes/forms/action/{id}/done" hx-swap="none" hx-trigger="click" hx-on::after-request="htmx.ajax('GET', '/suite/minutes/fragments/actions', '#min-actions-body')">Concluir</button>"#,
                id = r.id,
            )
        } else { String::from("<span class=\"gb-ok\">✓</span>") };
        format!(
            r##"<tr>
<td>{title}</td><td>{owner}</td><td>{due}</td>
<td><span class="gb-badge {cls}">{priority}</span></td>
<td><span class="gb-badge {status_cls}">{status}</span></td>
<td>{action}</td>
</tr>"##,
            title = htmx_escape(&r.title),
            owner = htmx_escape(&r.owner),
            due = htmx_escape(&due),
            cls = cls,
            priority = htmx_escape(&r.priority),
            status_cls = if r.status == "done" { "ok" } else { "warn" },
            status = htmx_escape(&r.status),
            action = action,
        )
    }).collect::<String>();
    Ok(Html(format!(
        r##"<div class="min-actions" hx-get="/suite/minutes/fragments/actions" hx-trigger="every 30s" hx-swap="outerHTML">
  <header class="gb-toolbar">
<button class="gb-btn gb-btn-primary" hx-get="/suite/minutes/forms/action-modal" hx-target="#min-action-modal" hx-swap="innerHTML">+ Nova ação</button>
<input type="search" name="q" placeholder="Filtrar…" class="gb-search" hx-get="/suite/minutes/fragments/actions" hx-trigger="keyup delay:300ms changed" hx-target="#min-actions-body" hx-swap="innerHTML">
  </header>
  <div id="min-actions-body" class="gb-table-wrapper"><table class="gb-table">
<thead><tr><th>Título</th><th>Responsável</th><th>Prazo</th><th>Prioridade</th><th>Status</th><th>Ações</th></tr></thead>
<tbody>{body}</tbody>
  </table></div>
</div>"##,
        body = body,
    )))
}

async fn templates() -> Result<Html<String>, (StatusCode, String)> {
    Ok(Html(r##"<div class="min-templates" hx-get="/suite/minutes/fragments/templates" hx-trigger="every 5m" hx-swap="outerHTML">
  <div class="min-template-grid">
<article class="min-template-card">
  <h4>☕ Stand-up diário</h4>
  <p class="min-muted">Duração: 15 min</p>
  <ul><li>O que fiz ontem</li><li>O que farei hoje</li><li>Impedimentos</li></ul>
  <button class="gb-btn gb-btn-primary" hx-post="/api/minutes/forms/template/standup" hx-swap="none">Usar template</button>
</article>
<article class="min-template-card">
  <h4>🤝 1:1 trimestral</h4>
  <p class="min-muted">Duração: 45 min</p>
  <ul><li>Conquistas e desafios</li><li>Metas de carreira</li><li>Feedback mútuo</li></ul>
  <button class="gb-btn gb-btn-primary" hx-post="/api/minutes/forms/template/1on1" hx-swap="none">Usar template</button>
</article>
<article class="min-template-card">
  <h4>🔄 Retrospectiva</h4>
  <p class="min-muted">Duração: 60 min</p>
  <ul><li>O que foi bem</li><li>O que melhorar</li><li>Ações</li></ul>
  <button class="gb-btn gb-btn-primary" hx-post="/api/minutes/forms/template/retro" hx-swap="none">Usar template</button>
</article>
<article class="min-template-card">
  <h4>🏛️ Board meeting</h4>
  <p class="min-muted">Duração: 90 min</p>
  <ul><li>Update do CEO</li><li>Revisão financeira</li><li>Decisões estratégicas</li></ul>
  <button class="gb-btn gb-btn-primary" hx-post="/api/minutes/forms/template/board" hx-swap="none">Usar template</button>
</article>
  </div>
</div>"##.to_string()))
}

async fn signatures() -> Result<Html<String>, (StatusCode, String)> {
    Ok(Html(r##"<div class="gb-table-wrapper"><table class="gb-table">
<thead><tr><th>Documento</th><th>Signatários</th><th>Status</th><th>Ações</th></tr></thead>
<tbody>
<tr><td>Ata Q1 2025</td><td>alice, bruno, carla</td><td><span class="gb-badge warn">pendente</span></td><td><button class="gb-btn gb-btn-primary" hx-get="/suite/minutes/forms/sign-pad/doc-001" hx-target="#min-sign-modal" hx-swap="innerHTML">Assinar</button></td></tr>
<tr><td>Ata Q4 2024</td><td>alice, bruno</td><td><span class="gb-badge ok">assinado</span></td><td>—</td></tr>
</tbody></table></div>"##.to_string()))
}

async fn attendance(Path(id): Path<String>) -> Result<Html<String>, (StatusCode, String)> {
    Ok(Html(format!(
        r##"<div class="min-attendance" hx-post="/api/minutes/forms/attendance/{id}" hx-swap="outerHTML">
  <h3>Confirmação de presença</h3>
  <ul class="min-att-list">
<li><label><input type="checkbox" name="alice" checked> Alice Silva</label></li>
<li><label><input type="checkbox" name="bruno" checked> Bruno Costa</label></li>
<li><label><input type="checkbox" name="carla"> Carla Souza</label></li>
<li><label><input type="checkbox" name="diego"> Diego Lima</label></li>
  </ul>
  <footer class="gb-form-actions">
<button type="button" class="gb-btn" onclick="document.getElementById('min-attendance-modal').style.display='none'">Fechar</button>
<button type="submit" class="gb-btn gb-btn-primary">Salvar presença</button>
  </footer>
</div>"##,
        id = htmx_escape(&id),
    )))
}
