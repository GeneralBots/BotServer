//! SSE frame consumption for `OpenAIClient::generate_stream`.
//! Split from openai_stream.rs for file-size limits.

use log::{info, trace};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::llm_models::get_handler;

/// Drains the provider's SSE byte stream, forwarding content, reasoning and
/// tool-call deltas to `tx` until the provider closes the stream.
pub(crate) async fn consume_sse_stream(
    mut chunk_rx: mpsc::Receiver<Result<Vec<u8>, String>>,
    tx: mpsc::Sender<String>,
    model: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let handler = get_handler(model);
        let mut stream_state = String::new();
        let mut line_buffer = String::new(); // for SSE line fragmentation across chunks
        let mut first_bytes: Option<String> = None;
        let mut last_bytes = String::new();
        let mut total_size: usize = 0;
        let mut content_sent: usize = 0;
        // Parallel tool calls accumulate per tool_call index. A single shared
        // (name, args) pair concatenated the arguments of a second call into
        // the first one's JSON string (two classify_media calls produced
        // "{...}{...}"), which then failed to parse and executed with empty
        // arguments (issue: a no-caption video filed nothing).
        let mut tool_calls_by_index: std::collections::BTreeMap<i64, (String, String)> =
            std::collections::BTreeMap::new();
        let mut legacy_tool_call_name = String::new();
        let mut legacy_tool_call_args = String::new();
        let mut reasoning_from_field = String::new();   // from reasoning_content/reasoning API field
        let mut reasoning_from_content = String::new(); // from <think> tags inside content

        info!("LLM stream starting for model: {}", model);

    let idle_timeout = std::time::Duration::from_secs(60);
    const MAX_SSE_LINE_BUFFER: usize = 4 * 1024 * 1024;
    loop {
            let chunk_result = match tokio::time::timeout(idle_timeout, chunk_rx.recv()).await {
                Ok(Some(Ok(chunk))) => chunk,
                Ok(Some(Err(e))) => return Err(format!("Stream error: {}", e).into()),
                Ok(None) => break,
                Err(_) => {
                    log::error!("LLM stream idle timeout after 5s (no data received)");
                    return Err("LLM stream idle timeout after 5s".into());
                }
            };
            let chunk_str = String::from_utf8_lossy(&chunk_result);
            total_size += chunk_result.len();
            trace!("LLM chunk raw: {} bytes", chunk_result.len());
            if first_bytes.is_none() {
                first_bytes = Some(chunk_str.chars().take(100).collect());
            }
            last_bytes = chunk_str.chars().take(100).collect();

            // Accumulate lines across chunks to handle SSE fragmentation
            line_buffer.push_str(&chunk_str);
            if line_buffer.len() > MAX_SSE_LINE_BUFFER {
                log::error!(
                    "LLM SSE line buffer exceeded {} bytes without newline, dropping partial line",
                    MAX_SSE_LINE_BUFFER
                );
                line_buffer.clear();
                continue;
            }
            let mut pos = 0;
            while let Some(newline) = line_buffer[pos..].find('\n') {
                let end = pos + newline;
                let line = line_buffer[pos..end].trim_end_matches('\r').to_string();
                pos = end + 1;
                if !line.starts_with("data: ") || line.contains("[DONE]") {
                    continue;
                }
                if let Ok(data) = serde_json::from_str::<Value>(&line[6..]) {
                        // Log only when finish_reason or function/tool calls present
                        let fr = &data["choices"][0]["finish_reason"];
                        let has_fr = !fr.is_null();
                        let has_tc = !data["choices"][0]["delta"]["tool_calls"].is_null();
                        let has_fc = !data["choices"][0]["delta"]["function_call"].is_null();
                        if has_fr || has_tc || has_fc {
                            info!("SSE finish_reason={} tool_calls={} function_call={}", fr, has_tc, has_fc);
                        }
                        if let Some(filter_result) = data["choices"][0]["delta"]["content_filter_result"].as_object() {
                            if let Some(error) = filter_result.get("error") {
                                let code = error.get("code").and_then(|c| c.as_str()).unwrap_or("unknown");
                                let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("no message");
                                log::error!("LLM Content filter error: code={}, message={}", code, message);
                            } else {
                                trace!("LLM Content filter result (no error): {:?}", filter_result);
                            }
                        }

                        // Different models put data in different fields:
                        //   NVIDIA/Nemotron:    content=answer, reasoning=thinking (separate chunks)
                        //   DeepSeek paid:      content=answer, reasoning_content=thinking
                        let content_text = data["choices"][0]["delta"]["content"].as_str()
                            .map(|s| s.to_string())
                            .filter(|s| !s.is_empty());

                        // Reasoning/thinking from separate API field
                        let reasoning_text = data["choices"][0]["delta"]["reasoning"].as_str()
                            .or_else(|| data["choices"][0]["delta"]["reasoning_content"].as_str())
                            .map(|s| s.to_string());

                        // Always accumulate reasoning from separate API field
                        if let Some(ref reasoning) = reasoning_text {
                            reasoning_from_field.push_str(reasoning);
                        }

                        // Process content through handler — this extracts think-tag reasoning
                        if let Some(ref content) = content_text {
                            if !content.is_empty() {
                                let processed = handler.process_content_streaming(content, &mut stream_state);
                                if !processed.content.is_empty() {
                                    content_sent += processed.content.len();
                                    let _ = tx.send(processed.content).await;
                                }
                                if !processed.reasoning.is_empty() {
                                    reasoning_from_content.push_str(&processed.reasoning);
                                    // Stream reasoning chunks live to the frontend
                                    let reasoning_chunk = serde_json::json!({
                                        "__reasoning__": processed.reasoning.trim()
                                    }).to_string();
                                    let _ = tx.send(reasoning_chunk).await;
                                }
                            }
                        }

                        // Handle legacy function_call format (Groq legacy functions API)
                        if legacy_tool_call_name.is_empty() {
                            if let Some(func_call) = data["choices"][0]["delta"]["function_call"].as_object() {
                                if let Some(name) = func_call.get("name").and_then(|n| n.as_str()) {
                                    legacy_tool_call_name = name.to_string();
                                }
                                if let Some(args) = func_call.get("arguments").and_then(|a| a.as_str()) {
                                    legacy_tool_call_args.push_str(args);
                                }
                            }
                        }

                        // Handle modern tool_calls format (OpenAI tools API)
                        // Accumulate across streaming chunks per tool_call index,
                        // don't send via tx
                        if let Some(tool_calls) = data["choices"][0]["delta"]["tool_calls"].as_array() {
                            if !tool_calls.is_empty() {
                                info!("SSE delta with tool_calls: {} entries, first: {:?}", tool_calls.len(), tool_calls[0].get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()));
                            }
                            for tool_call in tool_calls {
                                let index = tool_call.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
                                if let Some(func) = tool_call.get("function") {
                                    let entry = tool_calls_by_index.entry(index).or_insert_with(|| (String::new(), String::new()));
                                    if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                        entry.0 = name.to_string();
                                    }
                                    if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                                        entry.1.push_str(args);
                                    }
                                }
                            }
                        }

                        // Handle legacy function_call format (NVIDIA, some open-source models)
                        if let Some(func_call) = data["choices"][0]["delta"]["function_call"].as_object() {
                            if let Some(name) = func_call.get("name").and_then(|n| n.as_str()) {
                                legacy_tool_call_name = name.to_string();
                            }
                            if let Some(args) = func_call.get("arguments").and_then(|a| a.as_str()) {
                                legacy_tool_call_args.push_str(args);
                            }
                        }
                    }
                }
            // Keep trailing partial line for next chunk
            line_buffer = line_buffer[pos..].to_string();
        }

        // Combine all reasoning from both sources
        let mut total_reasoning = reasoning_from_field;
        if !reasoning_from_content.is_empty() {
            if !total_reasoning.is_empty() {
                total_reasoning.push('\n');
            }
            total_reasoning.push_str(&reasoning_from_content);
        }

        // Send accumulated reasoning to frontend as a thinking block
        if !total_reasoning.is_empty() {
            let reasoning_msg = serde_json::json!({
                "__reasoning__": total_reasoning.trim()
            }).to_string();
            let _ = tx.send(reasoning_msg).await;
            info!("LLM reasoning sent: {} bytes", total_reasoning.len());
        }

        // After streaming ends, send every accumulated tool call. Parallel
        // calls (index > 0) are sent as separate __tool_call__ messages so
        // each executes with its own arguments instead of being merged into
        // the first call's JSON.
        for (index, (name, args)) in tool_calls_by_index.iter() {
            if name.is_empty() || args.is_empty() {
                continue;
            }
            if *index > 0 {
                info!("LLM tool_call #{} accumulated: {} with {} bytes args", index + 1, name, args.len());
            } else {
                info!("LLM tool_call accumulated: {} with {} bytes args", name, args.len());
            }
            let tool_call_msg = serde_json::json!({
                "__tool_call__": true,
                "name": name,
                "arguments": args
            });
            let _ = tx.send(tool_call_msg.to_string()).await;
            tokio::task::yield_now().await;
        }
        if !legacy_tool_call_name.is_empty() && !legacy_tool_call_args.is_empty() {
            let tool_call_msg = serde_json::json!({
                "__tool_call__": true,
                "name": legacy_tool_call_name,
                "arguments": legacy_tool_call_args
            });
            let _ = tx.send(tool_call_msg.to_string()).await;
            tokio::task::yield_now().await;
            info!("LLM legacy function_call accumulated: {} with {} bytes args", legacy_tool_call_name, legacy_tool_call_args.len());
        }

        trace!("LLM stream done: size={} bytes, content_sent={}, reasoning={}B, first={:?}, last={}",
            total_size, content_sent, total_reasoning.len(), first_bytes, last_bytes);

        Ok(())
}
