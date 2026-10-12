pub mod branding;
pub mod db_driver;
pub mod db_pool;
pub mod error;
pub mod key_utils;
#[cfg(feature = "http-client")]
pub mod http_client;
#[cfg(feature = "i18n")]
pub mod i18n;
pub mod limits;
pub mod logging;
pub mod message_types;
pub mod models;
pub mod os;
pub mod resilience;
pub mod security;
pub mod text;
pub mod traits;
pub mod version;
pub mod work_path;

pub use branding::{
    branding, init_branding, is_white_label, platform_name, platform_short, BrandingConfig,
};
pub use error::{BotError, BotResult};
#[cfg(feature = "i18n")]
pub use i18n::{available_locales, get, get_with_args, init as init_i18n, is_initialized, Locale};
pub use limits::{
    check_array_length_limit, check_file_size_limit, check_loop_limit, check_recursion_limit,
    check_string_length_limit, format_limit_error_response, LimitExceeded, LimitType, RateLimiter,
    SystemLimits, MAX_API_CALLS_PER_HOUR, MAX_API_CALLS_PER_MINUTE, MAX_ARRAY_LENGTH,
    MAX_BOTS_PER_TENANT, MAX_CONCURRENT_REQUESTS_GLOBAL, MAX_CONCURRENT_REQUESTS_PER_USER,
    MAX_DB_CONNECTIONS_PER_TENANT, MAX_DB_QUERY_RESULTS, MAX_DRIVE_STORAGE_BYTES,
    MAX_FILE_SIZE_BYTES, MAX_KB_DOCUMENTS_PER_BOT, MAX_KB_DOCUMENT_SIZE_BYTES,
    MAX_LLM_REQUESTS_PER_MINUTE, MAX_LLM_TOKENS_PER_REQUEST, MAX_LOOP_ITERATIONS,
    MAX_PENDING_TASKS, MAX_RECURSION_DEPTH, MAX_REQUEST_BODY_BYTES, MAX_SCRIPT_EXECUTION_SECONDS,
    MAX_SESSIONS_PER_USER, MAX_SESSION_IDLE_SECONDS, MAX_STRING_LENGTH, MAX_TOOLS_PER_BOT,
    MAX_UPLOAD_SIZE_BYTES, MAX_WEBSOCKET_CONNECTIONS_GLOBAL, MAX_WEBSOCKET_CONNECTIONS_PER_USER,
    RATE_LIMIT_BURST_MULTIPLIER, RATE_LIMIT_WINDOW_SECONDS,
};
pub use message_types::MessageType;
pub use models::{ApiResponse, BotResponse, Session, Suggestion, Switcher, TriggerKind, UserMessage, WorkflowExecution};
pub use resilience::{ResilienceError, RetryConfig};
pub use traits::{
    BoxFutureBool, BoxFutureDriveList, BoxFutureOptionDriveMeta, BoxFutureOptionValue,
    BoxFutureResult, BoxFutureString, BoxFutureUnit, BoxFutureValue, BoxFutureVecDriveObject,
    BoxFutureVecString, BoxFutureVecU8, BoxFutureVecValue, BoxError,
};
pub use version::{
    get_botserver_version, init_version_registry, register_component, version_string,
    ComponentSource, ComponentStatus, ComponentVersion, VersionRegistry, BOTSERVER_VERSION,
};

#[cfg(feature = "http-client")]
pub use http_client::BotServerClient;
