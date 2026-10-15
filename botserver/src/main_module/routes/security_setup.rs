use std::sync::Arc;
use log::{info, error};
use crate::security::auth::{AuthConfig, AuthMiddlewareState};
use crate::security::auth_provider::{ApiKeyAuthProvider, AuthProviderBuilder};
use crate::security::cors::create_cors_layer;
use crate::security::jwt::{JwtConfig, JwtKey, JwtManager};
use crate::security::rbac_middleware::{build_default_route_permissions, RbacConfig, RbacManager};
use botcore::shared::state::AppState;

pub struct SecurityComponents {
    pub cors: tower_http::cors::CorsLayer,
    pub auth_config: Arc<AuthConfig>,
    pub jwt_manager: Option<Arc<JwtManager>>,
    pub rbac_manager: Arc<RbacManager>,
    pub auth_provider_registry: Arc<dyn std::any::Any + Send + Sync>,
    pub auth_middleware_state: AuthMiddlewareState,
}

pub async fn setup_security(app_state: &Arc<AppState>) -> SecurityComponents {
    use botcore::config::ConfigManager;

    let cfg = ConfigManager::new(app_state.conn.clone());
    let origins_str = cfg.get_config(&uuid::Uuid::nil(), "cors-allowed-origins", None).unwrap_or_default();
    if !origins_str.is_empty() {
        let origins: Vec<String> = origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !origins.is_empty() {
            info!("Loaded {} CORS allowed origins from config", origins.len());
            crate::security::cors::set_cors_allowed_origins(origins);
        }
    }

    let cors = create_cors_layer();

    let auth_config = {
        let mut cfg = AuthConfig::from_env();
        // ⚠️ Remove catch-all "/" from public paths — CRITICAL SECURITY.
        // The default AuthConfig includes "/" which makes every endpoint
        // publicly accessible, including /api/files/buckets that enumerates
        // all MinIO buckets. Only explicitly listed paths are public.
        cfg.public_paths.retain(|p| p != "/");

        Arc::new(cfg
            .add_anonymous_path("/health")
            .add_anonymous_path("/healthz")
            .add_anonymous_path("/api/health")
            .add_anonymous_path("/api/product")
            .add_anonymous_path("/api/manifest")
            .add_anonymous_path("/api/i18n")
            .add_anonymous_path("/api/auth")
            .add_anonymous_path("/api/auth/login")
            .add_anonymous_path("/api/auth/refresh")
            .add_anonymous_path("/api/auth/bootstrap")
            .add_anonymous_path("/api/setup/status")
            .add_anonymous_path("/api/bot/public")
            .add_anonymous_path("/api/marketing/track/")
            .add_anonymous_path("/api/suggestions")
            .add_anonymous_path("/api/client-errors")
            // #1342 — the app catalog feeds the desktop launcher, which must
            // render for anonymous visitors of a public bot (the reported
            // "Design app won't open" is the launcher grid failing on the
            // 401). The handler already scopes per-tenant and degrades to the
            // global (nil) scope for unbound callers, so anonymous reads
            // never leak other orgs' bots or published apps.
            .add_anonymous_path("/api/apps/catalog")
            // #1428 — the Integrations provider catalog is the same static
            // surface: an internal app's browse list. The handler serves the
            // built-in provider seeds only (no tenant data), so the 401 the
            // middleware answered made the catalog show a "Sign in"
            // interstitial that blocked the whole app.
            .add_anonymous_path("/api/apps/integrations/catalog")
            .add_anonymous_path("/ws")
            // Terminal WS: the browser WebSocket API cannot send an
            // Authorization header, so the upgrade is anonymous and gated by
            // the session id (?id=...) issued by the authenticated
            // /api/terminal/create endpoint (a random UUID capability).
            .add_anonymous_path("/api/terminal/ws")
            .add_anonymous_path("/auth")
            .add_anonymous_path("/webhook/whatsapp")
            .add_anonymous_path("/api/whatsapp/webhook")
            .add_anonymous_path("/api/facebook/webhook")
            // Inbound channel deliveries carry no token: the provider posts
            // straight from its own infrastructure. Each handler proves the
            // call itself (Telegram secret token / Instagram verify token +
            // signature), so the transport gate must let them in — otherwise
            // the whole channel is unreachable in production (#1327, #1330).
            .add_anonymous_path("/webhook/telegram")
            // Per-bot Telegram webhook: the bot name selects the bot; the
            // handler proves the call with the secret-token header (#1327).
            .add_anonymous_path("/webhook/telegram/*")
            .add_anonymous_path("/api/instagram/webhook")
            .add_anonymous_path("/api/msteams/messages")
            .add_anonymous_path("/api/catalog")
            // Only the bot access-check endpoint is anonymous: it must answer
            // whether a bot is public before the caller has a token. The
            // trailing-slash `/api/bots/` previously listed here was a no-op
            // (the matcher produced the `/api/bots//` prefix), so bot config
            // mutations stayed protected — keep it that way with an exact glob.
            .add_anonymous_path("/api/bots/*/access")
            .add_anonymous_path("/api/cloud/auth/login")
            .add_anonymous_path("/api/cloud/auth/signup")
            .add_anonymous_path("/api/cloud/auth")
            .add_anonymous_path("/api/cloud/store")
            .add_anonymous_path("/api/cloud/plans")
            .add_anonymous_path("/api/cloud/offers")
            .add_anonymous_path("/api/cloud/llm-providers")
            // Public store catalog only: the Store page reads
            // /api/products/items anonymously; all other /api/products/*
            // (pricelists, inventory, low-stock, services, stats) stay
            // authenticated — prefix matching must never widen this to the
            // whole namespace (issue #738).
            .add_anonymous_path("/api/products/items")
            .add_anonymous_path("/api/sheet")
            .add_anonymous_path("/api/ui/sheet")
            .add_anonymous_path("/suite/sheet")
            .add_public_path("/static")
            .add_public_path("/favicon.ico")
            .add_public_path("/suite")
            .add_public_path("/themes")
            .add_public_path("/api/product")
            .add_public_path("/api/auth/suite-sso")
            // Vibe domain forward-auth: Caddy calls this without a token;
            // it validates the domain cookie/JWT itself (public gate).
            .add_anonymous_path("/api/vibe/domain-auth")
            // Drive public share links: token-only anonymous download. The
            // token is a random 128-bit UUID captured at creation time; this
            // prefix only exposes the exact-token download handler, nothing
            // else under /api/files.
            .add_anonymous_path("/api/files/public")
            // Integration OAuth2 callback: the provider browser redirect
            // carries no JWT; the HMAC-signed state parameter is the
            // authenticity proof (verified inside the handler).
            .add_anonymous_path("/api/bots/*/integrations/oauth/*/callback")
            // External drive OAuth callback (OneDrive / Google Drive): same
            // shape — the provider redirects a browser that carries no API
            // token, and the HMAC-signed state is the authenticity proof
            // (verified in the handler before any token is stored).
            .add_anonymous_path("/api/external-drives/callback")
        )
    };

    let jwt_secret = crate::main_module::directory_setup::resolve_saas_jwt_secret();

    let jwt_config = JwtConfig::default();
    let jwt_key = JwtKey::from_secret(&jwt_secret);
    let jwt_manager = match JwtManager::new(jwt_config, jwt_key) {
        Ok(manager) => {
            #[cfg(feature = "cache")]
            let manager = {
                // Persist the token blacklist in the shared cache so revoked
                // tokens stay rejected across process restarts (#901). Falls
                // back to in-memory when no cache client is available.
                match &app_state.cache {
                    Some(client) => {
                        let store = Arc::new(crate::security::redis_blacklist_store::RedisBlacklistStore::new(Arc::clone(client)));
                        info!("JWT Manager using Redis-backed token blacklist");
                        manager.with_blacklist_store(store)
                    }
                    None => {
                        info!("JWT Manager using in-memory token blacklist (no cache client)");
                        manager
                    }
                }
            };

            info!("JWT Manager initialized successfully");
            Some(Arc::new(manager))
        }
        Err(e) => {
            error!("Failed to initialize JWT Manager: {e}");
            None
        }
    };

    let rbac_config = RbacConfig::default();
    let rbac_manager = Arc::new(RbacManager::new(rbac_config));

    let default_permissions = build_default_route_permissions();
    let default_permission_count = default_permissions.len();
    rbac_manager.register_routes(default_permissions).await;
    info!(
        "RBAC Manager initialized with {} default route permissions",
        default_permission_count
    );

    let auth_provider_registry = {
        let mut builder = AuthProviderBuilder::new()
            .with_api_key_provider(Arc::new(ApiKeyAuthProvider::new()))
            .with_auth_config(Arc::clone(&auth_config));

        if let Some(ref manager) = jwt_manager {
            builder = builder.with_jwt_manager(Arc::clone(manager));
        }

        // Cloud login JWTs (handle_login/signup, minted with the persisted
        // saas_jwt_secret) carry claim-light payloads (sub/email/org/branch,
        // no iss/aud). The generic local-jwt provider rejects them, which used
        // to degrade every cloud-authenticated suite request to anonymous.
        builder = builder.with_provider(Arc::new(
            crate::security::saas_jwt_auth::SaasJwtAuthProvider::new(jwt_secret.clone()),
        ));

        let zitadel_configured = std::env::var("ZITADEL_ISSUER_URL").is_ok()
            && std::env::var("ZITADEL_CLIENT_ID").is_ok();

        if zitadel_configured {
            info!("Zitadel environment variables detected - external IdP authentication available");
        }

        Arc::new(builder.build().await)
    };

    info!(
        "Auth provider registry initialized with {} providers",
        auth_provider_registry.provider_count().await
    );

    let auth_middleware_state = AuthMiddlewareState::new(
        Arc::clone(&auth_config),
        Arc::clone(&auth_provider_registry),
    );

    SecurityComponents {
        cors,
        auth_config,
        jwt_manager,
        rbac_manager,
        auth_provider_registry,
        auth_middleware_state,
    }
}
