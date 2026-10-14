use crate::campaign::models::{
    Campaign, CampaignMetrics, CampaignStatus, ChannelMetrics,
    ContentStatus, Schedule,
};
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct CampaignExecution {
    pub campaign_id: String,
    pub scheduled_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
    pub status: ExecutionStatus,
    pub result: Option<ExecutionResult>,
    pub channels_executed: Vec<String>,
    pub content_published: usize,
}

#[derive(Debug, Clone)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Skipped,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub message: String,
    pub metrics_update: Option<CampaignMetrics>,
}

#[derive(Debug)]
pub struct CampaignScheduler {
    campaigns: Arc<RwLock<HashMap<String, Campaign>>>,
    executions: Arc<RwLock<Vec<CampaignExecution>>>,
}

impl Default for CampaignScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl CampaignScheduler {
    pub fn new() -> Self {
        Self {
            campaigns: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_campaign(&self, campaign: Campaign) -> Result<()> {
        let mut store = self.campaigns.write().await;
        store.insert(campaign.id.clone(), campaign);
        info!("Campaign registered in scheduler: {}", store.len());
        Ok(())
    }

    pub async fn unregister_campaign(&self, campaign_id: &str) -> Result<bool> {
        let mut store = self.campaigns.write().await;
        let removed = store.remove(campaign_id).is_some();
        if removed {
            info!("Campaign {} unregistered from scheduler", campaign_id);
        }
        Ok(removed)
    }

    pub async fn get_campaign(&self, campaign_id: &str) -> Option<Campaign> {
        let store = self.campaigns.read().await;
        store.get(campaign_id).cloned()
    }

    pub async fn list_campaigns(&self) -> Vec<Campaign> {
        let store = self.campaigns.read().await;
        store.values().cloned().collect()
    }

    pub async fn list_active_campaigns(&self) -> Vec<Campaign> {
        let store = self.campaigns.read().await;
        store
            .values()
            .filter(|c| c.is_active())
            .cloned()
            .collect()
    }

    pub async fn check_and_execute(&self) -> Result<Vec<CampaignExecution>> {
        let now = Utc::now();
        let mut executed = Vec::new();

        let active_campaigns = {
            let store = self.campaigns.read().await;
            store
                .values()
                .filter(|c| c.is_active())
                .cloned()
                .collect::<Vec<_>>()
        };

        for mut campaign in active_campaigns {
            let should_run = self.should_execute_now(&campaign.schedule, now);
            if !should_run {
                continue;
            }

            let execution = self.execute_campaign(&mut campaign, now).await;
            let campaign_id = campaign.id.clone();
            executed.push(execution);

            {
                let mut store = self.campaigns.write().await;
                if let Some(existing) = store.get_mut(&campaign_id) {
                    existing.metrics = campaign.metrics;
                    existing.updated_at = now.to_rfc3339();
                }
            }
        }

        let mut history = self.executions.write().await;
        history.extend(executed.clone());

        if history.len() > 1000 {
            let drain_end = history.len().saturating_sub(1000);
            history.drain(0..drain_end);
        }

        Ok(executed)
    }

    fn should_execute_now(&self, schedule: &Schedule, now: DateTime<Utc>) -> bool {
        if let Some(ref start) = schedule.start_date {
            if let Ok(start_dt) = DateTime::parse_from_rfc3339(start) {
                let start_utc = start_dt.with_timezone(&Utc);
                if now < start_utc {
                    return false;
                }
            }
        }

        if let Some(ref end) = schedule.end_date {
            if let Ok(end_dt) = DateTime::parse_from_rfc3339(end) {
                let end_utc = end_dt.with_timezone(&Utc);
                if now > end_utc {
                    return false;
                }
            }
        }

        let time_matches = schedule.time_slots.iter().any(|slot| {
            let parts: Vec<&str> = slot.split(':').collect();
            if parts.len() < 2 {
                return false;
            }
            let hour: u32 = parts[0].parse().unwrap_or(99);
            let minute: u32 = parts[1].parse().unwrap_or(99);
            now.hour() == hour && now.minute() == minute
        });

        if !time_matches && !schedule.time_slots.is_empty() {
            return false;
        }

        match schedule.frequency.to_lowercase().as_str() {
            "hourly" => true,
            "daily" => true,
            "weekly" => matches!(now.weekday(), Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri),
            "weekdays" => matches!(now.weekday(), Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri),
            "weekends" => matches!(now.weekday(), Weekday::Sat | Weekday::Sun),
            "monthly" => now.day() == 1,
            "once" => schedule.start_date.as_ref().is_some_and(|start| {
                DateTime::parse_from_rfc3339(start)
                    .map(|dt| {
                        let start_utc = dt.with_timezone(&Utc);
                        (now - start_utc).num_minutes().abs() < 1
                    })
                    .unwrap_or(false)
            }),
            _ => false,
        }
    }

    async fn execute_campaign(
        &self,
        campaign: &mut Campaign,
        now: DateTime<Utc>,
    ) -> CampaignExecution {
        let campaign_id = campaign.id.clone();
        let mut channels_executed = Vec::new();
        let mut content_published = 0;

        for channel in &campaign.channels {
            if !channel.enabled {
                continue;
            }

            let channel_pieces: Vec<_> = campaign
                .content
                .iter()
                .filter(|c| c.channel == channel.channel && matches!(c.status, ContentStatus::Approved | ContentStatus::Scheduled))
                .cloned()
                .collect();

            for piece in &channel_pieces {
                let mut updated = piece.clone();
                updated.set_status(ContentStatus::Published);

                if let Some(existing) = campaign
                    .content
                    .iter_mut()
                    .find(|c| c.id == piece.id)
                {
                    existing.set_status(ContentStatus::Published);
                }

                content_published += 1;
                channels_executed.push(channel.channel.clone());

                let channel_metrics = ChannelMetrics {
                    impressions: 0,
                    clicks: 0,
                    conversions: 0,
                    spend: 0.0,
                    revenue: None,
                };

                campaign
                    .metrics
                    .update_channel_metrics(channel.channel.clone(), channel_metrics);
            }
        }

        if content_published > 0 {
            campaign.set_status(CampaignStatus::Active);
        }

        CampaignExecution {
            campaign_id,
            scheduled_at: now,
            executed_at: Some(now),
            status: if content_published > 0 {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Skipped
            },
            result: Some(ExecutionResult {
                success: content_published > 0,
                message: format!(
                    "Published {} pieces across {} channels",
                    content_published,
                    channels_executed.len()
                ),
                metrics_update: Some(campaign.metrics.clone()),
            }),
            channels_executed,
            content_published,
        }
    }

    pub async fn get_execution_history(
        &self,
        campaign_id: Option<&str>,
        limit: usize,
    ) -> Vec<CampaignExecution> {
        let history = self.executions.read().await;
        let result: Vec<CampaignExecution> = if let Some(cid) = campaign_id {
            history
                .iter()
                .filter(|e| e.campaign_id == cid)
                .cloned()
                .collect()
        } else {
            history.clone()
        };
        result.into_iter().rev().take(limit).collect()
    }

    pub async fn execution_count(&self, campaign_id: &str) -> usize {
        let history = self.executions.read().await;
        history.iter().filter(|e| e.campaign_id == campaign_id).count()
    }

    pub async fn last_execution(&self, campaign_id: &str) -> Option<CampaignExecution> {
        let history = self.executions.read().await;
        history
            .iter().rfind(|e| e.campaign_id == campaign_id)
            .cloned()
    }

    pub async fn next_scheduled_time(&self, campaign_id: &str) -> Option<DateTime<Utc>> {
        let store = self.campaigns.read().await;
        let campaign = store.get(campaign_id)?;

        let now = Utc::now();
        let schedule = &campaign.schedule;

        if let Some(ref end) = schedule.end_date {
            if let Ok(end_dt) = DateTime::parse_from_rfc3339(end) {
                let end_utc = end_dt.with_timezone(&Utc);
                if now > end_utc {
                    return None;
                }
            }
        }

        let next_slot = schedule.time_slots.iter().find_map(|slot| {
            let parts: Vec<&str> = slot.split(':').collect();
            let hour: u32 = parts.first().and_then(|p| p.parse().ok())?;
            let minute: u32 = parts.get(1).and_then(|p| p.parse().ok())?;

            let candidate = now
                .with_hour(hour)
                .and_then(|d| d.with_minute(minute))
                .and_then(|d| d.with_second(0))?;

            if candidate > now {
                Some(candidate)
            } else {
                None
            }
        });

        next_slot.or_else(|| Some(now + Duration::hours(1)))
    }
}

pub struct SchedulerService {
    scheduler: Arc<CampaignScheduler>,
    running: Arc<RwLock<bool>>,
}

impl SchedulerService {
    pub fn new(scheduler: Arc<CampaignScheduler>) -> Self {
        Self {
            scheduler,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn scheduler(&self) -> &Arc<CampaignScheduler> {
        &self.scheduler
    }

    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                warn!("Scheduler already running");
                return Ok(());
            }
            *running = true;
        }

        let scheduler = Arc::clone(&self.scheduler);
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                {
                    let is_running = running.read().await;
                    if !*is_running {
                        info!("Scheduler stopped");
                        break;
                    }
                }

                match scheduler.check_and_execute().await {
                    Ok(executions) => {
                        for exec in &executions {
                            match exec.status {
                                ExecutionStatus::Completed => {
                                    info!(
                                        "Campaign {} executed: {} content published",
                                        exec.campaign_id, exec.content_published
                                    );
                                }
                                ExecutionStatus::Skipped => {
                                    info!("Campaign {} skipped (no content ready)", exec.campaign_id);
                                }
                                ExecutionStatus::Failed(ref e) => {
                                    error!("Campaign {} failed: {}", exec.campaign_id, e);
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        error!("Scheduler execution error: {}", e);
                    }
                }
            }
        });

        info!("Campaign scheduler started");
        Ok(())
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Campaign scheduler stop requested");
    }

    pub async fn is_running(&self) -> bool {
        let running = self.running.read().await;
        *running
    }
}
