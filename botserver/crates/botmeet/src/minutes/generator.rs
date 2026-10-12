use anyhow::Result;
use log::info;
use uuid::Uuid;
use chrono::Utc;

use crate::minutes::types::{
    MeetingMinute, MinuteActionItem, AttendeeEntry, MinuteStatus,
    Transcription,
};

pub struct MinutesGenerator;

impl MinutesGenerator {
    pub async fn from_transcription(
        transcription: &Transcription,
        title: &str,
        custom_attendees: Option<Vec<AttendeeEntry>>,
        llm_api_url: Option<&str>,
        llm_api_key: Option<&str>,
        llm_model: Option<&str>,
    ) -> Result<MeetingMinute> {
        let attendees = custom_attendees
            .unwrap_or_else(|| {
                transcription.speakers.iter().map(|s| {
                    AttendeeEntry {
                        name: s.name.clone(),
                        role: Some("Participant".to_string()),
                    }
                }).collect()
            });

        let (summary, key_points, action_items, decisions) = if let (Some(url), Some(key)) = (llm_api_url, llm_api_key) {
            Self::generate_via_llm(transcription, url, key, llm_model.unwrap_or("default")).await?
        } else {
            Self::generate_via_rules(transcription)
        };

        let duration_minutes = if transcription.segments.is_empty() {
            None
        } else {
            let total_secs: f64 = transcription.segments.iter()
                .map(|s| s.end - s.start)
                .sum();
            Some((total_secs / 60.0).ceil() as i32)
        };

        info!("Minutes generated via LLM for: {title}");

        Ok(MeetingMinute {
            id: Uuid::new_v4(),
            bot_id: Uuid::nil(),
            recording_id: Some(transcription.recording_id),
            meeting_id: None,
            title: title.to_string(),
            summary,
            key_points,
            action_items,
            decisions,
            attendees,
            duration_minutes,
            status: MinuteStatus::Draft,
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn generate_via_llm(
        transcript: &Transcription,
        api_url: &str,
        api_key: &str,
        model: &str,
    ) -> Result<(String, Vec<String>, Vec<MinuteActionItem>, Vec<String>)> {
        let prompt = format!(
            "Based on this meeting transcript, generate structured minutes.\n\nTranscript:\n{}\n\nReturn JSON with:\n- summary: 2-3 sentence summary\n- key_points: array of strings\n- action_items: array of {{task, assignee (or null), due_date (or null)}}\n- decisions: array of strings",
            transcript.full_text
        );

        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": "You generate structured meeting minutes from transcripts. Return ONLY valid JSON."},
                {"role": "user", "content": prompt}
            ],
            "response_format": {"type": "json_object"}
        });

        let resp = client.post(api_url)
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("LLM request failed: {e}"))?;

        let result: serde_json::Value = resp.json().await
            .map_err(|e| anyhow::anyhow!("LLM response parse failed: {e}"))?;

        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No content in LLM response"))?;

        let parsed: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| anyhow::anyhow!("Failed to parse LLM JSON: {e}"))?;

        let summary = parsed["summary"].as_str().unwrap_or("No summary generated").to_string();

        let key_points: Vec<String> = parsed["key_points"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let action_items: Vec<MinuteActionItem> = parsed["action_items"].as_array()
            .map(|a| a.iter().map(|v| MinuteActionItem {
                task: v["task"].as_str().unwrap_or("").to_string(),
                assignee: v["assignee"].as_str().map(String::from),
                due_date: v["due_date"].as_str().map(String::from),
                status: "open".to_string(),
            }).collect())
            .unwrap_or_default();

        let decisions: Vec<String> = parsed["decisions"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok((summary, key_points, action_items, decisions))
    }

    fn generate_via_rules(transcript: &Transcription) -> (String, Vec<String>, Vec<MinuteActionItem>, Vec<String>) {
        let segment_texts: Vec<&str> = transcript.segments.iter().map(|s| s.text.as_str()).collect();
        let full = segment_texts.join(" ");

        // Transcripts are speech in any language; cut on a char boundary.
        let summary = botlib::text::truncate_with_suffix(&full, 200, "...");

        let action_phrases = ["i will", "i'll", "action item", "to-do", "todo", "need to", "must", "should", "follow up", "assign"];
        let decision_phrases = ["decided", "agreed", "consensus", "approved", "confirmed", "finalized"];

        let mut action_items = Vec::new();
        let mut decisions = Vec::new();

        for seg in &transcript.segments {
            let lower = seg.text.to_lowercase();
            if action_phrases.iter().any(|p| lower.contains(p)) {
                let assignee = lower.find("assign to ")
                    .or_else(|| lower.find("@"))
                    .map(|pos| {
                        let start = if lower[pos..].starts_with("assign to ") { pos + 10 } else { pos + 1 };
                        seg.text[start..].split(['.', ',', ' ']).next().unwrap_or("").to_string()
                    });
                action_items.push(MinuteActionItem {
                    task: seg.text.clone(),
                    assignee,
                    due_date: None,
                    status: "open".to_string(),
                });
            }
            if decision_phrases.iter().any(|p| lower.contains(p)) {
                decisions.push(seg.text.clone());
            }
        }

        let key_points: Vec<String> = transcript.segments.iter()
            .filter(|s| s.text.len() > 20)
            .take(10)
            .map(|s| s.text.clone())
            .collect();

        (summary, key_points, action_items, decisions)
    }
}
