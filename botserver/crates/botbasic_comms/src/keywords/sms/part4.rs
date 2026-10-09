use super::*;

pub(super) async fn send_via_twilio(
    state: &dyn BasicRuntime,
    bot_id: &Uuid,
    phone: &str,
    message: &str,
    priority: &SmsPriority,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let config_manager = ConfigManager::new(state.db_pool().clone());

    let account_sid = config_manager
        .get_config(bot_id, "twilio-account-sid", None)
        .map_err(|_| "Twilio account SID not configured. Set twilio-account-sid in config.")?;

    let auth_token = config_manager
        .get_config(bot_id, "twilio-auth-token", None)
        .map_err(|_| "Twilio auth token not configured. Set twilio-auth-token in config.")?;

    let from_number = config_manager
        .get_config(bot_id, "twilio-from-number", None)
        .map_err(|_| "Twilio from number not configured. Set twilio-from-number in config.")?;

    let client = reqwest::Client::new();
    let url = format!(
        "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
        account_sid
    );

    let final_message = match priority {
        SmsPriority::Urgent => format!("[URGENT] {}", message),
        SmsPriority::High => format!("[HIGH] {}", message),
        _ => message.to_string(),
    };

    let params = [
        ("To", phone),
        ("From", from_number.as_str()),
        ("Body", final_message.as_str()),
    ];

    let response = client
        .post(&url)
        .basic_auth(&account_sid, Some(&auth_token))
        .form(&params)
        .send()
        .await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        let sid = json["sid"].as_str().map(|s| s.to_string());
        Ok(sid)
    } else {
        let error_text = response.text().await?;
        Err(format!("Twilio API error: {}", error_text).into())
    }
}
pub(super) async fn send_via_aws_sns(
    state: &dyn BasicRuntime,
    bot_id: &Uuid,
    phone: &str,
    message: &str,
    priority: &SmsPriority,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let config_manager = ConfigManager::new(state.db_pool().clone());

    let access_key = config_manager
        .get_config(bot_id, "aws-access-key", None)
        .map_err(|_| "AWS access key not configured. Set aws-access-key in config.")?;

    let secret_key = config_manager
        .get_config(bot_id, "aws-secret-key", None)
        .map_err(|_| "AWS secret key not configured. Set aws-secret-key in config.")?;

    let region = config_manager
        .get_config(bot_id, "aws-region", Some("us-east-1"))
        .unwrap_or_else(|_| "us-east-1".to_string());

    let client = reqwest::Client::new();
    let url = format!("https://sns.{}.amazonaws.com/", region);

    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    let sms_type = match priority {
        SmsPriority::High | SmsPriority::Urgent => "Transactional",
        _ => "Promotional",
    };

    let params = [
        ("Action", "Publish"),
        ("PhoneNumber", phone),
        ("Message", message),
        ("Version", "2010-03-31"),
        ("MessageAttributes.entry.1.Name", "AWS.SNS.SMS.SMSType"),
        ("MessageAttributes.entry.1.Value.DataType", "String"),
        ("MessageAttributes.entry.1.Value.StringValue", sms_type),
    ];

    let response = client
        .post(&url)
        .form(&params)
        .header("X-Amz-Date", &timestamp)
        .basic_auth(&access_key, Some(&secret_key))
        .send()
        .await?;

    if response.status().is_success() {
        let body = response.text().await?;

        if let Some(start) = body.find("<MessageId>") {
            if let Some(end) = body.find("</MessageId>") {
                let message_id = &body[start + 11..end];
                return Ok(Some(message_id.to_string()));
            }
        }
        Ok(None)
    } else {
        let error_text = response.text().await?;
        Err(format!("AWS SNS API error: {}", error_text).into())
    }
}
pub(super) async fn send_via_vonage(
    state: &dyn BasicRuntime,
    bot_id: &Uuid,
    phone: &str,
    message: &str,
    priority: &SmsPriority,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let config_manager = ConfigManager::new(state.db_pool().clone());

    let api_key = config_manager
        .get_config(bot_id, "vonage-api-key", None)
        .map_err(|_| "Vonage API key not configured. Set vonage-api-key in config.")?;

    let api_secret = config_manager
        .get_config(bot_id, "vonage-api-secret", None)
        .map_err(|_| "Vonage API secret not configured. Set vonage-api-secret in config.")?;

    let from_number = config_manager
        .get_config(bot_id, "vonage-from-number", None)
        .map_err(|_| "Vonage from number not configured. Set vonage-from-number in config.")?;

    let client = reqwest::Client::new();

    let message_class = match priority {
        SmsPriority::Urgent => Some("0"),
        _ => None,
    };

    let mut body = serde_json::json!({
        "api_key": api_key,
        "api_secret": api_secret,
        "to": phone,
        "from": from_number,
        "text": message
    });

    if let Some(class) = message_class {
        body["message-class"] = serde_json::Value::String(class.to_string());
    }

    let response = client
        .post("https://rest.nexmo.com/sms/json")
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Vonage API error: {error_text}").into());
    }

    let json: serde_json::Value = response.json().await?;
    let messages = json["messages"].as_array();

    if let Some(msgs) = messages {
        if let Some(first) = msgs.first() {
            if first["status"].as_str() == Some("0") {
                return Ok(first["message-id"].as_str().map(|s| s.to_string()));
            }
            let error_text = first["error-text"].as_str().unwrap_or("Unknown error");
            return Err(format!("Vonage error: {error_text}").into());
        }
    }
    Err("Invalid Vonage response".into())
}
pub(super) async fn send_via_messagebird(
    state: &dyn BasicRuntime,
    bot_id: &Uuid,
    phone: &str,
    message: &str,
    priority: &SmsPriority,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let config_manager = ConfigManager::new(state.db_pool().clone());

    let api_key = config_manager
        .get_config(bot_id, "messagebird-api-key", None)
        .map_err(|_| "MessageBird API key not configured. Set messagebird-api-key in config.")?;

    let originator = config_manager
        .get_config(bot_id, "messagebird-originator", None)
        .map_err(|_| {
            "MessageBird originator not configured. Set messagebird-originator in config."
        })?;

    let client = reqwest::Client::new();

    let type_details = match priority {
        SmsPriority::Urgent => Some(serde_json::json!({"class": 0})),
        SmsPriority::High => Some(serde_json::json!({"class": 1})),
        _ => None,
    };

    let mut body = serde_json::json!({
        "originator": originator,
        "recipients": [phone],
        "body": message
    });

    if let Some(details) = type_details {
        body["typeDetails"] = details;
    }

    let response = client
        .post("https://rest.messagebird.com/messages")
        .header("Authorization", format!("AccessKey {}", api_key))
        .json(&body)
        .send()
        .await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        Ok(json["id"].as_str().map(|s| s.to_string()))
    } else {
        let error_text = response.text().await?;
        Err(format!("MessageBird API error: {}", error_text).into())
    }
}
pub(super) async fn send_via_custom_webhook(
    state: &dyn BasicRuntime,
    bot_id: &Uuid,
    webhook_name: &str,
    phone: &str,
    message: &str,
    priority: &SmsPriority,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    let config_manager = ConfigManager::new(state.db_pool().clone());

    let webhook_url = config_manager
        .get_config(bot_id, &format!("{}-webhook-url", webhook_name), None)
        .map_err(|_| {
            format!(
                "Custom SMS webhook URL not configured. Set {}-webhook-url in config.",
                webhook_name
            )
        })?;

    let api_key = config_manager
        .get_config(bot_id, &format!("{}-api-key", webhook_name), None)
        .ok();

    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "to": phone,
        "message": message,
        "provider": webhook_name,
        "priority": priority.to_string()
    });

    let mut request = client.post(&webhook_url).json(&payload);

    if let Some(key) = api_key {
        request = request.header("Authorization", format!("Bearer {}", key));
    }

    let response = request.send().await?;

    if response.status().is_success() {
        let json: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        Ok(json["message_id"]
            .as_str()
            .or_else(|| json["id"].as_str())
            .map(|s| s.to_string()))
    } else {
        let error_text = response.text().await?;
        Err(format!("Custom webhook error: {}", error_text).into())
    }
}
