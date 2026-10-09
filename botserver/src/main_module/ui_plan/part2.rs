use super::*;

/// Strips HTML/control characters and truncates a raw LLM value.
pub fn sanitize_value(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | '\0'))
        .collect();
    cleaned.chars().take(MAX_VALUE_LEN).collect()
}
pub(super) fn valid_app_id(app: &str) -> bool {
    let trimmed = app.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.len() > 64 {
        return false;
    }
    trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
}
/// Validates a decoded plan. Returns the sanitized plan on success.
pub fn validate_ui_plan(raw: &UiPlan) -> Result<UiPlan, String> {
    if raw.steps.is_empty() {
        return Err("UI plan has no steps".to_string());
    }
    if raw.steps.len() > MAX_UI_STEPS {
        return Err(format!(
            "UI plan exceeds step limit of {MAX_UI_STEPS}"
        ));
    }

    if let Some(app) = &raw.app {
        if !valid_app_id(app) {
            return Err(format!("Invalid app id '{app}'"));
        }
    }

    let mut sanitized_steps = Vec::with_capacity(raw.steps.len());
    for (idx, step) in raw.steps.iter().enumerate() {
        let sanitized = sanitize_step(step)?;
        if step.op == UiOp::Open {
            let app = step
                .app
                .as_deref()
                .or(raw.app.as_deref())
                .ok_or_else(|| "open step requires an app".to_string())?;
            if !valid_app_id(app) {
                return Err(format!("Invalid app id '{app}' at step {idx}"));
            }
        }
        sanitized_steps.push(sanitized);
    }

    Ok(UiPlan {
        app: raw.app.as_deref().map(sanitize_value),
        steps: sanitized_steps,
    })
}
pub(super) fn sanitize_step(step: &UiStep) -> Result<UiStep, String> {
    let mut out = UiStep {
        op: step.op.clone(),
        app: None,
        label: None,
        field: None,
        value: None,
        ms: None,
        cell: None,
    };
    if let Some(app) = &step.app {
        out.app = Some(sanitize_value(app));
    }
    if let Some(label) = &step.label {
        out.label = Some(sanitize_value(label));
    }
    if let Some(field) = &step.field {
        out.field = Some(sanitize_value(field));
    }
    if let Some(value) = &step.value {
        out.value = Some(sanitize_value(value));
    }
    if let Some(ms) = step.ms {
        out.ms = Some(ms.min(30_000));
    }
    if let Some(cell) = &step.cell {
        let upper = cell.trim().to_uppercase();
        if !is_cell_ref(&upper) {
            return Err(format!("Invalid cell reference '{cell}' at step"));
        }
        out.cell = Some(upper);
    }
    if step.op == UiOp::Cell && out.cell.is_none() {
        return Err("cell step requires a cell reference".to_string());
    }
    if step.op == UiOp::Cell && out.value.is_none() {
        return Err("cell step requires a value".to_string());
    }
    Ok(out)
}
/// Validates a spreadsheet cell reference like "A1" or "B5".
pub(super) fn is_cell_ref(reference: &str) -> bool {
    let bytes = reference.as_bytes();
    if bytes.is_empty() || bytes.len() > 5 {
        return false;
    }
    let letters: Vec<u8> = bytes.iter().copied().take_while(u8::is_ascii_uppercase).collect();
    if letters.is_empty() || letters.len() > 3 {
        return false;
    }
    let rest = &bytes[letters.len()..];
    if rest.is_empty() || rest.len() > 3 {
        return false;
    }
    rest.iter().all(u8::is_ascii_digit)
}
/// Attempts to decode a raw `__ui_plan__` payload into a validated plan.
pub fn parse_ui_plan(payload: &serde_json::Value) -> Result<UiPlan, String> {
    let raw: UiPlan = serde_json::from_value(payload.clone())
        .map_err(|e| format!("UI plan parse error: {e}"))?;
    validate_ui_plan(&raw)
}
/// Extracts and validates the first `{"__ui_plan__": {...}}` object from a
/// raw LLM response string. Returns `None` when no plan is present.
pub fn extract_and_validate_plan(raw_response: &str) -> Option<Result<UiPlan, String>> {
    let start = raw_response.find(UI_PLAN_TRIGGER)?;
    let obj_start = raw_response[..start].rfind('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut end = obj_start;
    for (idx, c) in raw_response[obj_start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = obj_start + idx + c.len_utf8();
                    break;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    let candidate = raw_response[obj_start..end].trim();
    let payload: serde_json::Value = match serde_json::from_str(candidate) {
        Ok(v) => v,
        Err(e) => return Some(Err(format!("UI plan JSON error: {e}"))),
    };
    let plan_value = payload.get("__ui_plan__").cloned().unwrap_or(payload);
    Some(parse_ui_plan(&plan_value))
}
/// Removes the `{"__ui_plan__": {...}}` JSON block from a response so the
/// plan contract never reaches the user-visible chat content. Handles the
/// trigger spanning stream chunk boundaries.
pub fn strip_plan_json(raw_response: &str) -> String {
    let Some(start) = raw_response.find(UI_PLAN_TRIGGER) else {
        return raw_response.to_string();
    };
    let Some(obj_start) = raw_response[..start].rfind('{') else {
        return raw_response.to_string();
    };
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut end = obj_start;
    for (idx, c) in raw_response[obj_start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = obj_start + idx + c.len_utf8();
                    break;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return raw_response.to_string();
    }
    let mut stripped = String::with_capacity(raw_response.len());
    stripped.push_str(raw_response[..obj_start].trim_start_matches(|c: char| c.is_whitespace()));
    stripped.push_str(raw_response[end..].trim_start_matches(|c: char| c == '\n'));
    stripped.trim().to_string()
}
