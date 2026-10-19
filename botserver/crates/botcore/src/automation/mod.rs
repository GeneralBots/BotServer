// ScriptService stays in botserver layer
// use crate::basic::ScriptService;
use crate::shared::models::{Automation, TriggerKind};
use crate::shared::utils::current_org_id;
use log::{error, trace};
use tokio::time::{interval, Duration};

/// Normalizes a cron schedule by converting 6-field (with seconds) to 5-field format.
/// If the schedule already has 5 fields or is in natural language format, it's returned as-is.
fn normalize_cron_schedule(schedule: &str) -> String {
    let trimmed = schedule.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    let result = match parts.len() {
        // 6 fields: assume seconds format, remove seconds
        6 => parts[1..].join(" "),
        // 4 fields: missing day-of-week, add "*"
        4 => format!("{} *", parts.join(" ")),
        // 5 fields: standard format
        5 => parts.join(" "),
        // Invalid: return as-is and let cron parser handle the error
        _ => trimmed.to_string(),
    };
    
    result.trim().to_string()
}



#[derive(Debug)]
pub struct AutomationService {
    state: Arc<AppState>,
}
impl AutomationService {
    #[must_use]
    pub fn new(state: Arc<AppState>) -> Self {
        // episodic_memory scheduler started from botserver layer (botllm depends on botcore)
        Self { state }
    }
    pub async fn spawn(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut ticker = interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            if let Err(e) = self.check_scheduled_tasks().await {
                log::error!("Error checking scheduled tasks: {}", e);
            }
        }
    }
    pub async fn check_scheduled_tasks(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::shared::schema::system_automations::dsl::{
            id, is_active, kind, last_triggered as lt_column, system_automations,
        };
        let mut conn = self
            .state
            .conn
            .get()
            .map_err(|e| format!("Failed to acquire database connection: {}", e))?;
        let automations: Vec<Automation> = system_automations
            .filter(is_active.eq(true))
            .filter(kind.eq(TriggerKind::Scheduled as i32))
            .load::<Automation>(&mut conn)?;
        for automation in automations {
            if let Some(schedule_str) = &automation.schedule {
                let normalized_schedule = normalize_cron_schedule(schedule_str);
                trace!("Parsing schedule: original='{}', normalized='{}'", schedule_str, normalized_schedule);
                match Schedule::from_str(&normalized_schedule) {
                    Ok(parsed_schedule) => {
                        let now = Utc::now();
                        let next_run = parsed_schedule.upcoming(Utc).next();
                        if let Some(next_time) = next_run {
                            let time_until_next = next_time - now;
                            if time_until_next.num_minutes() < 1 {
                                if let Some(last_triggered) = automation.last_triggered {
                                    if (now - last_triggered).num_minutes() < 1 {
                                        continue;
                                    }
                                }
                                if let Err(e) = self.execute_automation(&automation).await {
                                    log::error!("Error executing automation {}: {}", automation.id, e);
                                }
                                if let Err(e) =
                                    diesel::update(system_automations.filter(id.eq(automation.id)))
                                        .set(lt_column.eq(Some(now)))
                                        .execute(&mut conn)
                                {
                                    error!(
                                        "Error updating last_triggered for automation {}: {}",
                                        automation.id, e
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        trace!(
                            "Skipping automation {} with invalid schedule (original: {}, normalized: {}): {}",
                            automation.id, schedule_str, normalized_schedule, e
                        );
                    }
                }
            }
        }
        Ok(())
    }
    async fn execute_automation(
        &self,
        automation: &Automation,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bot_id = automation.bot_id;
        let bot_name: String = {
            use crate::shared::models::schema::bots::dsl::*;
            let mut conn = self
                .state
                .conn
                .get()
                .map_err(|e| format!("Failed to acquire database connection: {}", e))?;
            bots.filter(id.eq(bot_id))
                .select(name)
                .first(&mut conn)?
        };
        let script_path = format!(
            "./work/{org_id}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/{param}.ast",
            org_id = current_org_id(), param = automation.param
        );
        let script_content = match tokio::fs::read_to_string(&script_path).await {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to read script {}: {}", script_path, e);
                return Ok(());
            }
        };
        let session = {
            let mut sm = self.state.session_manager.lock().await;
            sm.get_or_create_user_session(bot_id, bot_id, "Automation")?
                .ok_or("Failed to create session")?
        };
        // ScriptService stays in botserver — use ScriptRunner trait
        if let Some(runner) = &self.state.script_runner {
            match runner.run_script(&script_content, session.id, &bot_id.to_string()).await {
                Ok(_) => {}
                Err(e) => {
                    log::error!("Script execution failed: {}", e);
                }
            }
        } else {
            log::error!("No script runner available for automation {}", automation.id);
        }
        Ok(())
    }
}

use crate::shared::state::AppState;
use chrono::Utc;
use cron::Schedule;
use diesel::prelude::*;
use std::str::FromStr;
use std::sync::Arc;// incremental test
