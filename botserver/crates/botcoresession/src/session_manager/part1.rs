use super::*;

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionManager")
            .field("pool", &"Pool<PgConnection>")
            .field("sessions", &self.sessions)
            .field("waiting_for_input", &self.waiting_for_input)
            .field("redis", &self.redis.is_some())
            .finish()
    }
}
