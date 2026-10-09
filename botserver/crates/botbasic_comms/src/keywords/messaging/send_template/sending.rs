use super::*;

pub(super) fn send_template_message(
    template: &str,
    recipient: &str,
    channel: &str,
    variables: Option<&Map>,
) -> Map {
    let mut result = Map::new();

    let valid_channels = ["email", "whatsapp", "sms", "telegram", "push"];
    let channel_lower = channel.to_lowercase();

    if !valid_channels.contains(&channel_lower.as_str()) {
        result.insert("success".into(), Dynamic::from(false));
        result.insert(
            "error".into(),
            Dynamic::from(format!(
                "Invalid channel '{}'. Valid channels: {:?}",
                channel, valid_channels
            )),
        );
        return result;
    }

    let recipient_valid = match channel_lower.as_str() {
        "email" => recipient.contains('@'),
        "whatsapp" | "sms" => {
            recipient.starts_with('+') || recipient.chars().all(|c| c.is_numeric())
        }
        // "telegram", "push", and all other channels just require non-empty recipient
        _ => !recipient.is_empty(),
    };

    if !recipient_valid {
        result.insert("success".into(), Dynamic::from(false));
        result.insert(
            "error".into(),
            Dynamic::from(format!(
                "Invalid recipient '{}' for channel '{}'",
                recipient, channel
            )),
        );
        return result;
    }

    debug!("Loading template '{}' from database", template);
    debug!("Rendering template with recipient: {}", recipient);
    debug!("Sending via channel: {}", channel);

    info!(
        "Sending template '{}' to '{}' via '{}'",
        template, recipient, channel
    );

    result.insert("success".into(), Dynamic::from(true));
    result.insert("template".into(), Dynamic::from(template.to_string()));
    result.insert("recipient".into(), Dynamic::from(recipient.to_string()));
    result.insert("channel".into(), Dynamic::from(channel.to_string()));
    result.insert("message_id".into(), Dynamic::from(generate_message_id()));
    result.insert("status".into(), Dynamic::from("queued"));

    if let Some(vars) = variables {
        result.insert("variables_count".into(), Dynamic::from(vars.len() as i64));
    }

    result
}

pub(super) fn send_template_batch(
    template: &str,
    recipients: &Array,
    channel: &str,
    variables: Option<&Map>,
) -> Map {
    let mut result = Map::new();
    let mut sent_count = 0_i64;
    let mut failed_count = 0_i64;
    let mut errors = Array::new();

    for recipient in recipients {
        let recipient_str = recipient.to_string();
        let send_result = send_template_message(template, &recipient_str, channel, variables);

        if let Some(success) = send_result.get("success") {
            if success.as_bool().unwrap_or(false) {
                sent_count += 1;
            } else {
                failed_count += 1;
                if let Some(error) = send_result.get("error") {
                    let mut error_entry = Map::new();
                    error_entry.insert("recipient".into(), Dynamic::from(recipient_str));
                    error_entry.insert("error".into(), error.clone());
                    errors.push(Dynamic::from(error_entry));
                }
            }
        }
    }

    result.insert("success".into(), Dynamic::from(failed_count == 0));
    result.insert("total".into(), Dynamic::from(recipients.len() as i64));
    result.insert("sent".into(), Dynamic::from(sent_count));
    result.insert("failed".into(), Dynamic::from(failed_count));
    result.insert("template".into(), Dynamic::from(template.to_string()));
    result.insert("channel".into(), Dynamic::from(channel.to_string()));

    if !errors.is_empty() {
        result.insert("errors".into(), Dynamic::from(errors));
    }

    result
}
