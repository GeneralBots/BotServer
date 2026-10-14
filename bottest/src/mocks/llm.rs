use super::{new_expectation_store, Expectation, ExpectationStore};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct MockLLM {
    server: MockServer,
    port: u16,
    expectations: ExpectationStore,
    default_model: String,
    latency: Arc<Mutex<Option<Duration>>>,
    error_rate: Arc<Mutex<f32>>,
    call_count: Arc<AtomicUsize>,
    next_error: Arc<Mutex<Option<(u16, String)>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<ChatChoice>,
    usage: Usage,
}

#[derive(Serialize)]
struct ChatChoice {
    index: u32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Serialize)]
struct Usage {
    #[serde(rename = "prompt_tokens")]
    pub prompt: u32,
    #[serde(rename = "completion_tokens")]
    pub completion: u32,
    #[serde(rename = "total_tokens")]
    pub total: u32,
}

#[derive(Serialize)]
struct EmbeddingResponse {
    object: String,
    data: Vec<EmbeddingData>,
    model: String,
    usage: EmbeddingUsage,
}

#[derive(Serialize)]
struct EmbeddingData {
    object: String,
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Serialize)]
struct EmbeddingUsage {
    #[serde(rename = "prompt_tokens")]
    pub prompt: u32,
    #[serde(rename = "total_tokens")]
    pub total: u32,
}

#[derive(Serialize)]
struct StreamChunk {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<StreamChoice>,
}

#[derive(Serialize)]
struct StreamChoice {
    index: u32,
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Serialize)]
struct StreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    message: String,
    r#type: String,
    code: String,
}

impl MockLLM {
    pub async fn start(port: u16) -> Result<Self> {
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .context("Failed to bind MockLLM port")?;

        let server = MockServer::builder().listener(listener).start().await;

        let mock = Self {
            server,
            port,
            expectations: new_expectation_store(),
            default_model: "gpt-4".to_string(),
            latency: Arc::new(Mutex::new(None)),
            error_rate: Arc::new(Mutex::new(0.0)),
            call_count: Arc::new(AtomicUsize::new(0)),
            next_error: Arc::new(Mutex::new(None)),
        };

        mock.setup_default_routes().await;

        Ok(mock)
    }

