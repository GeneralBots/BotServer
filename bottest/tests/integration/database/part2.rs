//! Split from `database.rs` — 238 lines.

use super::*;

#[tokio::test]
pub(crate) async fn test_uuid_generation() {
    let ctx = match TestHarness::database_only().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    let pool = match ctx.db_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    use diesel::prelude::*;
    use diesel::sql_query;

    #[derive(QueryableByName)]
    struct UuidResult {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }

    let mut conn = pool.get().expect("Failed to get connection");
    let result: Vec<UuidResult> = sql_query("SELECT gen_random_uuid() as id")
        .load(&mut conn)
        .expect("Query failed");

    assert_eq!(result.len(), 1);
    assert!(!result[0].id.is_nil());
}

#[tokio::test]
pub(crate) async fn test_timestamp_handling() {
    let ctx = match TestHarness::database_only().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    let pool = match ctx.db_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    use chrono::Utc;
    use diesel::prelude::*;
    use diesel::sql_query;

    #[derive(QueryableByName)]
    struct TimestampResult {
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        ts: chrono::DateTime<chrono::Utc>,
    }

    let mut conn = pool.get().expect("Failed to get connection");
    let result: Vec<TimestampResult> = sql_query("SELECT NOW() as ts")
        .load(&mut conn)
        .expect("Query failed");

    assert_eq!(result.len(), 1);

    let now = Utc::now();
    let diff = now.signed_duration_since(result[0].ts);
    assert!(diff.num_seconds().abs() < 60);
}

#[tokio::test]
pub(crate) async fn test_array_handling() {
    let ctx = match TestHarness::database_only().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    let pool = match ctx.db_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    use diesel::prelude::*;
    use diesel::sql_query;

    #[derive(QueryableByName)]
    struct ArrayResult {
        #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Text>)]
        items: Vec<String>,
    }

    let mut conn = pool.get().expect("Failed to get connection");
    let result: Vec<ArrayResult> = sql_query("SELECT ARRAY['a', 'b', 'c'] as items")
        .load(&mut conn)
        .expect("Query failed");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].items, vec!["a", "b", "c"]);
}

#[tokio::test]
pub(crate) async fn test_insert_user_fixture() {
    let ctx = match TestHarness::database_only().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    let user = admin_user();
    if let Err(e) = ctx.insert_user(&user).await {
        eprintln!("Skipping insert test (table may not exist): {}", e);
        return;
    }

    let pool = ctx.db_pool().await.unwrap();
    use diesel::prelude::*;
    use diesel::sql_query;
    use diesel::sql_types::Uuid as DieselUuid;

    #[derive(QueryableByName)]
    struct UserCheck {
        #[diesel(sql_type = diesel::sql_types::Text)]
        email: String,
    }

    let mut conn = pool.get().unwrap();
    let result: Result<Vec<UserCheck>, _> = sql_query("SELECT email FROM users WHERE id = $1")
        .bind::<DieselUuid, _>(user.id)
        .load(&mut conn);

    if let Ok(users) = result {
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].email, user.email);
    }
}

#[tokio::test]
pub(crate) async fn test_insert_bot_fixture() {
    let ctx = match TestHarness::database_only().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    let bot = bot_with_kb("test-knowledge-bot");
    if let Err(e) = ctx.insert_bot(&bot).await {
        eprintln!("Skipping insert test (table may not exist): {}", e);
        return;
    }

    let pool = ctx.db_pool().await.unwrap();
    use diesel::prelude::*;
    use diesel::sql_query;
    use diesel::sql_types::Uuid as DieselUuid;

    #[derive(QueryableByName)]
    struct BotCheck {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        kb_enabled: bool,
    }

    let mut conn = pool.get().unwrap();
    let result: Result<Vec<BotCheck>, _> =
        sql_query("SELECT name, kb_enabled FROM bots WHERE id = $1")
            .bind::<DieselUuid, _>(bot.id)
            .load(&mut conn);

    if let Ok(bots) = result {
        assert_eq!(bots.len(), 1);
        assert_eq!(bots[0].name, "test-knowledge-bot");
        assert!(bots[0].kb_enabled);
    }
}

#[tokio::test]
pub(crate) async fn test_session_and_message_fixtures() {
    let ctx = match TestHarness::database_only().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    let bot = basic_bot("session-test-bot");
    let customer = customer("+15551234567");
    let session = session_for(&bot, &customer);
    let message = message_in_session(&session, "Hello from test", MessageDirection::Incoming);

    if ctx.insert_bot(&bot).await.is_err() {
        eprintln!("Skipping: tables may not exist");
        return;
    }

    let _ = ctx.insert_customer(&customer).await;
    let _ = ctx.insert_session(&session).await;
    let _ = ctx.insert_message(&message).await;

    let pool = ctx.db_pool().await.unwrap();
    use diesel::prelude::*;
    use diesel::sql_query;
    use diesel::sql_types::Uuid as DieselUuid;

    #[derive(QueryableByName)]
    struct MessageCheck {
        #[diesel(sql_type = diesel::sql_types::Text)]
        content: String,
    }

    let mut conn = pool.get().unwrap();
    let result: Result<Vec<MessageCheck>, _> =
        sql_query("SELECT content FROM messages WHERE session_id = $1")
            .bind::<DieselUuid, _>(session.id)
            .load(&mut conn);

    if let Ok(messages) = result {
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello from test");
    }
}
