use super::*;

impl Default for AuditActor {
    fn default() -> Self {
        Self {
            user_id: None,
            username: None,
            email: None,
            ip_address: None,
            user_agent: None,
            session_id: None,
            actor_type: ActorType::Anonymous,
        }
    }
}
impl AuditActor {
    pub fn user(user_id: Uuid) -> Self {
        Self {
            user_id: Some(user_id),
            actor_type: ActorType::User,
            ..Default::default()
        }
    }

    pub fn system() -> Self {
        Self {
            actor_type: ActorType::System,
            ..Default::default()
        }
    }

    pub fn service(name: &str) -> Self {
        Self {
            username: Some(name.to_string()),
            actor_type: ActorType::Service,
            ..Default::default()
        }
    }

    pub fn bot(bot_id: Uuid) -> Self {
        Self {
            user_id: Some(bot_id),
            actor_type: ActorType::Bot,
            ..Default::default()
        }
    }

    pub fn anonymous() -> Self {
        Self::default()
    }

    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_user_agent(mut self, ua: String) -> Self {
        self.user_agent = Some(ua);
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}
