use super::*;

impl ApprovalRequest {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();

        map.insert("id".into(), self.id.to_string().into());
        map.insert("bot_id".into(), self.bot_id.to_string().into());
        map.insert("session_id".into(), self.session_id.to_string().into());
        map.insert("initiated_by".into(), self.initiated_by.to_string().into());
        map.insert("approval_type".into(), self.approval_type.clone().into());
        map.insert(
            "status".into(),
            format!("{:?}", self.status).to_lowercase().into(),
        );
        map.insert("channel".into(), self.channel.to_string().into());
        map.insert("recipient".into(), self.recipient.clone().into());
        map.insert("context".into(), json_to_dynamic(&self.context));
        map.insert("message".into(), self.message.clone().into());
        map.insert(
            "timeout_seconds".into(),
            i64::from(self.timeout_seconds as u32).into(),
        );
        map.insert("current_level".into(), i64::from(self.current_level).into());
        map.insert("total_levels".into(), i64::from(self.total_levels).into());
        map.insert("created_at".into(), self.created_at.to_rfc3339().into());
        map.insert("expires_at".into(), self.expires_at.to_rfc3339().into());

        if let Some(ref decision) = self.decision {
            map.insert(
                "decision".into(),
                format!("{:?}", decision).to_lowercase().into(),
            );
        }

        if let Some(ref decided_by) = self.decided_by {
            map.insert("decided_by".into(), decided_by.clone().into());
        }

        if let Some(ref decided_at) = self.decided_at {
            map.insert("decided_at".into(), decided_at.to_rfc3339().into());
        }

        if let Some(ref comments) = self.comments {
            map.insert("comments".into(), comments.clone().into());
        }

        Dynamic::from(map)
    }
}
pub(super) fn json_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let array: Array = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(array)
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}
pub fn register_approval_keywords(engine: &mut Engine) {
    engine.register_fn("approval_is_approved", |request: Map| -> bool {
        request
            .get("status")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(|s| s == "approved")
            .unwrap_or(false)
    });

    engine.register_fn("approval_is_rejected", |request: Map| -> bool {
        request
            .get("status")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(|s| s == "rejected")
            .unwrap_or(false)
    });

    engine.register_fn("approval_is_pending", |request: Map| -> bool {
        request
            .get("status")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(|s| s == "pending")
            .unwrap_or(false)
    });

    engine.register_fn("approval_is_timed_out", |request: Map| -> bool {
        request
            .get("status")
            .and_then(|v| v.clone().try_cast::<String>())
            .map(|s| s == "timedout")
            .unwrap_or(false)
    });

    engine.register_fn("approval_decision", |request: Map| -> String {
        request
            .get("decision")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_else(|| "pending".to_string())
    });

    engine.register_fn("approval_decided_by", |request: Map| -> String {
        request
            .get("decided_by")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default()
    });

    engine.register_fn("approval_comments", |request: Map| -> String {
        request
            .get("comments")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default()
    });

    info!("Approval keywords registered");
}
