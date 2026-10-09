use super::*;

impl Episode {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();

        map.insert("id".into(), self.id.to_string().into());
        map.insert("user_id".into(), self.user_id.to_string().into());
        map.insert("bot_id".into(), self.bot_id.to_string().into());
        map.insert("session_id".into(), self.session_id.to_string().into());
        map.insert("summary".into(), self.summary.clone().into());

        let topics: Array = self
            .key_topics
            .iter()
            .map(|t| Dynamic::from(t.clone()))
            .collect();
        map.insert("key_topics".into(), topics.into());

        let decisions: Array = self
            .decisions
            .iter()
            .map(|d| Dynamic::from(d.clone()))
            .collect();
        map.insert("decisions".into(), decisions.into());

        let action_items: Array = self
            .action_items
            .iter()
            .map(|a| {
                let mut item_map = Map::new();
                item_map.insert("description".into(), a.description.clone().into());
                item_map.insert(
                    "assignee".into(),
                    a.assignee.clone().unwrap_or_default().into(),
                );
                item_map.insert(
                    "priority".into(),
                    format!("{:?}", a.priority).to_lowercase().into(),
                );
                item_map.insert("completed".into(), a.completed.into());
                Dynamic::from(item_map)
            })
            .collect();
        map.insert("action_items".into(), action_items.into());

        let mut sentiment_map = Map::new();
        sentiment_map.insert("score".into(), self.sentiment.score.into());
        sentiment_map.insert(
            "label".into(),
            format!("{:?}", self.sentiment.label).to_lowercase().into(),
        );
        sentiment_map.insert("confidence".into(), self.sentiment.confidence.into());
        map.insert("sentiment".into(), sentiment_map.into());

        map.insert(
            "resolution".into(),
            format!("{:?}", self.resolution).to_lowercase().into(),
        );
        map.insert("message_count".into(), (self.message_count as i64).into());
        map.insert("created_at".into(), self.created_at.to_rfc3339().into());
        map.insert(
            "conversation_start".into(),
            self.conversation_start.to_rfc3339().into(),
        );
        map.insert(
            "conversation_end".into(),
            self.conversation_end.to_rfc3339().into(),
        );

        Dynamic::from(map)
    }
}
pub fn register_episodic_memory_keywords(engine: &mut Engine) {
    engine.register_fn("episode_summary", |episode: Map| -> String {
        episode
            .get("summary")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default()
    });

    engine.register_fn("episode_topics", |episode: Map| -> Array {
        episode
            .get("key_topics")
            .and_then(|v| v.clone().try_cast::<Array>())
            .unwrap_or_default()
    });

    engine.register_fn("episode_decisions", |episode: Map| -> Array {
        episode
            .get("decisions")
            .and_then(|v| v.clone().try_cast::<Array>())
            .unwrap_or_default()
    });

    engine.register_fn("episode_action_items", |episode: Map| -> Array {
        episode
            .get("action_items")
            .and_then(|v| v.clone().try_cast::<Array>())
            .unwrap_or_default()
    });

    engine.register_fn("episode_sentiment_score", |episode: Map| -> f64 {
        episode
            .get("sentiment")
            .and_then(|v| v.clone().try_cast::<Map>())
            .and_then(|m| m.get("score").and_then(|s| s.clone().try_cast::<f64>()))
            .unwrap_or(0.0)
    });

    engine.register_fn("episode_was_resolved", |episode: Map| -> bool {
        episode
            .get("resolution")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(|s| s == "resolved")
            .unwrap_or(false)
    });

    info!("Episodic memory keywords registered");
}
