use super::*;

/// Drive a provider stream to completion under [`LLM_CALL_TIMEOUT_SECS`].
///
/// The producer runs concurrently with the collector: `generate_stream` fills
/// a bounded channel, so awaiting it *before* draining deadlocks once the
/// model emits more chunks than the channel holds (a reasoning model writing
/// a whole BASIC program sends hundreds) — the producer blocks on `send`, the
/// collector never reaches `recv`, and the timeout then abandons a turn that
/// was actually progressing. Chat never hits this because its pipeline loops
/// on `recv` while the producer is still running.
pub async fn collect_llm_stream(
    llm_ops: &dyn LlmProviderOps,
    prompt: &str,
    config: &serde_json::Value,
    model: &str,
    key: &str,
    system_prompt: Option<&str>,
) -> Result<String, BoxError> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let producer = llm_ops.generate_stream(prompt, config, tx, model, key, system_prompt);
    let collect = async {
        let mut response = String::new();
        while let Some(chunk) = rx.recv().await {
            response.push_str(&chunk);
        }
        response
    };
    // Polled together in this task (no spawn: callers may run off-runtime):
    // the producer drains because the collector consumes concurrently. The
    // timeout stays as the stall guard for a provider that stops emitting.
    let call = async { tokio::join!(producer, collect) };
    let (producer_result, response) = tokio::time::timeout(
        std::time::Duration::from_secs(LLM_CALL_TIMEOUT_SECS),
        call,
    )
    .await
    .map_err(|_| format!("llm call timed out after {LLM_CALL_TIMEOUT_SECS}s"))?;
    // A failed stream fails the collect even with partial output: feeding
    // truncated JSON to the parser would turn a provider outage into a
    // fabricated fallback plan — an honest error lets the caller report
    // and retry instead.
    producer_result?;
    Ok(response)
}
