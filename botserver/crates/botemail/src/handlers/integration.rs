use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use diesel::prelude::*;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::AppState;
use crate::types::*;

pub async fn get_feature_flags(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<FeatureFlags>, StatusCode> {
    let mut conn = state.pool.get().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    #[derive(QueryableByName)]
    struct FlagRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        feature: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        enabled: bool,
    }

    let flags: Vec<FlagRow> = diesel::sql_query(
        "SELECT feature, enabled FROM feature_flags WHERE org_id = $1"
    )
    .bind::<diesel::sql_types::Uuid, _>(org_id)
    .load(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let crm_enabled = flags.iter().any(|f| f.feature == "crm" && f.enabled);
    let campaigns_enabled = flags.iter().any(|f| f.feature == "campaigns" && f.enabled);

    Ok(Json(FeatureFlags {
        crm_enabled,
        campaigns_enabled,
    }))
}

pub async fn extract_lead_from_email(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<LeadExtractionRequest>,
) -> Result<Json<LeadExtractionResponse>, StatusCode> {
    let email = req.from.clone();

    let name_part = email.split('@').next().unwrap_or("");
    let parts: Vec<&str> = name_part.split('.').collect();

    let first_name = parts.first().map(|s| capitalize(s));
    let last_name = if parts.len() > 1 {
        parts.get(1).map(|s| capitalize(s))
    } else {
        None
    };

    let company = email
        .split('@')
        .nth(1)
        .and_then(|d| d.split('.').next())
        .map(capitalize);

    Ok(Json(LeadExtractionResponse {
        first_name,
        last_name,
        email,
        company,
        phone: None,
        value: None,
    }))
}

pub async fn get_crm_context_by_email(
    State(state): State<Arc<AppState>>,
    Path(email): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut conn = state.pool.get().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    #[derive(QueryableByName)]
    struct ContactJsonRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        row_to_json: String,
    }

    let contact: Option<ContactJsonRow> = diesel::sql_query(
        "SELECT row_to_json(c.*) as row_to_json FROM crm_contacts c WHERE c.email = $1"
    )
    .bind::<diesel::sql_types::Text, _>(&email)
    .get_result(&mut conn)
    .ok();

    match contact {
        Some(c) => {
            let contact_value: serde_json::Value = serde_json::from_str(&c.row_to_json).unwrap_or(json!({}));
            Ok(Json(json!({ "found": true, "contact": contact_value })))
        }
        None => Ok(Json(json!({ "found": false }))),
    }
}

pub async fn link_email_to_crm(
    State(state): State<Arc<AppState>>,
    Json(link): Json<EmailCrmLink>,
) -> Result<StatusCode, StatusCode> {
    let mut conn = state.pool.get().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    diesel::sql_query(
        "INSERT INTO email_crm_links (email_id, contact_id, opportunity_id) VALUES ($1, $2, $3)"
    )
    .bind::<diesel::sql_types::Uuid, _>(link.email_id)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(link.contact_id)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(link.opportunity_id)
    .execute(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

pub async fn categorize_email(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<LeadExtractionRequest>,
) -> Result<Json<EmailCategoryResponse>, StatusCode> {
    let category = match call_llm_classify(&req.subject, &req.body).await {
        Ok(c) => c,
        Err(_) => fallback_classify(&req.subject, &req.body),
    };
    let confidence = if category == "general" { 0.6 } else { 0.85 };

    Ok(Json(EmailCategoryResponse { category, confidence }))
}

async fn call_llm_classify(subject: &str, body: &str) -> Result<String, String> {
    let llm_url = match std::env::var("BOT_EMAIL_LLM_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return Err("no LLM URL".into()),
    };

    let prompt = format!(
        "Classifique este email em UMA das categorias: urgent, meeting_request, informational, spam.\n\
         Assunto: {}\nCorpo: {}\nResponda apenas com o nome da categoria, sem pontuacao.",
        subject, body
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build().map_err(|e| e.to_string())?;

    let resp = client.post(&llm_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": std::env::var("BOT_EMAIL_LLM_MODEL").unwrap_or_else(|_| "default".into()),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 20,
        }))
        .send().await.map_err(|e| e.to_string())?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    let text = text.trim().to_lowercase();

    if text.contains("urgent") { Ok("urgent".into()) }
    else if text.contains("meeting") { Ok("meeting_request".into()) }
    else if text.contains("spam") { Ok("spam".into()) }
    else { Ok("informational".into()) }
}

fn fallback_classify(subject: &str, body: &str) -> String {
    let text = format!("{} {}", subject.to_lowercase(), body.to_lowercase());
    if text.contains("meeting") || text.contains("calendar") || text.contains("schedule") || text.contains("invite") {
        "meeting_request".to_string()
    } else if text.contains("urgent") || text.contains("asap") || text.contains("deadline") {
        "urgent".to_string()
    } else if text.contains("unsubscribe") || text.contains("newsletter") || text.contains("marketing") {
        "spam".to_string()
    } else {
        "informational".to_string()
    }
}

pub async fn generate_smart_reply(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SmartReplyRequest>,
) -> Result<Json<SmartReplyResponse>, StatusCode> {
    let suggestions = match call_llm_suggest_reply(req.email_id, req.context.as_deref()).await {
        Ok(s) => s,
        Err(_) => vec![
            "Thank you for your email. I'll get back to you shortly.".to_string(),
            "I appreciate you reaching out. Let me review this and respond soon.".to_string(),
            "Thanks for the update. I'll take a look and follow up.".to_string(),
        ],
    };
    Ok(Json(SmartReplyResponse { suggestions }))
}

async fn call_llm_suggest_reply(_email_id: Uuid, context: Option<&str>) -> Result<Vec<String>, String> {
    let llm_url = match std::env::var("BOT_EMAIL_LLM_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return Err("no LLM URL".into()),
    };

    let ctx = context.unwrap_or("responda de forma profissional e cordial");
    let prompt = format!(
        "Gere 3 sugestoes de resposta curtas para um email. Contexto: {}\n\
         Responda apenas com as 3 sugestoes, uma por linha, sem numeracao.",
        ctx
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build().map_err(|e| e.to_string())?;

    let resp = client.post(&llm_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": std::env::var("BOT_EMAIL_LLM_MODEL").unwrap_or_else(|_| "default".into()),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 150,
        }))
        .send().await.map_err(|e| e.to_string())?;

    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok(text.lines().filter(|l| !l.is_empty()).map(|l| l.trim().to_string()).collect())
}

