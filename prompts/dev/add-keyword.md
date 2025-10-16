Create a new Rhai custom keyword implementation with these specifications:

- When generating new Rhai keyword handlers in Rust, always design them to be thread-safe and fully compatible with async execution. Use Arc for shared state, perform heavy or async operations with tokio::task::block_in_place and Handle::current().block_on, and return results as rhai::Dynamic to maintain safe cross-thread communication between Rust and the Rhai engine.

1. DATABASE REQUIREMENTS:
- No enums in database schema (only in Rust code)
- Use direct integer values for enum variants in queries
- Follow existing connection pooling pattern with AppState
- Include proper error handling and logging

2. RUST IMPLEMENTATION:
- Enum definition (Rust-only, no DB enum):
```rust
#[repr(i32)]
pub enum KeywordAction {
    Action1 = 0,
    Action2 = 1,
    Action3 = 2
}
```

3. KEYWORD TEMPLATE:
```rust
pub fn {keyword_name}_keyword(state: &AppState, engine: &mut Engine) {
    let db = state.db_custom.clone();

    engine.register_custom_syntax(
        {syntax_pattern},
        {is_raw},
        {
            let db = db.clone();
            move |context, inputs| {
                // Input processing
                {input_processing}

                let binding = db.as_ref().unwrap();
                let fut = execute_{keyword_name}(binding, {params});

                let result = tokio::task::block_in_place(||
                    tokio::runtime::Handle::current().block_on(fut))
                        .map_err(|e| format!("DB error: {}", e))?;

                {result_handling}
            }
        }
    ).unwrap();
}

pub async fn execute_{keyword_name}(
    pool: &PgPool,
    {params_with_types}
) -> Result<Value, Box<dyn std::error::Error>> {
    info!("Executing {keyword_name} with: {debug_params}");

    let result = sqlx::query(
        "{sql_query_with_i32_enum}"
    )
    .bind({enum_value} as i32)
    {additional_binds}
    .execute(pool)
    .await?;

    Ok(json!({
        "command": "{keyword_name}",
        {result_fields}
        "rows_affected": result.rows_affected()
    }))
}
```

4. EXAMPLE IMPLEMENTATION (SET SCHEDULE):
```rust
// Enum (Rust-only)
#[repr(i32)]
pub enum TriggerKind {
    Scheduled = 0,
    TableUpdate = 1,
    TableInsert = 2,
    TableDelete = 3
}

// Keyword implementation
pub fn set_schedule_keyword(state: &AppState, engine: &mut Engine) {
    let db = state.db_custom.clone();

    engine.register_custom_syntax(
        ["SET", "SCHEDULE", "$string$"],
        true,
        {
            let db = db.clone();
            move |context, inputs| {
                let cron = context.eval_expression_tree(&inputs[0])?.to_string();
                let script_name = format!("cron_{}.rhai", cron.replace(' ', "_"));

                let binding = db.as_ref().unwrap();
                let fut = execute_set_schedule(binding, &cron, &script_name);

                let result = tokio::task::block_in_place(||
                    tokio::runtime::Handle::current().block_on(fut))
                        .map_err(|e| format!("DB error: {}", e))?;

                if let Some(rows_affected) = result.get("rows_affected") {
                    Ok(Dynamic::from(rows_affected.as_i64().unwrap_or(0)))
                } else {
                    Err("No rows affected".into())
                }
            }
        }
    ).unwrap();
}

pub async fn execute_set_schedule(
    pool: &PgPool,
    cron: &str,
    script_name: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    info!("Executing schedule: {}, {}", cron, script_name);

    let result = sqlx::query(
        "INSERT INTO system_automations
        (kind, schedule, script_name)
        VALUES ($1, $2, $3)"
    )
    .bind(TriggerKind::Scheduled as i32)
    .bind(cron)
    .bind(script_name)
    .execute(pool)
    .await?;

    Ok(json!({
        "command": "set_schedule",
        "schedule": cron,
        "script_name": script_name,
        "rows_affected": result.rows_affected()
    }))
}
```

5. ADDITIONAL REQUIREMENTS:
- Maintain consistent tokio runtime handling
- Include parameter validation
- Follow existing JSON response format
- Ensure proper script name generation
- Include debug logging for all operations

6. OUTPUT FORMAT:
Provide complete implementation with:
1. Rust enum definition
2. Keyword registration function
3. Execution function
4. Example usage in Rhai
