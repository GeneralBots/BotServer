use super::*;

impl ContextMiddlewareState {
    pub fn new(_db_pool: DbPool, jwt_secret: String) -> Self {
        Self {
            jwt_secret: Arc::new(jwt_secret),
            org_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            user_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
            cache_ttl_seconds: 300,
        }
    }

    pub fn with_cache_ttl(mut self, ttl_seconds: u64) -> Self {
        self.cache_ttl_seconds = ttl_seconds;
        self
    }

    async fn get_organization_context(&self, org_id: Uuid) -> Option<OrganizationContext> {
        {
            let cache = self.org_cache.read().await;
            if let Some(cached) = cache.get(&org_id) {
                let age = chrono::Utc::now()
                    .signed_duration_since(cached.cached_at)
                    .num_seconds() as u64;
                if age < self.cache_ttl_seconds {
                    return Some(cached.context.clone());
                }
            }
        }

        let context = OrganizationContext::new(org_id)
            .with_name("Organization".to_string())
            .with_plan("free".to_string());

        {
            let mut cache = self.org_cache.write().await;
            cache.insert(
                org_id,
                CachedOrganization {
                    context: context.clone(),
                    cached_at: chrono::Utc::now(),
                },
            );
        }

        Some(context)
    }

    async fn get_user_roles_groups(
        &self,
        user_id: Uuid,
        _org_id: Option<Uuid>,
    ) -> (Vec<String>, Vec<String>) {
        {
            let cache = self.user_cache.read().await;
            if let Some(cached) = cache.get(&user_id) {
                let age = chrono::Utc::now()
                    .signed_duration_since(cached.cached_at)
                    .num_seconds() as u64;
                if age < self.cache_ttl_seconds {
                    return (cached.roles.clone(), cached.groups.clone());
                }
            }
        }

        let roles = vec!["member".to_string()];
        let groups = Vec::new();

        {
            let mut cache = self.user_cache.write().await;
            cache.insert(
                user_id,
                CachedUserData {
                    roles: roles.clone(),
                    groups: groups.clone(),
                    cached_at: chrono::Utc::now(),
                },
            );
        }

        (roles, groups)
    }

    pub async fn clear_org_cache(&self, org_id: Uuid) {
        let mut cache = self.org_cache.write().await;
        cache.remove(&org_id);
    }

    pub async fn clear_user_cache(&self, user_id: Uuid) {
        let mut cache = self.user_cache.write().await;
        cache.remove(&user_id);
    }

    pub async fn clear_all_caches(&self) {
        {
            let mut cache = self.org_cache.write().await;
            cache.clear();
        }
        {
            let mut cache = self.user_cache.write().await;
            cache.clear();
        }
    }
}
pub async fn organization_context_middleware(
    State(state): State<Arc<ContextMiddlewareState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Derive the tenant scope authoritatively from the authenticated token:
    // the JWT `org_id` claim is server-minted at login and is the binding
    // source of truth (issue #736). The `X-Organization-Id` header may only
    // confirm the token scope — a client-supplied mismatch is rejected, never
    // trusted, so a tenant can never view another tenant's context by
    // spoofing the header.
    let claims_org = request
        .extensions()
        .get::<AuthenticatedUser>()
        .and_then(|u| u.token_claims.as_ref())
        .and_then(|c| c.org_id.as_ref())
        .and_then(|o| Uuid::parse_str(o).ok());

    let header_org = request.headers().get("X-Organization-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .or_else(|| {
            // Compatibility: some legacy callers send `org_id` as a query
            // parameter; treat it as a header-equivalent with the same rules.
            request.uri().query().and_then(|q| {
                q.split('&').find_map(|kv| {
                    let (k, v) = kv.split_once('=')?;
                    if k == "org_id" || k == "organization_id" {
                        Uuid::parse_str(v).ok()
                    } else {
                        None
                    }
                })
            })
        });

    let org_id = match (claims_org, header_org) {
        (Some(claims), Some(header)) if claims != header => {
            // The client requested a tenant different from the one the token
            // is bound to: deny, fail closed.
            return ForbiddenResponse::new(
                "Requested organization does not match the authenticated session",
            )
            .into_response();
        }
        (Some(claims), _) => Some(claims),
        // Anonymous callers get no tenant context at all — a client-supplied
        // header must never grant a tenant scope (issue #734).
        (None, Some(_)) => None,
        (None, None) => None,
    };

    if let Some(org_id) = org_id {
        if let Some(context) = state.get_organization_context(org_id).await {
            request.extensions_mut().insert(context);
        }
    }

    next.run(request).await
}
/// Extract and validate user authentication, adding context to extensions
pub async fn authentication_middleware(
    State(state): State<Arc<ContextMiddlewareState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let user = match extract_and_validate_user(&request, &state).await {
        Ok(user) => user,
        Err(_) => AuthenticatedUser::anonymous(),
    };

    // If authenticated, fetch roles and groups
    let user = if user.is_authenticated() {
        let org_id = user.organization_id.or_else(|| {
            request
                .extensions()
                .get::<OrganizationContext>()
                .map(|o| o.organization_id)
        });

        let (roles, groups) = state.get_user_roles_groups(user.user_id, org_id).await;
        user.with_roles(roles).with_groups(groups)
    } else {
        user
    };

    request.extensions_mut().insert(user);
    next.run(request).await
}
/// Combine organization and user context into RequestContext
pub async fn request_context_middleware(mut request: Request<Body>, next: Next) -> Response {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    let organization = request.extensions().get::<OrganizationContext>().cloned();

    let client_ip = extract_client_ip(&request);
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let bot_id = extract_bot_id(&request);
    let conversation_id = extract_conversation_id(&request);

    let mut context = RequestContext::new(user).with_client_info(client_ip, user_agent);

    if let Some(org) = organization {
        context = context.with_organization(org);
    }

    if let Some(bot_id) = bot_id {
        context = context.with_bot(bot_id);
    }

    if let Some(conv_id) = conversation_id {
        context = context.with_conversation(conv_id);
    }

    request.extensions_mut().insert(context);
    next.run(request).await
}
/// Require authentication - returns 401 if not authenticated
pub async fn require_authentication_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    if !user.is_authenticated() {
        return Err(UnauthorizedResponse::new("Authentication required").into_response());
    }

    Ok(next.run(request).await)
}
