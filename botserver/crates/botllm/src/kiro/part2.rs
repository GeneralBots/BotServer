use super::*;

impl KiroClient {
    async fn emit_stream(
        &self,
        stream: impl futures_util::Stream<Item = reqwest::Result<bytes::Bytes>>,
        tx: &mpsc::Sender<String>,
        handler: &dyn ModelHandler,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use futures_util::StreamExt;
        let mut stream = std::pin::pin!(stream);
        let mut buffer = String::new();
        let mut state_buffer = String::new();
        const EVENT_PATTERNS: [&str; 11] = [
            "{\"content\":", "{\"name\":", "{\"input\":", "{\"stop\":",
            "{\"contextUsagePercentage\":", "{\"followupPrompt\":", "{\"usage\":",
            "{\"toolUseId\":", "{\"unit\":", "{\"error\":", "{\"Error\":",
        ];
        loop {
            let next = stream.next().await;
            if let Some(Ok(bytes)) = &next {
                buffer.push_str(&String::from_utf8_lossy(bytes));
            }
            // Parse all complete events out of the buffer.
            loop {
                let mut earliest: Option<usize> = None;
                for p in EVENT_PATTERNS.iter() {
                    if let Some(idx) = buffer.find(p) {
                        earliest = Some(match earliest {
                            Some(e) if e <= idx => e,
                            _ => idx,
                        });
                    }
                }
                let Some(start) = earliest else { break };
                let Some(end) = find_json_end(&buffer, start) else { break };
                let raw: String = buffer.drain(..end + 1).collect();
                let raw = raw[start..].to_string();
                if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
                    if let Some(content) = parsed.get("content").and_then(|c| c.as_str()) {
                        let processed = handler.process_content_streaming(content, &mut state_buffer);
                        if !processed.content.is_empty() {
                            let _ = tx.send(processed.content).await;
                        }
                        if !processed.reasoning.is_empty() {
                            let _ = tx
                                .send(serde_json::json!({"__reasoning__": processed.reasoning}).to_string())
                                .await;
                        }
                    } else if let Some(tool) = parsed.get("toolUseId").cloned() {
                        // toolUse events: {"toolUseId":…,"name":…,"input":…}
                        let _ = tool;
                        if let (Some(name), Some(input)) = (
                            parsed.get("name").and_then(|n| n.as_str()),
                            parsed.get("input").cloned(),
                        ) {
                            let args = match input {
                                Value::String(s) => s,
                                v => v.to_string(),
                            };
                            let msg = json!({"__tool_call__": true, "name": name, "arguments": args});
                            let _ = tx.send(msg.to_string()).await;
                        }
                    } else if let Some(err) = parsed.get("error").or_else(|| parsed.get("Error")) {
                        let msg = match err.as_str() {
                            Some(s) => s.to_string(),
                            None => err.to_string(),
                        };
                        let _ = tx
                            .send(format!("__kiro_error__{msg}"))
                            .await;
                    }
                }
            }
            if next.is_none() {
                break;
            }
        }
        Ok(())
    }
}
