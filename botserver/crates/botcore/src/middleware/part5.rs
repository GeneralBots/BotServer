use super::*;

/// Require specific role - returns 403 if role not present
pub fn require_role_middleware(
    required_role: &'static str,
) -> impl Fn(Request<Body>, Next) -> MiddlewareFuture + Clone + Send {
    move |request: Request<Body>, next: Next| {
        Box::pin(async move {
            let user = request
                .extensions()
                .get::<AuthenticatedUser>()
                .cloned()
                .unwrap_or_else(AuthenticatedUser::anonymous);

            if !user.has_role(required_role) {
                return Err(ResponseError::new(
                    ForbiddenResponse::new(&format!("Required role: {required_role}")).into_response(),
                ));
            }

            Ok(next.run(request).await)
        })
    }
}
/// Require organization context - returns 400 if no organization
pub async fn require_organization_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, ResponseError> {
    let has_org = request.extensions().get::<OrganizationContext>().is_some();

    if !has_org {
        return Err(ResponseError::new(
            BadRequestResponse::new("Organization context required").into_response(),
        ));
    }

    Ok(next.run(request).await)
}
/// Require admin role within organization
pub async fn require_org_admin_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, ResponseError> {
    let context = request.extensions().get::<RequestContext>().cloned();

    let is_admin = context
        .as_ref()
        .map(|c| c.user.is_admin || c.user.is_super_admin || c.has_org_permission("admin"))
        .unwrap_or(false);

    if !is_admin {
        return Err(ResponseError::new(
            ForbiddenResponse::new("Organization admin access required").into_response(),
        ));
    }

    Ok(next.run(request).await)
}

// ============================================================================
// Extractors
// ============================================================================
/// Extract bot ID from request
pub(super) fn extract_bot_id(request: &Request<Body>) -> Option<Uuid> {
    // Try header
    if let Some(header) = request.headers().get("X-Bot-Id") {
        if let Ok(s) = header.to_str() {
            if let Ok(id) = Uuid::parse_str(s) {
                return Some(id);
            }
        }
    }

    // Try path
    let path = request.uri().path();
    if path.contains("/bots/") {
        let parts: Vec<&str> = path.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "bots" && i + 1 < parts.len() {
                if let Ok(id) = Uuid::parse_str(parts[i + 1]) {
                    return Some(id);
                }
            }
        }
    }

    None
}
/// Extract conversation ID from request
pub(super) fn extract_conversation_id(request: &Request<Body>) -> Option<Uuid> {
    // Try header
    if let Some(header) = request.headers().get("X-Conversation-Id") {
        if let Ok(s) = header.to_str() {
            if let Ok(id) = Uuid::parse_str(s) {
                return Some(id);
            }
        }
    }

    // Try path
    let path = request.uri().path();
    if path.contains("/conversations/") {
        let parts: Vec<&str> = path.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "conversations" && i + 1 < parts.len() {
                if let Ok(id) = Uuid::parse_str(parts[i + 1]) {
                    return Some(id);
                }
            }
        }
    }

    None
}
/// Extract client IP from request headers
pub(super) fn extract_client_ip(request: &Request<Body>) -> Option<String> {
    // Try common proxy headers
    for header_name in &[
        "X-Forwarded-For",
        "X-Real-IP",
        "CF-Connecting-IP",
        "True-Client-IP",
    ] {
        if let Some(header) = request.headers().get(*header_name) {
            if let Ok(value) = header.to_str() {
                // X-Forwarded-For can contain multiple IPs
                let ip = value.split(',').next().map(|s| s.trim().to_string());
                if ip.is_some() {
                    return ip;
                }
            }
        }
    }

    None
}
/// Extract and validate user from authorization header
pub(super) async fn extract_and_validate_user(
    request: &Request<Body>,
    state: &ContextMiddlewareState,
) -> Result<AuthenticatedUser, AuthError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    let token = if let Some(stripped) = auth_header.strip_prefix("Bearer ") {
        stripped
    } else {
        return Err(AuthError::InvalidFormat);
    };

    // Validate JWT token
    let claims = validate_jwt(token, &state.jwt_secret)?;

    let user_id =
        Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken("Invalid user ID".to_string()))?;

    let user = AuthenticatedUser::new(user_id).with_email(claims.sub.clone());

    Ok(AuthenticatedUser {
        token_claims: Some(claims),
        ..user
    })
}
/// Validate JWT token and extract claims using jsonwebtoken crate
pub(super) fn validate_jwt(token: &str, secret: &str) -> Result<TokenClaims, AuthError> {
    // Configure validation rules
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_nbf = false;
    validation.set_required_spec_claims(&["sub", "exp"]);

    // Also accept RS256 tokens (common with OIDC providers like Zitadel)
    // Try HS256 first, then RS256 if that fails
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    match decode::<TokenClaims>(token, &decoding_key, &validation) {
        Ok(token_data) => Ok(token_data.claims),
        Err(e) => {
            // If HS256 fails, try decoding without signature verification
            // This handles cases where the token is from an external OIDC provider
            // and we just need to read the claims (signature already verified upstream)
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                    // Try RS256 with the secret as a PEM key
                    let mut rs_validation = Validation::new(Algorithm::RS256);
                    rs_validation.validate_exp = true;
                    rs_validation.validate_nbf = false;
                    rs_validation.set_required_spec_claims(&["sub", "exp"]);

                    // If secret looks like a PEM key, try to decode with it
                    if secret.contains("-----BEGIN") {
                        if let Ok(rsa_key) = DecodingKey::from_rsa_pem(secret.as_bytes()) {
                            if let Ok(token_data) = decode::<TokenClaims>(token, &rsa_key, &rs_validation) {
                                return Ok(token_data.claims);
                            }
                        }
                    }

                    // Fallback: decode without validation for trusted internal tokens
                    Err(AuthError::InvalidToken(format!("Invalid signature: {}", e)))
                }
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    Err(AuthError::TokenExpired)
                }
                jsonwebtoken::errors::ErrorKind::InvalidToken => {
                    Err(AuthError::InvalidToken("Malformed token".to_string()))
                }
                jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                    Err(AuthError::InvalidToken("Invalid issuer".to_string()))
                }
                jsonwebtoken::errors::ErrorKind::InvalidAudience => {
                    Err(AuthError::InvalidToken("Invalid audience".to_string()))
                }
                jsonwebtoken::errors::ErrorKind::InvalidSubject => {
                    Err(AuthError::InvalidToken("Invalid subject".to_string()))
                }
                jsonwebtoken::errors::ErrorKind::MissingRequiredClaim(claim) => {
                    Err(AuthError::InvalidToken(format!("Missing required claim: {}", claim)))
                }
                _ => {
                    Err(AuthError::InvalidToken(format!("Token validation failed: {}", e)))
                }
            }
        }
    }
}
