use super::*;

impl SessionManager {
    pub fn new(
        pool: Pool<ConnectionManager<PgConnection>>,
        #[cfg(feature = "cache")]
        redis_client: Option<Arc<Client>>,
    ) -> Self {
        let master_key = load_master_encryption_key();
        Self {
            pool,
            master_key,
            sessions: HashMap::new(),
            waiting_for_input: HashSet::new(),
            branch_id: Uuid::nil(),
            #[cfg(feature = "cache")]
            redis: redis_client,
        }
    }

    pub fn with_key(
        pool: Pool<ConnectionManager<PgConnection>>,
        master_key: Vec<u8>,
        #[cfg(feature = "cache")]
        redis_client: Option<Arc<Client>>,
    ) -> Self {
        Self {
            pool,
            master_key,
            sessions: HashMap::new(),
            waiting_for_input: HashSet::new(),
            branch_id: Uuid::nil(),
            #[cfg(feature = "cache")]
            redis: redis_client,
        }
    }

    pub fn provide_input(
        &mut self,
        session_id: Uuid,
        input: String,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        trace!("SessionManager.provide_input called for session {}", session_id);
        let sess = if let Some(existing) = self.sessions.get(&session_id) {
            let mut sess = existing.clone();
            sess.data = input;
            sess
        } else {
            SessionData {
                id: session_id,
                user_id: None,
                data: input,
            }
        };
        self.sessions.insert(session_id, sess);
        self.waiting_for_input.remove(&session_id);
        Ok(Some("user_input".to_string()))
    }

    pub fn mark_waiting(&mut self, session_id: Uuid) {
        self.waiting_for_input.insert(session_id);
    }

    pub fn get_session_by_id(
        &mut self,
        session_id: Uuid,
    ) -> Result<Option<UserSession>, Box<dyn Error + Send + Sync>> {
        let mut conn = self.pool.get()?;
        let result = user_sessions::table
            .filter(user_sessions::id.eq(session_id))
            .first::<UserSession>(&mut conn)
            .optional()?;
        Ok(result)
    }

    pub fn get_user_session(
        &mut self,
        uid: Uuid,
        bid: Uuid,
    ) -> Result<Option<UserSession>, Box<dyn Error + Send + Sync>> {
        let mut conn = self.pool.get()?;
        let result = user_sessions::table
            .filter(user_sessions::user_id.eq(uid.to_string()))
            .filter(user_sessions::bot_id.eq(bid))
            .order(user_sessions::created_at.desc())
            .first::<UserSession>(&mut conn)
            .optional()?;
        Ok(result)
    }

    pub fn get_or_create_user_session(
        &mut self,
        uid: Uuid,
        bid: Uuid,
        session_title: &str,
    ) -> Result<Option<UserSession>, Box<dyn Error + Send + Sync>> {
        if let Some(existing) = self.get_user_session(uid, bid)? {
            return Ok(Some(existing));
        }
        self.create_session(uid, bid, session_title).map(Some)
    }

    pub fn get_or_create_anonymous_user(
        &mut self,
        uid: Option<Uuid>,
    ) -> Result<Uuid, Box<dyn Error + Send + Sync>> {
        let mut conn = self.pool.get()?;
        let user_id = uid.unwrap_or_else(Uuid::new_v4);
        let user_exists: Option<Uuid> = users::table
            .filter(users::id.eq(user_id))
            .select(users::id)
            .first(&mut conn)
            .optional()?;
        if user_exists.is_none() {
            let now = Utc::now();
            diesel::insert_into(users::table)
                .values((
                    users::id.eq(user_id),
                    users::username.eq(format!("guest_{}", &user_id.to_string()[..8])),
                    users::email.eq(format!(
                        "guest_{}@anonymous.local",
                        &user_id.to_string()[..8]
                    )),
                    users::password_hash.eq(""),
                    users::is_active.eq(true),
                    users::created_at.eq(now),
                    users::updated_at.eq(now),
                ))
                .execute(&mut conn)?;
        }
        Ok(user_id)
    }

    pub fn create_session(
        &mut self,
        uid: Uuid,
        bid: Uuid,
        session_title: &str,
    ) -> Result<UserSession, Box<dyn Error + Send + Sync>> {
        self.create_session_with_id(Uuid::new_v4(), uid, bid, session_title)
    }

