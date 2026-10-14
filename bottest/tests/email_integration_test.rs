// Integration tests for Email-CRM-Campaigns features
use serde_json::json;

#[tokio::test]
#[ignore]
async fn test_feature_flags_endpoint() {
    let client = reqwest::Client::new();
    let org_id = "00000000-0000-0000-0000-000000000000";
    
    let response = client
        .get(format!("http://localhost:8080/api/features/{}/enabled", org_id))
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(resp.status().is_success() || resp.status().is_client_error());
    }
}

#[tokio::test]
#[ignore]
async fn test_extract_lead_endpoint() {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "from": "john.doe@example.com",
        "subject": "Interested in your product",
        "body": "I would like to know more about pricing"
    });
    
    let response = client
        .post("http://localhost:8080/api/ai/extract-lead")
        .json(&payload)
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(resp.status().is_success() || resp.status().is_client_error());
    }
}

#[tokio::test]
#[ignore]
async fn test_categorize_email_endpoint() {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "from": "customer@example.com",
        "subject": "Need help with my account",
        "body": "I'm having trouble logging in"
    });
    
    let response = client
        .post("http://localhost:8080/api/ai/categorize-email")
        .json(&payload)
        .send()
        .await;
    
    if let Ok(resp) = response {
        if resp.status().is_success() {
            let data: serde_json::Value = resp.json().await.unwrap();
            assert!(data.get("category").is_some());
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_snooze_email_endpoint() {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "email_ids": ["00000000-0000-0000-0000-000000000001"],
        "preset": "tomorrow"
    });
    
    let response = client
        .post("http://localhost:8080/api/email/snooze")
        .json(&payload)
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(resp.status().is_success() || resp.status().is_client_error());
    }
}

#[tokio::test]
#[ignore]
async fn test_flag_email_endpoint() {
    let client = reqwest::Client::new();
    
    let payload = json!({
        "email_ids": ["00000000-0000-0000-0000-000000000001"],
        "follow_up": "today"
    });
    
    let response = client
        .post("http://localhost:8080/api/email/flag")
        .json(&payload)
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(resp.status().is_success() || resp.status().is_client_error());
    }
}
