use bottest::prelude::*;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct StatusRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

fn test_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

async fn seed_org_branch(ctx: &TestContext, name: &str) -> Option<Uuid> {
    let pool = ctx.db_pool().await.ok()?;
    let mut conn = pool.get().ok()?;
    let tenant_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let branch_id = Uuid::new_v4();
    let slug = format!("{}-{}", name, Uuid::new_v4().simple());

    use diesel::RunQueryDsl;
    let inserted = diesel::sql_query(
        "INSERT INTO tenants (id, name, slug, created_at, updated_at) \
         VALUES ($1, $2, $3, NOW(), NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(tenant_id)
    .bind::<diesel::sql_types::Text, _>(name)
    .bind::<diesel::sql_types::Text, _>(&slug)
    .execute(&mut conn)
    .and_then(|_| {
        diesel::sql_query(
            "INSERT INTO organizations (org_id, tenant_id, name, slug, created_at) \
             VALUES ($1, $2, $3, $4, NOW())",
        )
        .bind::<diesel::sql_types::Uuid, _>(org_id)
        .bind::<diesel::sql_types::Uuid, _>(tenant_id)
        .bind::<diesel::sql_types::Text, _>(name)
        .bind::<diesel::sql_types::Text, _>(&slug)
        .execute(&mut conn)
    })
    .and_then(|_| {
        diesel::sql_query(
            "INSERT INTO branches (id, org_id, tenant_id, slug, name, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .bind::<diesel::sql_types::Uuid, _>(org_id)
        .bind::<diesel::sql_types::Uuid, _>(tenant_id)
        .bind::<diesel::sql_types::Text, _>(&slug)
        .bind::<diesel::sql_types::Text, _>(name)
        .execute(&mut conn)
    });

    match inserted {
        Ok(_) => Some(branch_id),
        Err(e) => {
            eprintln!("Skipping: seed_org_branch failed ({e})");
            None
        }
    }
}

#[tokio::test]
async fn test_cloud_signup_login_roundtrip_with_zitadel() {
    let ctx = match TestHarness::quick().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {e}");
            return;
        }
    };

    let zitadel_user_id = Uuid::new_v4().to_string();
    if let Some(mock) = ctx.mock_zitadel() {
        mock.mount_cloud_flow(&zitadel_user_id).await;
    }

    let server = match ctx.start_botserver().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping: {e}");
            return;
        }
    };
    if !server.is_running() {
        eprintln!("Skipping: botserver did not start");
        return;
    }

    let client = test_client();
    let base_url = server.url.clone();
    let email = format!("roundtrip-{}@example.com", Uuid::new_v4().simple());
    let password = "Sup3rS3cret!";

    let signup = client
        .post(format!("{base_url}/api/cloud/auth/signup"))
        .json(&json!({
            "name": "Roundtrip User",
            "bot_name": format!("bot-{}", Uuid::new_v4().simple()),
            "email": email,
            "password": password,
            "plan": "shared",
        }))
        .send()
        .await
        .expect("signup request failed");

    assert_eq!(
        signup.status(),
        reqwest::StatusCode::OK,
        "signup should succeed"
    );
    let signup_body: serde_json::Value = signup.json().await.expect("signup JSON");
    assert_eq!(signup_body["status"], "ok");
    assert_eq!(signup_body["plan"], "shared");
    assert!(!signup_body["token"].as_str().unwrap_or("").is_empty());
    assert!(signup_body["org_id"].is_string());
    assert!(signup_body["branch_id"].is_string());
    let signup_token = signup_body["token"].as_str().unwrap().to_string();

    let login = client
        .post(format!("{base_url}/api/cloud/auth/login"))
        .json(&json!({
            "email": email,
            "password": password,
        }))
        .send()
        .await
        .expect("login request failed");
    assert_eq!(
        login.status(),
        reqwest::StatusCode::OK,
        "login should succeed after signup"
    );
    let login_body: serde_json::Value = login.json().await.expect("login JSON");
    let login_token = login_body["token"].as_str().unwrap_or("").to_string();
    assert!(!login_token.is_empty(), "login must issue a JWT");

    for token in [&signup_token, &login_token] {
        let plans = client
            .get(format!("{base_url}/api/cloud/plans"))
            .bearer_auth(token)
            .send()
            .await
            .expect("plans request failed");
        assert_eq!(
            plans.status(),
            reqwest::StatusCode::OK,
            "issued token must be accepted by protected routes"
        );
    }

    let rejected = client
        .get(format!("{base_url}/api/cloud/plans"))
        .send()
        .await
        .expect("anonymous plans request failed");
    assert!(
        rejected.status() == reqwest::StatusCode::UNAUTHORIZED
            || rejected.status() == reqwest::StatusCode::FORBIDDEN,
        "unauthenticated request must be rejected, got {}",
        rejected.status()
    );
}

mod part2;
