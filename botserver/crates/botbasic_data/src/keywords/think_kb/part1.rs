use super::*;

pub fn register_think_kb_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    let state_clone = state;
    let session_clone = user;

    if let Err(e) = engine.register_custom_syntax(["THINK", "KB", "$expr$"], true, move |context, inputs| {
        let query = context.eval_expression_tree(&inputs[0])?.to_string();

        info!(
            "THINK KB keyword executed - Query: '{}', Session: {}",
            query, session_clone.id
        );

        let session_id = session_clone.id;
        let bot_id = session_clone.bot_id;
        let user_id = session_clone.user_id;
        let db_pool = state_clone.db_pool().clone();
        let query_clone = query.clone();

        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match rt {
                Ok(rt) => rt.block_on(async { think_kb_search(&db_pool, session_id, bot_id, user_id, &query_clone).await }),
                Err(e) => Err(format!("Failed to create runtime: {}", e)),
            }
        })
        .join();

        match result {
            Ok(Ok(search_result)) => {
                info!(
                    "THINK KB completed - Found {} results, {} consolidation insights",
                    search_result.get("results")
                        .and_then(|r| r.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0),
                    search_result.get("consolidation_insights")
                        .and_then(|r| r.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0)
                );
                Ok(json_to_dynamic(search_result))
            }
            Ok(Err(e)) => {
                log::error!("THINK KB search failed: {}", e);
                Err(format!("THINK KB failed: {}", e).into())
            }
            Err(e) => {
                log::error!("THINK KB thread panic: {:?}", e);
                Err("THINK KB failed: thread panic".into())
            }
        }
    }) {
        log::error!("Failed to register THINK KB syntax: {e}");
    }
}
