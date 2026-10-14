use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationAnalytics {
    pub total_conversations: u64,
    pub avg_duration_seconds: f64,
    pub avg_messages_per_conversation: f64,
    pub csat_score: Option<f64>,
    pub resolution_rate: f64,
    pub top_intents: Vec<IntentCount>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentCount {
    pub intent: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelBreakdown {
    pub channel: String,
    pub total_conversations: u64,
    pub total_messages: u64,
    pub avg_duration_seconds: f64,
    pub csat_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotAnalytics {
    pub bot_id: Uuid,
    pub bot_name: String,
    pub channel_breakdown: Vec<ChannelBreakdown>,
    pub total_conversations: u64,
    pub total_messages: u64,
    pub csat_score: Option<f64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub channel: String,
    pub user_id: Uuid,
    pub message_count: u32,
    pub duration_seconds: f64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub csat_rating: Option<u32>,
    pub resolved: bool,
    pub messages: Vec<ConversationMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AnalyticsService {
    conversations: Vec<ConversationRecord>,
}

impl AnalyticsService {
    pub fn new() -> Self {
        Self {
            conversations: Vec::new(),
        }
    }

    pub fn record_conversation(&mut self, record: ConversationRecord) {
        self.conversations.push(record);
    }

    pub fn aggregate_by_date(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> ConversationAnalytics {
        let filtered: Vec<_> = self
            .conversations
            .iter()
            .filter(|c| c.started_at >= start && c.started_at <= end)
            .collect();

        let total = filtered.len() as u64;
        if total == 0 {
            return ConversationAnalytics {
                total_conversations: 0,
                avg_duration_seconds: 0.0,
                avg_messages_per_conversation: 0.0,
                csat_score: None,
                resolution_rate: 0.0,
                top_intents: Vec::new(),
                period_start: start,
                period_end: end,
            };
        }

        let total_duration: f64 = filtered.iter().map(|c| c.duration_seconds).sum();
        let total_messages: u64 = filtered.iter().map(|c| c.message_count as u64).sum();
        let resolved_count = filtered.iter().filter(|c| c.resolved).count() as f64;
        let ratings: Vec<u32> = filtered.iter().filter_map(|c| c.csat_rating).collect();
        let csat = if ratings.is_empty() {
            None
        } else {
            let sum: u32 = ratings.iter().sum();
            Some(sum as f64 / ratings.len() as f64)
        };

        let intents = self.extract_top_intents(&filtered, 10);

        ConversationAnalytics {
            total_conversations: total,
            avg_duration_seconds: total_duration / total as f64,
            avg_messages_per_conversation: total_messages as f64 / total as f64,
            csat_score: csat,
            resolution_rate: resolved_count / total as f64,
            top_intents: intents,
            period_start: start,
            period_end: end,
        }
    }

    pub fn by_channel(&self, channel: &str) -> Vec<&ConversationRecord> {
        self.conversations
            .iter()
            .filter(|c| c.channel == channel)
            .collect()
    }

    pub fn channel_breakdown(&self) -> Vec<ChannelBreakdown> {
        let mut channels: HashMap<String, Vec<&ConversationRecord>> = HashMap::new();
        for conv in &self.conversations {
            channels
                .entry(conv.channel.clone())
                .or_default()
                .push(conv);
        }

        channels
            .into_iter()
            .map(|(channel, records)| {
                let total = records.len() as u64;
                let total_msgs: u64 = records.iter().map(|r| r.message_count as u64).sum();
                let total_dur: f64 = records.iter().map(|r| r.duration_seconds).sum();
                let ratings: Vec<u32> = records.iter().filter_map(|r| r.csat_rating).collect();
                let csat = if ratings.is_empty() {
                    None
                } else {
                    let sum: u32 = ratings.iter().sum();
                    Some(sum as f64 / ratings.len() as f64)
                };

                ChannelBreakdown {
                    channel,
                    total_conversations: total,
                    total_messages: total_msgs,
                    avg_duration_seconds: if total > 0 {
                        total_dur / total as f64
                    } else {
                        0.0
                    },
                    csat_score: csat,
                }
            })
            .collect()
    }

    pub fn by_bot(&self, bot_id: Uuid) -> Option<BotAnalytics> {
        let records: Vec<_> = self
            .conversations
            .iter()
            .filter(|c| c.bot_id == bot_id)
            .collect();

        if records.is_empty() {
            return None;
        }

        let total = records.len() as u64;
        let total_msgs: u64 = records.iter().map(|r| r.message_count as u64).sum();
        let ratings: Vec<u32> = records.iter().filter_map(|r| r.csat_rating).collect();
        let csat = if ratings.is_empty() {
            None
        } else {
            let sum: u32 = ratings.iter().sum();
            Some(sum as f64 / ratings.len() as f64)
        };

        let mut channel_map: HashMap<String, Vec<&ConversationRecord>> = HashMap::new();
        for rec in &records {
            channel_map.entry(rec.channel.clone()).or_default().push(rec);
        }

        let channel_breakdown: Vec<ChannelBreakdown> = channel_map
            .into_iter()
            .map(|(ch, recs)| {
                let ch_total = recs.len() as u64;
                let ch_msgs: u64 = recs.iter().map(|r| r.message_count as u64).sum();
                let ch_dur: f64 = recs.iter().map(|r| r.duration_seconds).sum();
                let ch_ratings: Vec<u32> = recs.iter().filter_map(|r| r.csat_rating).collect();
                let ch_csat = if ch_ratings.is_empty() {
                    None
                } else {
                    let sum: u32 = ch_ratings.iter().sum();
                    Some(sum as f64 / ch_ratings.len() as f64)
                };

                ChannelBreakdown {
                    channel: ch,
                    total_conversations: ch_total,
                    total_messages: ch_msgs,
                    avg_duration_seconds: if ch_total > 0 {
                        ch_dur / ch_total as f64
                    } else {
                        0.0
                    },
                    csat_score: ch_csat,
                }
            })
            .collect();

        let earliest = records.iter().map(|r| r.started_at).min().unwrap_or(Utc::now());
        let latest = records
            .iter()
            .filter_map(|r| r.ended_at)
            .max()
            .unwrap_or(Utc::now());

        Some(BotAnalytics {
            bot_id,
            bot_name: String::new(),
            channel_breakdown,
            total_conversations: total,
            total_messages: total_msgs,
            csat_score: csat,
            period_start: earliest,
            period_end: latest,
        })
    }

    fn extract_top_intents(
        &self,
        records: &[&ConversationRecord],
        limit: usize,
    ) -> Vec<IntentCount> {
        let keyword_map = self.build_intent_keywords();
        let mut intent_counts: HashMap<String, u64> = HashMap::new();

        for record in records {
            let text: String = record
                .messages
                .iter()
                .filter(|m| m.role == "user")
                .map(|m| m.content.clone())
                .collect::<Vec<_>>()
                .join(" ");
            let lower = text.to_lowercase();

            for (intent, keywords) in &keyword_map {
                if keywords.iter().any(|kw| lower.contains(kw)) {
                    *intent_counts.entry(intent.clone()).or_default() += 1;
                }
            }
        }

        let mut counts: Vec<IntentCount> = intent_counts
            .into_iter()
            .map(|(intent, count)| IntentCount { intent, count })
            .collect();
        counts.sort_by_key(|a| std::cmp::Reverse(a.count.clone()));
        counts.truncate(limit);
        counts
    }

    fn build_intent_keywords(&self) -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();
        map.insert(
            "support".to_string(),
            vec![
                "help".to_string(),
                "support".to_string(),
                "issue".to_string(),
                "problem".to_string(),
                "broken".to_string(),
                "error".to_string(),
            ],
        );
        map.insert(
            "sales".to_string(),
            vec![
                "buy".to_string(),
                "purchase".to_string(),
                "price".to_string(),
                "cost".to_string(),
                "quote".to_string(),
                "order".to_string(),
            ],
        );
        map.insert(
            "account".to_string(),
            vec![
                "account".to_string(),
                "login".to_string(),
                "password".to_string(),
                "profile".to_string(),
                "settings".to_string(),
            ],
        );
        map.insert(
            "billing".to_string(),
            vec![
                "billing".to_string(),
                "invoice".to_string(),
                "payment".to_string(),
                "refund".to_string(),
                "charge".to_string(),
            ],
        );
        map.insert(
            "information".to_string(),
            vec![
                "info".to_string(),
                "details".to_string(),
                "how".to_string(),
                "what".to_string(),
                "tell me about".to_string(),
            ],
        );
        map
    }

    pub fn calculate_csat(&self, bot_id: Option<Uuid>) -> Option<f64> {
        let ratings: Vec<u32> = self
            .conversations
            .iter()
            .filter(|c| {
                if let Some(bid) = bot_id {
                    c.bot_id == bid && c.csat_rating.is_some()
                } else {
                    c.csat_rating.is_some()
                }
            })
            .filter_map(|c| c.csat_rating)
            .collect();

        if ratings.is_empty() {
            return None;
        }
        let sum: u32 = ratings.iter().sum();
        Some(sum as f64 / ratings.len() as f64)
    }
}

impl Default for AnalyticsService {
    fn default() -> Self {
        Self::new()
    }
}
