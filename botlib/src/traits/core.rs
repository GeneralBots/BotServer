use std::fmt::Debug;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
pub type BoxFutureResult = std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, BoxError>> + Send>>;
pub type BoxFutureString = std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>;
pub type BoxFutureUnit = std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>;
pub type BoxFutureBool = std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool, String>> + Send>>;
pub type BoxFutureVecU8 = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + Send>>;
pub type BoxFutureVecString = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send>>;
pub type BoxFutureVecDriveObject = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<DriveObjectInfo>, String>> + Send>>;
pub type BoxFutureOptionDriveMeta = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<DriveObjectMetadata>, String>> + Send>>;
pub type BoxFutureDriveList = std::pin::Pin<Box<dyn std::future::Future<Output = Result<DriveListResult, String>> + Send>>;
pub type BoxFutureOptionValue = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send>>;
pub type BoxFutureOptionString = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send>>;
pub type BoxFutureValue = std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>>;
pub type BoxFutureVecValue = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<serde_json::Value>, String>> + Send>>;
/// Future that borrows the adapter, for trait methods that build a boxed
/// future from `&self` (the channel-transport half of [`ChannelAdapter`]).
pub type BoxFutureRefUnit<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), BoxError>> + Send + 'a>>;
pub type BoxFutureRefOptionString<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send + 'a>>;
pub type BoxFutureRefValue<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'a>>;

/// The single LLM provider contract (#1498).
///
/// `botllm::LLMProvider` is a re-export of this trait, so the nine provider
/// implementations (OpenAI, Claude, Bedrock, Vertex, GLM, Kimi, Kiro and the
/// dynamic/cache wrappers) and the handful of callers that hold an
/// `Arc<dyn LLMProvider>` all speak the same trait — a provider fix can no
/// longer miss a caller that picked a different abstraction.
///
/// `generate_simple` / `generate_with_context` are conveniences for callers
/// that have no per-call model or key. They default to an empty model/key,
/// which is what a provider with its own configured defaults wants; the
/// botserver wraps a provider in `BotlibLLMProviderWrapper` to substitute the
/// bot's configured model and key instead.
#[async_trait::async_trait]
pub trait LLMProvider: Send + Sync + Debug {
    async fn generate(
        &self,
        prompt: &str,
        config: &serde_json::Value,
        model: &str,
        key: &str,
    ) -> Result<String, BoxError>;

    async fn generate_stream(
        &self,
        prompt: &str,
        config: &serde_json::Value,
        tx: tokio::sync::mpsc::Sender<String>,
        model: &str,
        key: &str,
        tools: Option<&Vec<serde_json::Value>>,
    ) -> Result<(), BoxError>;

    /// Cancels an in-flight generation; providers without a job registry
    /// ignore it.
    async fn cancel_job(&self, _session_id: &str) -> Result<(), BoxError> {
        Ok(())
    }

    async fn generate_simple(&self, prompt: &str) -> Result<String, String> {
        self.generate(prompt, &serde_json::Value::Null, "", "")
            .await
            .map_err(|e| e.to_string())
    }

    async fn generate_with_context(&self, prompt: &str, context: &str) -> Result<String, String> {
        self.generate(
            &format!("{context}\n\n{prompt}"),
            &serde_json::Value::Null,
            "",
            "",
        )
        .await
        .map_err(|e| e.to_string())
    }
}

/// The single channel contract (#1499).
///
/// The web and voice runtime adapters and the Telegram, Instagram and Teams
/// adapters all implement it, so a caller holding `dyn ChannelAdapter` can
/// drive any transport. A new channel registers itself by implementing this
/// trait — there is no per-channel copy of it any more.
///
/// Delivery has two shapes:
///
/// * `send_message` — the synchronous runtime path (web socket, voice);
/// * `send_bot_response` — the asynchronous path every channel transport uses,
///   since posting a full bot response is always I/O.
///
/// A transport that cannot serve one of them inherits the default, which says
/// so explicitly rather than silently succeeding.
pub trait ChannelAdapter: Send + Sync + Debug {
    /// Stable channel id: `web`, `voice`, `telegram`, `instagram`, `msteams`.
    fn channel_type(&self) -> &str;

