use diesel::{OptionalExtension, RunQueryDsl};
use diesel::PgConnection;
use std::sync::Arc;

pub use botlib::db_pool::DbPool;

pub type GetDefaultBotFn = Arc<dyn Fn(&mut PgConnection) -> (uuid::Uuid, String) + Send + Sync>;

pub type SendEmailFn = Arc<dyn Fn(&str, &str, &str, uuid::Uuid, Option<&str>) -> Result<String, String> + Send + Sync>;

pub type SendWhatsAppFn = Arc<dyn Fn(uuid::Uuid, &str, &str, Option<&str>, Option<&str>) -> Result<String, String> + Send + Sync>;

pub type GetConfigFn = Arc<dyn Fn(&uuid::Uuid, &str, Option<&str>) -> Result<String, String> + Send + Sync>;

pub type LlmGenerateFn = Arc<dyn Fn(&str, &serde_json::Value, &str, &str) -> Result<String, String> + Send + Sync>;

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
    pub send_email: SendEmailFn,
    pub send_whatsapp: SendWhatsAppFn,
    pub get_config: GetConfigFn,
    pub llm_generate: LlmGenerateFn,
    /// Durable campaign sender worker (#731). None until explicitly started.
    pub worker: Option<Arc<crate::campaign::CampaignWorker>>,
}

impl AppState {
    pub fn new(
        conn: Arc<DbPool>,
        get_default_bot: GetDefaultBotFn,
        send_email: SendEmailFn,
        send_whatsapp: SendWhatsAppFn,
        get_config: GetConfigFn,
        llm_generate: LlmGenerateFn,
    ) -> Self {
        Self {
            conn,
            get_default_bot,
            send_email,
            send_whatsapp,
            get_config,
            llm_generate,
            worker: None,
        }
    }

    /// Attaches the durable campaign worker and starts its background loop.
    pub fn with_worker(mut self, worker: crate::campaign::CampaignWorker) -> Self {
        self.worker = Some(Arc::new(worker));
        if let Some(worker) = self.worker.as_ref() {
            worker.clone().start();
        }
        self
    }

    pub fn get_bot_context(&self) -> (uuid::Uuid, uuid::Uuid) {
        let (org_id, _, bot_id) = self.get_scope();
        (org_id, bot_id)
    }

    /// Resolves (org_id, branch_id, bot_id) for the default bot.
    /// org_id is the gborg tenant owning the workspace; branch_id is the
    /// workspace branch; they are never conflated.
    pub fn get_scope(&self) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid) {
        let Ok(mut conn) = self.conn.get() else {
            return (uuid::Uuid::nil(), uuid::Uuid::nil(), uuid::Uuid::nil());
        };
        let (bot_id, _name) = (self.get_default_bot)(&mut conn);
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            org_id: uuid::Uuid,
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            branch_id: uuid::Uuid,
        }
        let row: Option<Row> = diesel::sql_query(
            "SELECT org_id, branch_id FROM bots WHERE id = $1 LIMIT 1",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .get_result(&mut conn)
        .optional()
        .ok()
        .flatten();
        match row {
            Some(r) => (r.org_id, r.branch_id, bot_id),
            None => (uuid::Uuid::nil(), uuid::Uuid::nil(), bot_id),
        }
    }
}
