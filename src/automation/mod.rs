use crate::basic::ScriptService;
use crate::shared::models::{Automation, TriggerKind};
use crate::shared::state::AppState;
use chrono::{DateTime, Datelike, Timelike, Utc};
use diesel::prelude::*;
use log::{error, info, trace, warn};
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
        trace!("Creating AutomationService with scripts_dir='{}'", scripts_dir);
        Self {
            state,
            scripts_dir: scripts_dir.to_string(),
        }
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        trace!("Spawning AutomationService background task");
        let service = Arc::new(self);
        tokio::task::spawn_local({
            let service = service.clone();
            async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                let mut last_check = Utc::now();
                loop {
                    interval.tick().await;
                    trace!("Automation cycle tick started; last_check={}", last_check);
                    if let Err(e) = service.run_cycle(&mut last_check).await {
                        error!("Automation cycle error: {}", e);
                    }
                    trace!("Automation cycle tick completed");
                }
            }
        })
    }

    async fn run_cycle(
        &self,
        last_check: &mut DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        trace!("Running automation cycle; last_check={}", last_check);
        let automations = self.load_active_automations().await?;
        trace!("Loaded {} active automations", automations.len());
        self.check_table_changes(&automations, *last_check).await;
        self.process_schedules(&automations).await;
        *last_check = Utc::now();
        trace!("Automation cycle finished; new last_check={}", last_check);
        Ok(())
    }

    async fn load_active_automations(&self) -> Result<Vec<Automation>, diesel::result::Error> {
        trace!("Loading active automations from database");
        use crate::shared::models::system_automations::dsl::*;
        let mut conn = self.state.conn.lock().unwrap();
        let result = system_automations
            .filter(is_active.eq(true))
            .load::<Automation>(&mut *conn);
        trace!("Database query for active automations completed");
        result.map_err(Into::into)
    }

    async fn check_table_changes(&self, automations: &[Automation], since: DateTime<Utc>) {
        trace!("Checking table changes since={}", since);
        for automation in automations {
            trace!(
                "Checking automation id={} kind={} target={:?}",
                automation.id,
                automation.kind,
                automation.target
            );

            let trigger_kind = match TriggerKind::from_i32(automation.kind) {
                Some(k) => k,
                None => {
                    trace!("Skipping automation {}: invalid TriggerKind", automation.id);
                    continue;
                }
            };

            if !matches!(
                trigger_kind,
                TriggerKind::TableUpdate | TriggerKind::TableInsert | TriggerKind::TableDelete
            ) {
                trace!(
                    "Skipping automation {}: trigger_kind {:?} not table-related",
                    automation.id,
                    trigger_kind
                );
                continue;
            }

            let table = match &automation.target {
                Some(t) => t,
                None => {
                    trace!("Skipping automation {}: no table target", automation.id);
                    continue;
                }
            };

            let column = match trigger_kind {
                TriggerKind::TableInsert => "created_at",
                _ => "updated_at",
            };
            trace!(
                "Building query for table='{}' column='{}' trigger_kind={:?}",
                table,
                column,
                trigger_kind
            );

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
                    trace!(
                        "Detected {} change(s) in table='{}'; triggering automation {}",
                        result.count,
                        table,
                        automation.id
                    );
                    drop(conn_guard);
                    self.execute_action(&automation.param).await;
                    self.update_last_triggered(automation.id).await;
                }
                Ok(result) => {
                    trace!(
                        "No changes detected for automation {} (count={})",
                        automation.id,
                        result.count
                    );
                }
                Err(e) => {
                    error!("Error checking changes for table '{}': {}", table, e);
                }
            }
        }
    }

    async fn process_schedules(&self, automations: &[Automation]) {
        let now = Utc::now();
        trace!(
            "Processing scheduled automations at UTC={}",
            now.format("%Y-%m-%d %H:%M:%S")
        );
        for automation in automations {
            if let Some(TriggerKind::Scheduled) = TriggerKind::from_i32(automation.kind) {
                trace!(
                    "Evaluating schedule pattern={:?} for automation {}",
                    automation.schedule,
                    automation.id
                );
                if let Some(pattern) = &automation.schedule {
                    if Self::should_run_cron(pattern, now.timestamp()) {
                        trace!(
                            "Pattern matched; executing automation {} param='{}'",
                            automation.id,
                            automation.param
                        );
                        self.execute_action(&automation.param).await;
                        self.update_last_triggered(automation.id).await;
                    } else {
                        trace!(
                            "Pattern did not match for automation {}",
                            automation.id
                        );
                    }
                }
            }
        }
    }

    async fn update_last_triggered(&self, automation_id: Uuid) {
        trace!("Updating last_triggered for automation_id={}", automation_id);
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
        } else {
            trace!("Successfully updated last_triggered for {}", automation_id);
        }
    }

    fn should_run_cron(pattern: &str, timestamp: i64) -> bool {
        trace!("Evaluating cron pattern='{}' at timestamp={}", pattern, timestamp);
        let parts: Vec<&str> = pattern.split_whitespace().collect();
        if parts.len() != 5 {
            trace!("Invalid cron pattern '{}'", pattern);
            return false;
        }
        let dt = match DateTime::<Utc>::from_timestamp(timestamp, 0) {
            Some(dt) => dt,
            None => {
                trace!("Invalid timestamp={}", timestamp);
                return false;
            }
        };
        let minute = dt.minute() as i32;
        let hour = dt.hour() as i32;
        let day = dt.day() as i32;
        let month = dt.month() as i32;
        let weekday = dt.weekday().num_days_from_monday() as i32;
        let match_result = [minute, hour, day, month, weekday]
            .iter()
            .enumerate()
            .all(|(i, &val)| Self::cron_part_matches(parts[i], val));
        trace!(
            "Cron pattern='{}' result={} at {}",
            pattern,
            match_result,
            dt
        );
        match_result
    }

    fn cron_part_matches(part: &str, value: i32) -> bool {
        trace!("Checking cron part '{}' against value={}", part, value);
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
        trace!("Starting execute_action with param='{}'", param);
        let bot_id_string = env::var("BOT_GUID").unwrap_or_else(|_| "default_bot".to_string());
        let bot_id = Uuid::parse_str(&bot_id_string).unwrap_or_else(|_| Uuid::new_v4());
        trace!("Resolved bot_id={} for param='{}'", bot_id, param);

        let redis_key = format!("job:running:{}:{}", bot_id, param);
        trace!("Redis key for job tracking: {}", redis_key);

        if let Some(redis_client) = &self.state.redis_client {
            match redis_client.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    trace!("Connected to Redis; checking if job '{}' is running", param);
                    let is_running: Result<bool, redis::RedisError> = redis::cmd("EXISTS")
                        .arg(&redis_key)
                        .query_async(&mut conn)
                        .await;

                    if let Ok(true) = is_running {
                        warn!(
                            "Job '{}' is already running for bot '{}'; skipping execution",
                            param, bot_id
                        );
                        return;
                    }

                    let _: Result<(), redis::RedisError> = redis::cmd("SETEX")
                        .arg(&redis_key)
                        .arg(300)
                        .arg("1")
                        .query_async(&mut conn)
                        .await;
                    trace!("Job '{}' marked as running in Redis", param);
                }
                Err(e) => {
                    warn!("Failed to connect to Redis for job tracking: {}", e);
                }
            }
        }

        let full_path = Path::new(&self.scripts_dir).join(param);
        trace!("Resolved full path: {}", full_path.display());

        let script_content = match tokio::fs::read_to_string(&full_path).await {
            Ok(content) => {
                trace!("Script '{}' read successfully", param);
                content
            }
            Err(e) => {
                error!("Failed to read script {}: {}", full_path.display(), e);
                self.cleanup_job_flag(&bot_id, param).await;
                return;
            }
        };

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
        trace!(
            "Created temporary UserSession id={} for bot_id={}",
            user_session.id,
            bot_id
        );

        let script_service = ScriptService::new(Arc::clone(&self.state), user_session);
        let ast = match script_service.compile(&script_content) {
            Ok(ast) => {
                trace!("Compilation successful for script '{}'", param);
                ast
            }
            Err(e) => {
                error!("Error compiling script '{}': {}", param, e);
                self.cleanup_job_flag(&bot_id, param).await;
                return;
            }
        };

        trace!("Running compiled script '{}'", param);
        match script_service.run(&ast) {
            Ok(_) => {
                info!("Script '{}' executed successfully", param);
            }
            Err(e) => {
                error!("Error executing script '{}': {}", param, e);
            }
        }

        trace!("Cleaning up Redis flag for job '{}'", param);
        self.cleanup_job_flag(&bot_id, param).await;
        trace!("Finished execute_action for '{}'", param);
    }

    async fn cleanup_job_flag(&self, bot_id: &Uuid, param: &str) {
        trace!(
            "Cleaning up Redis flag for bot_id={} param='{}'",
            bot_id,
            param
        );
        let redis_key = format!("job:running:{}:{}", bot_id, param);

        if let Some(redis_client) = &self.state.redis_client {
            match redis_client.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    let _: Result<(), redis::RedisError> = redis::cmd("DEL")
                        .arg(&redis_key)
                        .query_async(&mut conn)
                        .await;
                    trace!("Removed Redis key '{}'", redis_key);
                }
                Err(e) => {
                    warn!("Failed to connect to Redis for cleanup: {}", e);
                }
            }
        }
    }
}