    /// Human-readable name for the UI.
    fn display_name(&self) -> &'static str {
        "Unknown"
    }

    /// `false` when the channel is missing its credentials.
    fn is_configured(&self) -> bool {
        true
    }

    fn send_message(&self, to: &str, message: &str) -> Result<(), String> {
        let _ = (to, message);
        Err(format!(
            "{}: synchronous send is not supported, use send_bot_response",
            self.channel_type()
        ))
    }

    /// Delivers a full bot response through the channel transport.
    fn send_bot_response<'a>(
        &'a self,
        _response: crate::models::BotResponse,
    ) -> BoxFutureRefUnit<'a> {
        let channel = self.channel_type().to_string();
        Box::pin(async move {
            Err(Box::new(std::io::Error::other(format!(
                "{channel}: delivery is not implemented"
            ))) as BoxError)
        })
    }

    /// Normalises an inbound payload into chat text; `None` when the payload
    /// carries no message for this channel.
    fn receive_message<'a>(
        &'a self,
        _payload: serde_json::Value,
    ) -> BoxFutureRefOptionString<'a> {
        Box::pin(async { Ok(None) })
    }

    /// Platform-specific user record, defaulting to id + platform.
    fn get_user_info<'a>(&'a self, user_id: &str) -> BoxFutureRefValue<'a> {
        let platform = self.channel_type().to_string();
        let id = user_id.to_string();
        Box::pin(async move { Ok(serde_json::json!({ "id": id, "platform": platform })) })
    }

    fn send_message_to_session(&self, session_id: &str, message: &str) -> Result<(), String> {
        let _ = (session_id, message);
        Err("send_message_to_session: not implemented".to_string())
    }

    fn add_connection(&self, session_id: &str, sender: std::sync::mpsc::Sender<String>) -> Result<(), String> {
        let _ = (session_id, sender);
        Err("add_connection: not implemented".to_string())
    }

    fn remove_connection(&self, session_id: &str) -> Result<(), String> {
        let _ = session_id;
        Err("remove_connection: not implemented".to_string())
    }

    fn start_voice_session(&self, session_id: &str, user_id: &str) -> BoxFutureString {
        let _ = (session_id, user_id);
        Box::pin(async { Err("start_voice_session: not implemented".to_string()) })
    }

    fn stop_voice_session(&self, session_id: &str) -> BoxFutureUnit {
        let _ = session_id;
        Box::pin(async { Err("stop_voice_session: not implemented".to_string()) })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DriveObjectMetadata {
    pub size: u64,
    pub content_type: Option<String>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DriveObjectInfo {
    pub key: String,
    pub etag: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DriveListEntry {
    pub key: String,
    pub size: u64,
}

impl DriveListEntry {
    pub fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DriveListResult {
    pub objects: Vec<DriveListEntry>,
    pub common_prefixes: Vec<String>,
}

impl DriveListResult {
    pub fn contents(&self) -> &[DriveListEntry] {
        &self.objects
    }

    pub fn common_prefixes(&self) -> &[String] {
        &self.common_prefixes
    }
}

pub trait DriveRepository: Send + Sync + Debug {
    fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> BoxFutureUnit;

    fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> BoxFutureVecU8;

    fn delete_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> BoxFutureUnit;

    fn copy_object(
        &self,
        bucket: &str,
        from_key: &str,
        to_key: &str,
    ) -> BoxFutureUnit;

    fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<&str>,
    ) -> BoxFutureVecString;

    fn list_objects_with_metadata(
        &self,
        bucket: &str,
        prefix: Option<&str>,
    ) -> BoxFutureVecDriveObject;

    fn list_common_prefixes(
        &self,
        bucket: &str,
        delimiter: &str,
    ) -> BoxFutureVecString;

    fn list_all_buckets(
        &self,
    ) -> BoxFutureVecString;

    fn object_exists(
        &self,
        bucket: &str,
        key: &str,
    ) -> BoxFutureBool;

    fn get_object_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> BoxFutureOptionDriveMeta;

    fn create_bucket_if_not_exists(
        &self,
        bucket: &str,
    ) -> BoxFutureUnit;

    // Bucket deletion is only done at organization level (cloud),
    // not at the individual bot level.

    fn delete_objects(
        &self,
        bucket: &str,
        keys: Vec<String>,
    ) -> BoxFutureUnit;

    fn head_bucket(
        &self,
        bucket: &str,
    ) -> BoxFutureBool;

    fn list_objects_v2(
        &self,
        bucket: &str,
        prefix: &str,
        delimiter: Option<&str>,
    ) -> BoxFutureDriveList;

    fn upload_file(
        &self,
        bucket: &str,
        key: &str,
        file_path: &str,
        content_type: Option<&str>,
    ) -> BoxFutureUnit;

    fn download_file(
        &self,
        bucket: &str,
        key: &str,
        file_path: &str,
    ) -> BoxFutureUnit;

    fn get_object_direct(
        &self,
        bucket: &str,
        key: &str,
    ) -> BoxFutureVecU8 {
        self.get_object(bucket, key)
    }

    fn list_buckets(
        &self,
    ) -> BoxFutureVecString {
        self.list_all_buckets()
    }
}
