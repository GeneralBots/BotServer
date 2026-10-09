use super::*;

pub(super) fn create_message_template(name: &str, channel: &str, subject: Option<&str>, content: &str) -> Map {
    let mut result = Map::new();

    if name.is_empty() {
        result.insert("success".into(), Dynamic::from(false));
        result.insert(
            "error".into(),
            Dynamic::from("Template name cannot be empty"),
        );
        return result;
    }

    if content.is_empty() {
        result.insert("success".into(), Dynamic::from(false));
        result.insert(
            "error".into(),
            Dynamic::from("Template content cannot be empty"),
        );
        return result;
    }

    debug!(
        "Saving template '{}' to database for channel '{}'",
        name, channel
    );

    info!("Creating template '{}' for channel '{}'", name, channel);

    result.insert("success".into(), Dynamic::from(true));
    result.insert("name".into(), Dynamic::from(name.to_string()));
    result.insert("channel".into(), Dynamic::from(channel.to_string()));

    if let Some(subj) = subject {
        result.insert("subject".into(), Dynamic::from(subj.to_string()));
    }

    let variables = extract_template_variables(content);
    result.insert("variables".into(), Dynamic::from(variables));

    result
}

pub(super) fn get_message_template(name: &str, channel: Option<&str>) -> Map {
    let mut result = Map::new();

    debug!("Loading template '{}' from database", name);

    result.insert("name".into(), Dynamic::from(name.to_string()));
    result.insert("found".into(), Dynamic::from(false));
    debug!("Template '{}' not found in database", name);

    if let Some(ch) = channel {
        result.insert("channel".into(), Dynamic::from(ch.to_string()));
    }

    result.insert(
        "content".into(),
        Dynamic::from(format!("Template '{}' content placeholder", name)),
    );

    result
}

pub(super) fn extract_template_variables(content: &str) -> Array {
    let mut variables = Array::new();
    let mut in_variable = false;
    let mut current_var = String::new();

    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();

    for i in 0..len {
        if i + 1 < len && chars[i] == '{' && chars[i + 1] == '{' {
            in_variable = true;
            current_var.clear();
        } else if i + 1 < len && chars[i] == '}' && chars[i + 1] == '}' {
            if in_variable && !current_var.is_empty() {
                let var_name = current_var.trim().to_string();
                if !var_name.is_empty() {
                    variables.push(Dynamic::from(var_name));
                }
            }
            in_variable = false;
            current_var.clear();
        } else if in_variable && chars[i] != '{' {
            current_var.push(chars[i]);
        }
    }

    variables
}

pub(super) fn generate_message_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    format!("msg_{}", timestamp)
}
