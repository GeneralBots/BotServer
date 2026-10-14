
#[test]
pub(super) fn test_webhook_request_to_dynamic() {
    let mut headers = std::collections::HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let mut params = std::collections::HashMap::new();
    params.insert("id".to_string(), "123".to_string());

    let request = WebhookRequest::new(
        "POST",
        headers,
        params,
        json!({"order": "test"}),
        "/webhook/order-received",
    );

    let dynamic = request.to_dynamic();
    assert!(dynamic.is_map());
}

#[test]
pub(super) fn test_webhook_response_from_dynamic() {
    let mut map = rhai::Map::new();
    map.insert("status".into(), Dynamic::from(201_i64));
    map.insert(
        "body".into(),
        Dynamic::from(json!({"message": "created"}).to_string()),
    );

    let dynamic = Dynamic::from(map);
    let response = WebhookResponse::from_dynamic(&dynamic);

    assert_eq!(response.status, 201);
}

#[test]
pub(super) fn test_json_to_dynamic_and_back() {
    let original = json!({
        "name": "test",
        "count": 42,
        "active": true,
        "items": [1, 2, 3]
    });

    let dynamic = json_to_dynamic(&original);
    let back = dynamic_to_json(&dynamic);

    assert_eq!(original["name"], back["name"]);
    assert_eq!(original["count"], back["count"]);
    assert_eq!(original["active"], back["active"]);
}

#[test]
pub(super) fn test_webhook_response_default() {
    let response = WebhookResponse::default();
    assert_eq!(response.status, 200);
}

#[test]
pub(super) fn test_webhook_response_error() {
    let response = WebhookResponse::error(404, "Not found");
    assert_eq!(response.status, 404);
    assert_eq!(response.body["error"], "Not found");
}
