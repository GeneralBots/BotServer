pub mod biometric;
pub mod digital_signature;
pub mod integration;
pub mod kyc;
pub mod routes_security;
pub mod schema;

pub use botsecurity_auth::*;
pub use botsecurity_core::*;
pub use botsecurity_crypto::*;
pub use botsecurity_protection::*;

// ─── Re-exports from sub-crate modules (flattened for backward compatibility) ───

// From botsecurity-core
pub use botsecurity_core::panic_handler::{
    catch_panic, catch_panic_async, panic_handler_middleware,
    panic_handler_middleware_with_config, set_global_panic_hook, PanicError,
    PanicGuard, PanicHandlerConfig,
};
pub use botsecurity_core::cors::{
    create_cors_layer, create_cors_layer_for_production, create_cors_layer_with_origins,
    get_cors_allowed_origins, set_cors_allowed_origins, strip_proxy_cors_middleware,
    CorsConfig, OriginValidator,
};
pub use botsecurity_core::headers::{
    create_security_headers_layer, security_headers_middleware,
    security_headers_middleware_default, CspBuilder, SecurityHeadersConfig,
};
pub use botsecurity_core::request_id::{
    generate_prefixed_request_id, generate_request_id, get_current_sequence,
    get_request_id, get_request_id_string, request_id_middleware,
    request_id_middleware_with_config, RequestId, RequestIdConfig,
    CORRELATION_ID_HEADER, REQUEST_ID_HEADER,
};
pub use botsecurity_core::command_guard::{
    has_nvidia_gpu_safe, safe_nvidia_smi, safe_pandoc_async, safe_pdftotext_async,
    sanitize_filename, validate_argument, validate_path, CommandGuardError, SafeCommand,
};
pub use botsecurity_core::error_sanitizer::{
    log_and_sanitize, log_and_sanitize_json, log_and_sanitize_str, safe_error,
    safe_error_with_request_id, sanitize_for_log, ErrorSanitizer, SafeErrorResponse,
};
pub use botsecurity_core::sql_guard::{
    build_safe_count_query, build_safe_delete_query, build_safe_select_by_id_query,
    build_safe_select_query, check_for_injection_patterns, escape_string_literal,
    is_table_allowed, sanitize_identifier, validate_identifier, validate_order_column,
    validate_order_direction, validate_table_name, SqlGuardError,
};
pub use botsecurity_core::validation::{
    sanitize_html, strip_html_tags, validate_alphanumeric, validate_email, validate_length,
    validate_no_html, validate_no_script_injection, validate_one_of, validate_password_strength,
    validate_phone, validate_range, validate_required, validate_slug, validate_url, validate_url_ssrf,
    validate_username, validate_uuid, ValidationError, ValidationResult, Validator,
};
pub use botsecurity_core::antivirus::{
    AntivirusConfig, AntivirusManager, ProtectionStatus, ScanResult, ScanStatus, ScanType, Threat,
    ThreatSeverity, ThreatStatus, Vulnerability,
};
pub use botsecurity_core::audit::{
    ActorType, AuditActor, AuditConfig, AuditEvent, AuditEventCategory, AuditEventType,
    AuditLogger, AuditOutcome, AuditQuery, AuditQueryResult, AuditResource, AuditSeverity,
    AuditStore, InMemoryAuditStore,
};
pub use botsecurity_core::path_guard::{
    canonicalize_safe, is_safe_path, join_safe, sanitize_path_component,
    PathGuard, PathGuardConfig, PathGuardError,
};
pub use botsecurity_core::file_validation::{
    FileValidationConfig, FileValidationResult, validate_file_upload,
};
pub use botsecurity_core::safe_unwrap::{safe_unwrap_or, safe_unwrap_or_default, safe_unwrap_none_or};
pub use botsecurity_core::webhook::*;
pub use botsecurity_core::dlp::*;
pub use botsecurity_core::prompt_security::*;
pub use botsecurity_core::security_monitoring::*;
pub use botsecurity_core::log_sanitizer::sanitize_log_value as sanitize_log_value_compact;

