use bottest::prelude::*;

#[tokio::test]
async fn test_database_ping() {
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
    use diesel::sql_types::Text;

    #[derive(QueryableByName)]
    struct PingResult {
        #[diesel(sql_type = Text)]
        result: String,
    }

    let mut conn = pool.get().expect("Failed to get connection");
    let result: Vec<PingResult> = sql_query("SELECT 'pong' as result")
        .load(&mut conn)
        .expect("Query failed");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].result, "pong");
}

#[tokio::test]
async fn test_execute_raw_sql() {
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
    struct SumResult {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        sum: i32,
    }

    let mut conn = pool.get().expect("Failed to get connection");
    let result: Vec<SumResult> = sql_query("SELECT 2 + 2 as sum")
        .load(&mut conn)
        .expect("Query failed");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].sum, 4);
}

#[tokio::test]
async fn test_transaction_rollback() {
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

    let mut conn = pool.get().expect("Failed to get connection");

    let result: Result<(), diesel::result::Error> = conn
        .transaction::<(), diesel::result::Error, _>(|conn| {
            sql_query("SELECT 1").execute(conn)?;
            Err(diesel::result::Error::RollbackTransaction)
        });

    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_connections() {
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

    let conn1 = pool.get();
    let conn2 = pool.get();
    let conn3 = pool.get();

    assert!(conn1.is_ok());
    assert!(conn2.is_ok());
    assert!(conn3.is_ok());
}

#[tokio::test]
async fn test_query_result_types() {
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
    struct TypeTestRow {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        integer: i32,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        bigint: i64,
        #[diesel(sql_type = diesel::sql_types::Text)]
        text: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        flag: bool,
        #[diesel(sql_type = diesel::sql_types::Double)]
        decimal: f64,
    }

    let mut conn = pool.get().expect("Failed to get connection");
    let result: Vec<TypeTestRow> = sql_query(
        "SELECT
            42 as integer,
            9223372036854775807::bigint as bigint,
            'hello' as text,
            true as flag,
            3.125 as decimal",
    )
    .load(&mut conn)
    .expect("Query failed");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].integer, 42);
    assert_eq!(result[0].bigint, 9_223_372_036_854_775_807_i64);
    assert_eq!(result[0].text, "hello");
    assert!(result[0].flag);
    assert!((result[0].decimal - 3.125).abs() < 0.0001);
}

#[tokio::test]
async fn test_null_handling() {
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
    struct NullTestResult {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        nullable_val: Option<String>,
    }

    let mut conn = pool.get().expect("Failed to get connection");

    let result: Vec<NullTestResult> = sql_query("SELECT NULL::text as nullable_val")
        .load(&mut conn)
        .expect("Query failed");

    assert_eq!(result.len(), 1);
    assert!(result[0].nullable_val.is_none());

    let result: Vec<NullTestResult> = sql_query("SELECT 'value'::text as nullable_val")
        .load(&mut conn)
        .expect("Query failed");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].nullable_val, Some("value".to_string()));
}

#[tokio::test]
async fn test_json_handling() {
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
    struct JsonTestResult {
        #[diesel(sql_type = diesel::sql_types::Text)]
        json_text: String,
    }

    let mut conn = pool.get().expect("Failed to get connection");
    let result: Vec<JsonTestResult> =
        sql_query(r#"SELECT '{"key": "value", "number": 42}'::jsonb::text as json_text"#)
            .load(&mut conn)
            .expect("Query failed");

    assert_eq!(result.len(), 1);

    let parsed: serde_json::Value =
        serde_json::from_str(&result[0].json_text).expect("Failed to parse JSON");

    assert_eq!(parsed["key"], "value");
    assert_eq!(parsed["number"], 42);
}

mod part2;
