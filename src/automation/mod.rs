use crate::basic::ScriptService;
use crate::shared::models::{Automation, TriggerKind};
use crate::shared::state::AppState;
use chrono::{DateTime, Datelike, Timelike, Utc};
use diesel::prelude::*;
use log::{error, info, warn};
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::time::Duration;
use uuid::Uuid;

pub struct AutomationService {
    state: Arc<AppState>,
    scripts_dir: String,
}

impl AutomationService {
    pub fn new(state: Arc<AppState>, scripts_dir: &str) -> Self {
        Self {
            state,
            scripts_dir: scripts_dir.to_string(),
        }
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        let service = Arc::new(self);
        tokio::task::spawn_local({
            let service = service.clone();
            async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                let mut last_check = Utc::now();
                loop {
                    interval.tick().await;
                    if let Err(e) = service.run_cycle(&mut last_check).await {
                        error!("Automation cycle error: {}", e);
                    }
                }
            }
        })
    }

    async fn run_cycle(
        &self,
        last_check: &mut DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let automations = self.load_active_automations().await?;
        self.check_table_changes(&automations, *last_check).await;
        self.process_schedules(&automations).await;
        *last_check = Utc::now();
        Ok(())
    }

    async fn load_active_automations(&self) -> Result<Vec<Automation>, diesel::result::Error> {
        use crate::shared::models::system_automations::dsl::*;
        let mut conn = self.state.conn.lock().unwrap();
        system_automations
            .filter(is_active.eq(true))
            .load::<Automation>(&mut *conn)
            .map_err(Into::into)
    }

    async fn check_table_changes(&self, automations: &[Automation], since: DateTime<Utc>) {
        for automation in automations {
            let trigger_kind = match crate::shared::models::TriggerKind::from_i32(automation.kind) {
                Some(k) => k,
                None => continue,
            };

            if !matches!(
                trigger_kind,
                TriggerKind::TableUpdate | TriggerKind::TableInsert | TriggerKind::TableDelete
            ) {
                continue;
            }

            let table = match &automation.target {
                Some(t) => t,
                None => continue,
            };

            let column = match trigger_kind {
                TriggerKind::TableInsert => "created_at",
                _ => "updated_at",
            };

            let query = format!(
                "SELECT COUNT(*) as count FROM {} WHERE {} > $1",
                table, column
            );

            let mut conn_guard = self.state.conn.lock().unwrap();
            let conn = &mut *conn_guard;

            #[derive(diesel::QueryableByName)]
            struct CountResult {
                #[diesel(sql_type = diesel::sql_types::BigInt)]
                count: i64,
            }

            let count_result = diesel::sql_query(&query)
                .bind::<diesel::sql_types::Timestamp, _>(since.naive_utc())
                .get_result::<CountResult>(conn);

            match count_result {
                Ok(result) if result.count > 0 => {
                    drop(conn_guard);
                    self.execute_action(&automation.param).await;
                    self.update_last_triggered(automation.id).await;
                }
                Ok(_result) => {}
                Err(e) => {
                    error!("Error checking changes for table '{}': {}", table, e);
                }
            }
        }
    }

    async fn process_schedules(&self, automations: &[Automation]) {
        let now = Utc::now();
        for automation in automations {
            if let Some(TriggerKind::Scheduled) = TriggerKind::from_i32(automation.kind) {
                if let Some(pattern) = &automation.schedule {
                    if Self::should_run_cron(pattern, now.timestamp()) {
                        self.execute_action(&automation.param).await;
                        self.update_last_triggered(automation.id).await;
                    }
                }
            }
        }
    }

    async fn update_last_triggered(&self, automation_id: Uuid) {
        use crate::shared::models::system_automations::dsl::*;
        let mut conn = self.state.conn.lock().unwrap();
        let now = Utc::now();
        if let Err(e) = diesel::update(system_automations.filter(id.eq(automation_id)))
            .set(last_triggered.eq(now.naive_utc()))
            .execute(&mut *conn)
        {
            error!(
                "Failed to update last_triggered for automation {}: {}",
                automation_id, e
            );
        }
    }

    fn should_run_cron(pattern: &str, timestamp: i64) -> bool {
        let parts: Vec<&str> = pattern.split_whitespace().collect();
        if parts.len() != 5 {
            return false;
        }
        let dt = match DateTime::<Utc>::from_timestamp(timestamp, 0) {
            Some(dt) => dt,
            None => return false,
        };
        let minute = dt.minute() as i32;
        let hour = dt.hour() as i32;
        let day = dt.day() as i32;
        let month = dt.month() as i32;
        let weekday = dt.weekday().num_days_from_monday() as i32;
        [minute, hour, day, month, weekday]
            .iter()
            .enumerate()
            .all(|(i, &val)| Self::cron_part_matches(parts[i], val))
    }

    fn cron_part_matches(part: &str, value: i32) -> bool {
        if part == "*" {
            return true;
        }
        if part.contains('/') {
            let parts: Vec<&str> = part.split('/').collect();
            if parts.len() != 2 {
                return false;
            }
            let step: i32 = parts[1].parse().unwrap_or(1);
            if parts[0] == "*" {
                return value % step == 0;
            }
        }
        part.parse::<i32>().map_or(false, |num| num == value)
    }

    async fn execute_action(&self, param: &str) {
        // Get bot_id early to use in Redis key
        let bot_id_string = env::var("BOT_GUID").unwrap_or_else(|_| "default_bot".to_string());
        let bot_id = Uuid::parse_str(&bot_id_string).unwrap_or_else(|_| Uuid::new_v4());

        // Check if this job is already running for this bot
        let is_bas_file = param.ends_with(".bas");
        let redis_key = format!("job:running:{}:{}", bot_id, param);

        // Try to check if job is running using Redis
        if let Some(redis_client) = &self.state.redis_client {
            match redis_client.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    // Check if key exists
                    let is_running: Result<bool, redis::RedisError> = redis::cmd("EXISTS")
                        .arg(&redis_key)
                        .query_async(&mut conn)
                        .await;

                    if let Ok(true) = is_running {
                        if is_bas_file {
                            warn!(
                                "⚠️  Job '{}' is already running for bot '{}', skipping execution to allow only one .bas execution per bot",
                                param, bot_id
                            );
                        } else {
                            info!(
                                "Job '{}' is already running for bot '{}', skipping execution",
                                param, bot_id
                            );
                        }
                        return;
                    }

                    // Mark job as running (set with 300 second expiry as safety)
                    let _: Result<(), redis::RedisError> = redis::cmd("SETEX")
                        .arg(&redis_key)
                        .arg(300) // 5 minutes expiry
                        .arg("1")
                        .query_async(&mut conn)
                        .await;
                }
                Err(e) => {
                    warn!("Failed to connect to Redis for job tracking: {}", e);
                }
            }
        }

        let full_path = Path::new(&self.scripts_dir).join(param);
        let script_content = match tokio::fs::read_to_string(&full_path).await {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to read script {}: {}", full_path.display(), e);
                // Clean up running flag on error
                self.cleanup_job_flag(&bot_id, param).await;
                return;
            }
        };
        info!("Executing action with param: {} for bot: {}", param, bot_id);
        let user_session = crate::shared::models::UserSession {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            bot_id,
            title: "Automation".to_string(),
            answer_mode: 0,
            current_tool: None,
            context_data: serde_json::Value::Null,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let script_service = ScriptService::new(Arc::clone(&self.state), user_session);
        let ast = match script_service.compile(&script_content) {
            Ok(ast) => ast,
            Err(e) => {
                error!("Error compiling script: {}", e);
                return;
            }
        };

        match script_service.run(&ast) {
            Ok(_result) => {
                info!("Script executed successfully");
            }
            Err(e) => {
                error!("Error executing script: {}", e);
            }
        }

        // Clean up running flag after execution
        self.cleanup_job_flag(&bot_id, param).await;
    }

    async fn cleanup_job_flag(&self, bot_id: &Uuid, param: &str) {
        let redis_key = format!("job:running:{}:{}", bot_id, param);

        if let Some(redis_client) = &self.state.redis_client {
            match redis_client.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    let _: Result<(), redis::RedisError> = redis::cmd("DEL")
                        .arg(&redis_key)
                        .query_async(&mut conn)
                        .await;
                }
                Err(e) => {
                    warn!("Failed to connect to Redis for cleanup: {}", e);
                }
            }
        }
    }
}