// From botsecurity-auth
pub use botsecurity_auth::auth::{
    admin_only_middleware, auth_middleware, auth_middleware_with_providers, bot_operator_middleware,
    bot_owner_middleware, bot_scope_middleware, extract_user_from_request, extract_user_with_providers,
    require_auth_middleware, require_bot_access, require_bot_permission, require_permission,
    require_permission_middleware, require_role, require_role_middleware, AuthConfig, AuthError,
    AuthenticatedUser, AuthMiddlewareState, BotAccess, Permission, Role,
};
pub use botsecurity_auth::auth_provider::{
    ApiKeyAuthProvider, ApiKeyInfo, AuthProvider, AuthProviderBuilder, AuthProviderRegistry,
    LocalJwtAuthProvider, ZitadelAuthProviderAdapter, create_default_registry,
};
pub use botsecurity_auth::blacklist::{
    BlacklistEntry, BlacklistStore, InMemoryBlacklistStore,
};
pub use botsecurity_auth::jwt::{
    Claims, JwtAlgorithm, JwtConfig, JwtKey, JwtManager, JwkSet, TokenIntrospectionResponse,
    TokenPair, TokenType, extract_bearer_token,
};
pub use botsecurity_auth::mfa::{
    MfaConfig, MfaManager, MfaMethod, MfaStatus, OtpChallenge, RecoveryCode, TotpAlgorithm,
    TotpEnrollment, UserMfaState, WebAuthnChallenge, WebAuthnCredential,
};
pub use botsecurity_auth::password::{
    Argon2Config, PasswordConfig, PasswordHasher2, PasswordIssue, PasswordStrength,
    PasswordValidationResult, generate_recovery_codes, generate_secure_password, hash_password,
    validate_password, verify_password,
};
pub use botsecurity_auth::rbac_middleware::{
    match_wildcard, rbac_middleware_fn, rbac_middleware as rbac_middleware_layer,
    require_admin_middleware, require_super_admin_middleware,
    RequirePermission, RequireRole, RequireResourceAccess,
    RbacManager, ResourceAcl, ResourcePermission, AccessDecision, AccessDecisionResult,
    RoutePermission, build_default_route_permissions, RbacConfig, RbacError,
};
pub use botsecurity_auth::session::{
    DeviceInfo, InMemorySessionStore, SameSite, Session, SessionConfig, SessionManager,
    SessionStatus, SessionStore, extract_session_id_from_cookie, generate_session_id,
};
pub use botsecurity_auth::csrf::{
    CsrfConfig, CsrfLayer, CsrfManager, CsrfToken, CsrfValidationResult,
    SameSite as CsrfSameSite, csrf_middleware, extract_csrf_from_cookie, extract_csrf_from_form,
};
pub use botsecurity_auth::zitadel_auth::{ZitadelAuthConfig, ZitadelAuthProvider, ZitadelUser};
pub use botsecurity_auth::rate_limiter::{
    create_default_rate_limit_layer, create_rate_limit_layer, rate_limit_middleware,
    simple_rate_limit_middleware, CombinedRateLimiter, HttpRateLimitConfig,
};
pub use botsecurity_auth::request_limits::{
    request_size_middleware, upload_size_middleware, DEFAULT_MAX_REQUEST_SIZE, MAX_UPLOAD_SIZE,
};
pub use botsecurity_auth::api_keys::{
    ApiKey as ManagedApiKey, ApiKeyConfig, ApiKeyManager, ApiKeyScope, ApiKeyStatus, ApiKeyUsage,
    CreateApiKeyRequest, CreateApiKeyResponse, RateLimitConfig as ApiKeyRateLimitConfig,
    extract_api_key_from_header,
};
#[cfg(feature = "cache")]
pub use botsecurity_auth::redis_blacklist_store::RedisBlacklistStore;
#[cfg(feature = "cache")]
pub use botsecurity_auth::redis_csrf_store::RedisCsrfManager;
#[cfg(feature = "cache")]
pub use botsecurity_auth::redis_session_store::RedisSessionStore;

// From botsecurity-crypto
pub use botsecurity_crypto::ca::{CaConfig, CaManager, CertificateRequest, CertificateResponse};
pub use botsecurity_crypto::cert_pinning::{
    compute_spki_fingerprint, format_fingerprint, parse_fingerprint, CertPinningConfig,
    CertPinningManager, PinType, PinValidationResult, PinnedCert, PinningStats,
};
pub use botsecurity_crypto::encryption::{
    EncryptedData, EncryptionAlgorithm, EncryptionConfig, EncryptionKey, EncryptionManager,
    EnvelopeEncryptedData, KeyPurpose, decrypt_field, derive_key_from_password, encrypt_field,
    generate_salt, hash_for_search,
};
pub use botsecurity_crypto::mutual_tls::{
    services::{
        configure_directory_mtls, configure_forgejo_mtls, configure_livekit_mtls,
        configure_postgres_mtls, configure_qdrant_mtls,
    },
    MtlsConfig, MtlsError, MtlsManager,
};
pub use botsecurity_crypto::secrets::{
    is_sensitive_key, redact_sensitive_data, ApiKey, DatabaseCredentials, JwtSecret, SecretBytes,
    SecretString, SecretsStore,
};
pub use botsecurity_crypto::tls::{create_https_server, ServiceTlsConfig, TlsConfig, TlsManager, TlsRegistry};

// From botsecurity-protection
pub use botsecurity_protection::protection::{
    configure_protection_routes, ProtectionManager, ProtectionTool, ToolStatus,
};
pub use botsecurity_protection::protection::script_guard::{
    BlockedPattern, InspectionContext, LlmCallRecord, RiskLevel, ScriptGuard,
    ScriptInspectionResult,
};
pub use botsecurity_protection::protection::script_guard::AuditLogger as ScriptAuditLogger;
