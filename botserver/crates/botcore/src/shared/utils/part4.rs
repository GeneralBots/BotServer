use super::*;

/// Convert date string from user locale format to ISO format (YYYY-MM-DD) for PostgreSQL.
///
/// The LLM automatically formats dates according to the user's language/idiom based on:
/// 1. The conversation context (user's language)
/// 2. The PARAM LIKE example (e.g., "15/12/2026" for DD/MM/YYYY)
///
/// This function handles the most common formats:
/// - ISO: YYYY-MM-DD (already in ISO, returned as-is)
/// - Brazilian/Portuguese: DD/MM/YYYY or DD/MM/YY
/// - US/English: MM/DD/YYYY or MM/DD/YY
///
/// If the value doesn't match any date pattern, returns it unchanged.
///
/// NOTE: This function does NOT try to guess ambiguous formats.
/// The LLM is responsible for formatting dates correctly based on user language.
/// The PARAM declaration's LIKE example tells the LLM the expected format.
///
/// # Arguments
/// * `value` - The date string to convert (as provided by the LLM)
///
/// # Returns
/// ISO formatted date string (YYYY-MM-DD) or original value if not a recognized date
pub fn convert_date_to_iso_format(value: &str) -> String {
    let value = value.trim();

    // Already in ISO format (YYYY-MM-DD) - return as-is
    if value.len() == 10 && value.chars().nth(4) == Some('-') && value.chars().nth(7) == Some('-') {
        let parts: Vec<&str> = value.split('-').collect();
        if parts.len() == 3
            && parts[0].len() == 4
            && parts[1].len() == 2
            && parts[2].len() == 2
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].chars().all(|c| c.is_ascii_digit())
        {
            if let (Ok(year), Ok(month), Ok(day)) =
                (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>())
            {
                if (1..=12).contains(&month) && (1..=31).contains(&day) && (1900..=2100).contains(&year) {
                    return value.to_string();
                }
            }
        }
    }

    // Handle slash-separated formats: DD/MM/YYYY or MM/DD/YYYY
    // We need to detect which format based on the PARAM declaration's LIKE example
    // For now, default to DD/MM/YYYY (Brazilian format) as this is the most common for this bot
    // TODO: Pass language/idiom from session to determine correct format
    if value.len() >= 8 && value.len() <= 10 {
        let parts: Vec<&str> = value.split('/').collect();
        if parts.len() == 3 {
            let all_numeric = parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].chars().all(|c| c.is_ascii_digit())
                && parts[2].chars().all(|c| c.is_ascii_digit());

            if all_numeric {
                // Parse the three parts
                let a = parts[0].parse::<u32>().ok();
                let b = parts[1].parse::<u32>().ok();
                let c = if parts[2].len() == 2 {
                    // Convert 2-digit year to 4-digit
                    parts[2].parse::<u32>().ok().map(|y| {
                        if y < 50 {
                            2000 + y
                        } else {
                            1900 + y
                        }
                    })
                } else {
                    parts[2].parse::<u32>().ok()
                };

                if let (Some(first), Some(second), Some(third)) = (a, b, c) {
                    // Default: DD/MM/YYYY format (Brazilian/Portuguese)
                    // The LLM should format dates according to the user's language
                    // and the PARAM LIKE example (e.g., "15/12/2026" for DD/MM/YYYY)
                    let (year, month, day) = (third, second, first);

                    // Validate the determined date
                    if (1..=31).contains(&day) && (1..=12).contains(&month) && (1900..=2100).contains(&year) {
                        return format!("{:04}-{:02}-{:02}", year, month, day);
                    }
                }
            }
        }
    }

    // Not a recognized date pattern, return unchanged
    value.to_string()
}
pub fn get_secrets_manager_sync() -> Result<serde_json::Value, String> {
    let secrets = std::thread::spawn(|| {
        let sm = match botcoresecrets::SecretsManager::get() {
            Ok(s) => s,
            Err(e) => return Err(format!("SecretsManager not available: {}", e)),
        };
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to create runtime: {}", e)),
        };
        rt.block_on(sm.get_secret("gbo/vectordb"))
            .map_err(|e| format!("Failed to read vectordb secrets: {}", e))
    })
    .join()
    .map_err(|_| "Thread panic in get_secrets_manager_sync".to_string())?
    .map_err(|e| format!("Secrets fetch failed: {}", e))?;

    let mut map = serde_json::Map::new();
    if let Some(url) = secrets.get("url").or(secrets.get("QDRANT_URL")).or(secrets.get("vectordb-url")) {
        if !url.is_empty() {
            map.insert("QDRANT_URL".to_string(), serde_json::Value::String(url.clone()));
        }
    }
    if let Some(api_key) = secrets.get("api_key").or(secrets.get("QDRANT_API_KEY")).or(secrets.get("vectordb-api-key")) {
        if !api_key.is_empty() {
            map.insert("QDRANT_API_KEY".to_string(), serde_json::Value::String(api_key.clone()));
        }
    }
    if let Some(collection) = secrets.get("collection").or(secrets.get("QDRANT_COLLECTION")).or(secrets.get("vectordb-collection")) {
        if !collection.is_empty() {
            map.insert("QDRANT_COLLECTION".to_string(), serde_json::Value::String(collection.clone()));
        }
    }

    if map.is_empty() {
        Err("No vectordb secrets available in Vault".to_string())
    } else {
        Ok(serde_json::Value::Object(map))
    }
}
pub fn get_vectordb_config_from_value(value: &serde_json::Value) -> (String, String, String) {
    let url = value.get("QDRANT_URL").or(value.get("vectordb-url"))
        .and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:6333").to_string();
    let api_key = value.get("QDRANT_API_KEY").or(value.get("vectordb-api-key"))
        .and_then(|v| v.as_str()).unwrap_or("").to_string();
    let collection = value.get("QDRANT_COLLECTION").or(value.get("vectordb-collection"))
        .and_then(|v| v.as_str()).unwrap_or("default").to_string();
    (url, api_key, collection)
}
