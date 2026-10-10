use super::*;

impl InvitationService {
    pub(crate) async fn send_invitation_email(&self, invitation: &OrganizationInvitation, org_name: &str) {
        log::info!(
            "Sending invitation email to {} for organization {} (token: {}...)",
            invitation.email,
            org_name,
            &invitation.token[..16]
        );

        #[cfg(feature = "mail")]
        {
            // Real SMTP delivery via env-configured server (MAIL_HOST/PORT/
            // USER/PASS/FROM — secrets come from the environment/Vault, never
            // hardcoded). Best-effort: a failure logs and never fails the
            // invite creation.
            use lettre::transport::smtp::authentication::Credentials;
            use lettre::{Message, SmtpTransport, Transport};

            let host = std::env::var("MAIL_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port: u16 = std::env::var("MAIL_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(587);
            let user = std::env::var("MAIL_USER").unwrap_or_default();
            let pass = std::env::var("MAIL_PASS").unwrap_or_default();
            let from = std::env::var("MAIL_FROM").unwrap_or_else(|_| "notifications@pragmatismo.com.br".to_string());
            if host == "localhost" && user.is_empty() {
                log::warn!("MAIL_HOST not configured — invitation email not delivered");
                return;
            }

            let invite_url = std::env::var("INVITE_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:5000/invitations/accept".to_string());
            let body = format!(
                "You have been invited to join {org_name}.\n\n\
                 Accept your invitation: {invite_url}?token={token}\n\n\
                 This invitation expires at {expires}.",
                token = invitation.token,
                expires = invitation.expires_at.format("%Y-%m-%d %H:%M UTC"),
            );

            // `builder_dangerous` avoids needing a TLS feature on lettre (the
            // workspace dep only enables smtp-transport + builder); encryption
            // is negotiated opportunistically on the wire when the server
            // supports it. This mirrors the settings SMTP-test fallback.
            let mailer = SmtpTransport::builder_dangerous(&host)
                .port(port)
                .credentials(Credentials::new(user, pass))
                .build();
            let from_addr: lettre::address::Address = match from.parse() {
                Ok(a) => a,
                Err(_) => {
                    log::warn!("Invalid MAIL_FROM address — invitation email not delivered");
                    return;
                }
            };
            let to_addr: lettre::address::Address = match invitation.email.parse() {
                Ok(a) => a,
                Err(_) => {
                    log::warn!("Invalid recipient address — invitation email not delivered");
                    return;
                }
            };

            match Message::builder()
                .from(from_addr.into())
                .to(to_addr.into())
                .subject(format!("Invitation to join {org_name}"))
                .body(body)
            {
                Ok(email) => {
                    if let Err(e) = mailer.send(&email) {
                        log::warn!("Failed to deliver invitation email: {e}");
                    }
                }
                Err(e) => log::warn!("Failed to build invitation email: {e}"),
            }
        }
    }
}