pub async fn handle_refine_draft(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<RefineDraftRequest>,
) -> Result<Json<RefineDraftResponse>, StatusCode> {
    let refined = match call_llm_refine(&req.draft, &req.instruction).await {
        Ok(r) => r,
        Err(_) => format!("{}\n\n[AI refinement unavailable: {}]", req.draft, req.instruction),
    };
    Ok(Json(RefineDraftResponse { draft: refined }))
}

#[derive(Debug, Deserialize)]
pub struct ConflictCheckResponse {
    pub has_conflicts: bool,
    pub conflicts: Vec<serde_json::Value>,
}

pub async fn handle_resolve_meeting(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<MeetingResolveRequest>,
) -> Result<Json<MeetingResolveResponse>, StatusCode> {
    let conflicts = check_calendar_conflicts_internal(&req).await.unwrap_or_default();
    let has_conflicts = !conflicts.is_empty();

    let suggested_alternatives = if has_conflicts {
        generate_alternative_times(&req.proposed_start, &req.proposed_end)
    } else {
        Vec::new()
    };

    let reply_draft = match &req.body {
        Some(body) => generate_meeting_reply_draft(&req, has_conflicts, &suggested_alternatives, body),
        None => generate_meeting_reply_draft(&req, has_conflicts, &suggested_alternatives, ""),
    };

    Ok(Json(MeetingResolveResponse {
        has_conflicts,
        conflicts: conflicts.into_iter().map(|c| c.to_string()).collect(),
        suggested_alternatives,
        reply_draft,
    }))
}

async fn check_calendar_conflicts_internal(req: &MeetingResolveRequest) -> Result<Vec<String>, String> {
    let calendar_api = std::env::var("BOT_CALENDAR_API_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build().map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("{}/api/calendar/conflicts", calendar_api))
        .json(&serde_json::json!({
            "calendar_id": req.calendar_id.as_deref().unwrap_or("00000000-0000-0000-0000-000000000000"),
            "start_time": req.proposed_start,
            "end_time": req.proposed_end,
        }))
        .send().await.map_err(|e| e.to_string())?;

    let result: ConflictCheckResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(result.conflicts.into_iter().map(|c| c.to_string()).collect())
}

fn generate_alternative_times(proposed_start: &str, proposed_end: &str) -> Vec<String> {
    let start = chrono::DateTime::parse_from_rfc3339(proposed_start).ok();
    let end = chrono::DateTime::parse_from_rfc3339(proposed_end).ok();
    let (start, end) = match (start, end) {
        (Some(s), Some(e)) => (s, e),
        _ => return vec!["+1 hour".to_string(), "+1 day".to_string(), "-1 hour".to_string()],
    };
    let _duration = end - start;
    vec![
        (start + chrono::Duration::hours(1)).to_rfc3339(),
        (start + chrono::Duration::days(1)).to_rfc3339(),
        (start - chrono::Duration::hours(1)).to_rfc3339(),
    ]
}

fn generate_meeting_reply_draft(
    req: &MeetingResolveRequest,
    has_conflicts: bool,
    alternatives: &[String],
    _body: &str,
) -> String {
    if has_conflicts {
        let alts = if alternatives.is_empty() {
            String::new()
        } else {
            format!("\n\nSuggested alternative times:\n{}",
                alternatives.iter().enumerate()
                    .map(|(i, a)| format!("{}. {}", i + 1, a))
                    .collect::<Vec<_>>()
                    .join("\n"))
        };
        format!(
            "Subject: Re: {}\n\nHello,\n\nThank you for the meeting invitation. \
             Unfortunately, I have a scheduling conflict at that time.{} \
             \n\nPlease let me know if one of these alternatives works for you.\n\nBest regards",
            req.subject, alts
        )
    } else {
        format!(
            "Subject: Re: {}\n\nHello,\n\nThank you for the invitation. \
             I confirm my availability and look forward to the meeting.\n\nBest regards",
            req.subject
        )
    }
}

async fn call_llm_refine(draft: &str, instruction: &str) -> Result<String, String> {
    let llm_url = match std::env::var("BOT_EMAIL_LLM_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return Err("no LLM URL".into()),
    };

    let prompt = format!(
        "Refine o seguinte rascunho de email conforme a instrucao.\n\
         Instrucao: {}\nRascunho:\n{}\n\nResponda apenas com o rascunho refinado, sem explicacoes.",
        instruction, draft
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build().map_err(|e| e.to_string())?;

    let resp = client.post(&llm_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": std::env::var("BOT_EMAIL_LLM_MODEL").unwrap_or_else(|_| "default".into()),
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 500,
        }))
        .send().await.map_err(|e| e.to_string())?;

    resp.text().await.map_err(|e| e.to_string())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("john"), "John");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("a"), "A");
    }
}