    pub fn create_session_with_id(
        &mut self,
        session_id: Uuid,
        uid: Uuid,
        bid: Uuid,
        _session_title: &str,
    ) -> Result<UserSession, Box<dyn Error + Send + Sync>> {
        let verified_uid = self.get_or_create_anonymous_user(Some(uid))?;
        let mut conn = self.pool.get()?;
        let now = Utc::now();
        let inserted: UserSession = diesel::insert_into(user_sessions::table)
            .values((
                user_sessions::id.eq(session_id),
                user_sessions::branch_id.eq(self.branch_id),
                user_sessions::bot_id.eq(bid),
                user_sessions::session_id.eq(session_id.to_string()),
                user_sessions::user_id.eq(verified_uid.to_string()),
                user_sessions::created_at.eq(now),
                user_sessions::updated_at.eq(now),
            ))
            .returning(UserSession::as_returning())
            .get_result(&mut conn)
            .map_err(|e| {
                log::error!("Failed to create session in database: {}", e);
                e
            })?;

        log::info!("User {} created resource: session {}", verified_uid, inserted.id);
        Ok(inserted)
    }

    pub fn get_or_create_session_by_id(
        &mut self,
        session_id: Uuid,
        uid: Uuid,
        bid: Uuid,
        session_title: &str,
    ) -> Result<UserSession, Box<dyn Error + Send + Sync>> {
        if let Ok(Some(existing)) = self.get_session_by_id(session_id) {
            return Ok(existing);
        }
        self.create_session_with_id(session_id, uid, bid, session_title)
    }

    pub fn save_message(
        &mut self,
        sess_id: Uuid,
        uid: Uuid,
        ro: i32,
        content: &str,
        msg_type: i32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut conn = self.pool.get()?;
        let next_index: i32 = message_history::table
            .filter(message_history::session_id.eq(sess_id))
            .count()
            .get_result::<i64>(&mut conn)
            .unwrap_or(0) as i32;

        let bot_id: Uuid = user_sessions::table
            .filter(user_sessions::id.eq(sess_id))
            .select(user_sessions::bot_id)
            .first(&mut conn)?;
        let key_bytes = derive_scope_key(&self.master_key, "message", &bot_id);
        let encrypted_content = encrypt_field(content, &key_bytes)
            .map_err(|e| format!("Encryption failed: {e}"))?;

        diesel::insert_into(message_history::table)
            .values((
                message_history::id.eq(Uuid::new_v4()),
                message_history::session_id.eq(sess_id),
                message_history::user_id.eq(uid),
                message_history::role.eq(ro),
                message_history::content_encrypted.eq(encrypted_content),
                message_history::message_type.eq(msg_type),
                message_history::message_index.eq(next_index),
                message_history::created_at.eq(chrono::Utc::now()),
            ))
            .execute(&mut conn)?;
        trace!("Message saved for session {} with index {}", sess_id, next_index);
        Ok(())
    }

    pub fn update_session_context(
        &mut self,
        session_id: &Uuid,
        user_id: &Uuid,
        context_data: String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        #[cfg(feature = "cache")]
        {
            use redis::Commands;
            // TODO(#477): Pass org when available
            let redis_key = botlib::key_utils::build_key("", &["context", &user_id.to_string(), &session_id.to_string()]);
            if let Some(redis_client) = &self.redis {
                let mut conn = redis_client.get_connection()?;
                conn.set::<_, _, ()>(&redis_key, &context_data)?;
            } else {
                warn!("No Redis client configured, context not persisted");
            }
        }
        Ok(())
    }

    pub fn get_session_context_data(
        &self,
        session_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        #[cfg(feature = "cache")]
        {
            use redis::Commands;
            // TODO(#477): Pass org when available
            let base_key = botlib::key_utils::build_key("", &["context", &user_id.to_string(), &session_id.to_string()]);
            if let Some(redis_client) = &self.redis {
                let conn_option = redis_client
                    .get_connection()
                    .map_err(|e| {
                        warn!("Failed to get Cache connection: {}", e);
                        e
                    })
                    .ok();

                if let Some(mut connection) = conn_option {
                    match connection.get::<_, Option<String>>(&base_key) {
                        Ok(Some(context_name)) => {
                            // TODO(#477): Pass org when available
                            let full_key = botlib::key_utils::build_key("", &["context", &user_id.to_string(), &session_id.to_string(), &context_name]);
                            match connection.get::<_, Option<String>>(&full_key) {
                                Ok(Some(context_value)) => {
                                    trace!("Retrieved context value from Cache for key {}: {} chars", full_key, context_value.len());
                                    return Ok(context_value);
                                }
                                Ok(None) => {
                                    trace!("No context value found for key: {}", full_key);
                                }
                                Err(e) => {
                                    warn!("Failed to retrieve context value from Cache: {}", e);
                                }
                            }
                        }
                        Ok(None) => {
                            trace!("No context name found for key: {}", base_key);
                        }
                        Err(e) => {
                            warn!("Failed to retrieve context name from Cache: {}", e);
                        }
                    }
                }
            }
        }
        Ok(String::new())
    }
}