    async fn setup_default_routes(&self) {
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"id": "gpt-4", "object": "model", "owned_by": "openai"},
                    {"id": "gpt-3.5-turbo", "object": "model", "owned_by": "openai"},
                    {"id": "text-embedding-ada-002", "object": "model", "owned_by": "openai"},
                ]
            })))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_completion(&self, prompt_contains: &str, response: &str) {
        {
            let mut store = self.expectations.lock().unwrap();
            store.insert(
                format!("completion:{prompt_contains}"),
                Expectation::new(&format!("completion containing '{prompt_contains}'")),
            );
        }

        let response_text = response.to_string();
        let model = self.default_model.clone();
        let latency = self.latency.clone();
        let call_count = self.call_count.clone();

        let response_body = ChatCompletionResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.clone(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(response_text),
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt: 10,
                completion: 20,
                total: 30,
            },
        };

        let mut template = ResponseTemplate::new(200).set_body_json(&response_body);

        let latency_value = *latency.lock().unwrap();
        if let Some(delay) = latency_value {
            template = template.set_delay(delay);
        }

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(body_partial_json(serde_json::json!({
                "messages": [{"content": prompt_contains}]
            })))
            .respond_with(template)
            .mount(&self.server)
            .await;

        call_count.fetch_add(0, Ordering::SeqCst);
    }

    pub async fn expect_streaming(&self, prompt_contains: &str, chunks: Vec<&str>) {
        let model = self.default_model.clone();
        let id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
        let created = chrono::Utc::now().timestamp() as u64;

        let mut sse_body = String::new();

        let first_chunk = StreamChunk {
            id: id.clone(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.clone(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                },
                finish_reason: None,
            }],
        };
        let _ = writeln!(
            sse_body,
            "data: {}\n",
            serde_json::to_string(&first_chunk).unwrap()
        );

        for chunk_text in &chunks {
            let chunk = StreamChunk {
                id: id.clone(),
                object: "chat.completion.chunk".to_string(),
                created,
                model: model.clone(),
                choices: vec![StreamChoice {
                    index: 0,
                    delta: StreamDelta {
                        role: None,
                        content: Some((*chunk_text).to_string()),
                    },
                    finish_reason: None,
                }],
            };
            let _ = writeln!(
                sse_body,
                "data: {}\n",
                serde_json::to_string(&chunk).unwrap()
            );
        }

        let final_chunk = StreamChunk {
            id: id.clone(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.clone(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
        };
        let _ = writeln!(
            sse_body,
            "data: {}\n",
            serde_json::to_string(&final_chunk).unwrap()
        );
        sse_body.push_str("data: [DONE]\n\n");

        let template = ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(sse_body);

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(body_partial_json(serde_json::json!({"stream": true})))
            // The caller names the prompt this stream belongs to; match on it
            // the same way `expect_completion` does.
            .and(body_partial_json(serde_json::json!({
                "messages": [{"content": prompt_contains}]
            })))
            .respond_with(template)
            .mount(&self.server)
            .await;
    }

    pub async fn expect_tool_call(
        &self,
        _prompt_contains: &str,
        tool_name: &str,
        tool_args: serde_json::Value,
    ) {
        let tool_call = ToolCall {
            id: format!("call_{}", uuid::Uuid::new_v4()),
            r#type: "function".to_string(),
            function: ToolFunction {
                name: tool_name.to_string(),
                arguments: serde_json::to_string(&tool_args).unwrap(),
            },
        };

        let response_body = ChatCompletionResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: self.default_model.clone(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: Some(vec![tool_call]),
                },
                finish_reason: "tool_calls".to_string(),
            }],
            usage: Usage {
                prompt: 10,
                completion: 20,
                total: 30,
            },
        };

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_embedding(&self, dimensions: usize) {
        let embedding: Vec<f32> = (0..dimensions)
            .map(|i| (i as f32) / (dimensions as f32))
            .collect();

        let response_body = EmbeddingResponse {
            object: "list".to_string(),
            data: vec![EmbeddingData {
                object: "embedding".to_string(),
                embedding,
                index: 0,
            }],
            model: "text-embedding-ada-002".to_string(),
            usage: EmbeddingUsage {
                prompt: 5,
                total: 5,
            },
        };

        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_embedding_for(&self, input_contains: &str, embedding: Vec<f32>) {
        let response_body = EmbeddingResponse {
            object: "list".to_string(),
            data: vec![EmbeddingData {
                object: "embedding".to_string(),
                embedding,
                index: 0,
            }],
            model: "text-embedding-ada-002".to_string(),
            usage: EmbeddingUsage {
                prompt: 5,
                total: 5,
            },
        };

        Mock::given(method("POST"))
            .and(path("/v1/embeddings"))
            .and(body_partial_json(
                serde_json::json!({"input": input_contains}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&self.server)
            .await;
    }

    pub fn with_latency(&self, ms: u64) {
        *self.latency.lock().unwrap() = Some(Duration::from_millis(ms));
    }

    pub fn with_error_rate(&self, rate: f32) {
        *self.error_rate.lock().unwrap() = rate.clamp(0.0, 1.0);
    }

    pub async fn next_call_fails(&self, status: u16, message: &str) {
        *self.next_error.lock().unwrap() = Some((status, message.to_string()));

        let error_body = ErrorResponse {
            error: ErrorDetail {
                message: message.to_string(),
                r#type: "error".to_string(),
                code: format!("error_{status}"),
            },
        };

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(status).set_body_json(&error_body))
            .expect(1)
            .mount(&self.server)
            .await;
    }

    pub async fn expect_rate_limit(&self) {
        let error_body = serde_json::json!({
            "error": {
                "message": "Rate limit exceeded",
                "type": "rate_limit_error",
                "code": "rate_limit_exceeded"
            }
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_json(&error_body)
                    .insert_header("retry-after", "60"),
            )
            .mount(&self.server)
            .await;
    }

    pub async fn expect_server_error(&self) {
        let error_body = serde_json::json!({
            "error": {
                "message": "Internal server error",
                "type": "server_error",
                "code": "internal_error"
            }
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_json(&error_body))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_auth_error(&self) {
        let error_body = serde_json::json!({
            "error": {
                "message": "Invalid API key",
                "type": "invalid_request_error",
                "code": "invalid_api_key"
            }
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(&error_body))
            .mount(&self.server)
            .await;
    }

    pub async fn set_default_response(&self, response: &str) {
        let response_body = ChatCompletionResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: self.default_model.clone(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some(response.to_string()),
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt: 10,
                completion: 20,
                total: 30,
            },
        };

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&self.server)
            .await;
    }

    #[must_use]
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn verify(&self) -> Result<()> {
        let store = self.expectations.lock().unwrap();
        for exp in store.values() {
            exp.verify()?;
        }
        Ok(())
    }

    pub async fn reset(&self) {
        self.server.reset().await;
        self.expectations.lock().unwrap().clear();
        self.call_count.store(0, Ordering::SeqCst);
        *self.next_error.lock().unwrap() = None;
        self.setup_default_routes().await;
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }

    pub async fn call_count(&self) -> usize {
        self.server.received_requests().await.map_or(0, |r| r.len())
    }

    pub async fn assert_called_times(&self, expected: usize) {
        let actual = self.call_count().await;
        assert_eq!(
            actual, expected,
            "Expected {expected} calls to MockLLM, but got {actual}"
        );
    }

    pub async fn assert_called(&self) {
        let count = self.call_count().await;
        assert!(
            count > 0,
            "Expected at least one call to MockLLM, but got none"
        );
    }

    pub async fn assert_not_called(&self) {
        let count = self.call_count().await;
        assert_eq!(count, 0, "Expected no calls to MockLLM, but got {count}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "get_weather".to_string(),
                arguments: r#"{"location": "NYC"}"#.to_string(),
            },
        };

        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("get_weather"));
        assert!(json.contains("call_123"));
    }

    #[test]
    fn test_chat_completion_response_serialization() {
        let response = ChatCompletionResponse {
            id: "test-id".to_string(),
            object: "chat.completion".to_string(),
            created: 1_234_567_890,
            model: "gpt-4".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                },
                finish_reason: "stop".to_string(),
            }],
            usage: Usage {
                prompt: 10,
                completion: 5,
                total: 15,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("chat.completion"));
        assert!(json.contains("Hello!"));
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_error_response_serialization() {
        let error = ErrorResponse {
            error: ErrorDetail {
                message: "Test error".to_string(),
                r#type: "test_error".to_string(),
                code: "test_code".to_string(),
            },
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Test error"));
        assert!(json.contains("test_code"));
    }
}
