use super::*;

impl QueryDecomposer {
    pub fn new(llm_endpoint: &str, api_key: &str) -> Self {
        log::trace!(
            "Creating QueryDecomposer with endpoint={} api_key_len={}",
            llm_endpoint,
            api_key.len()
        );
        Self {
            llm_endpoint: llm_endpoint.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub fn decompose(&self, query: &str) -> Result<Vec<String>, String> {
        log::trace!(
            "Decomposing query using endpoint={} (api_key configured: {})",
            self.llm_endpoint,
            !self.api_key.is_empty()
        );
        let mut sub_queries = Vec::new();

        let conjunctions = ["and", "also", "as well as", "in addition to"];
        let mut parts: Vec<&str> = vec![query];

        for conj in &conjunctions {
            parts = parts
                .iter()
                .flat_map(|p| p.split(conj))
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
        }

        if parts.len() > 1 {
            for part in parts {
                sub_queries.push(part.to_string());
            }
        } else {
            let question_words = ["what", "how", "why", "when", "where", "who"];
            let lower = query.to_lowercase();

            let mut has_multiple_questions = false;
            for qw in &question_words {
                if lower.matches(qw).count() > 1 {
                    has_multiple_questions = true;
                    break;
                }
            }

            if has_multiple_questions {
                for part in query.split('?') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        sub_queries.push(format!("{}?", trimmed));
                    }
                }
            }
        }

        if sub_queries.is_empty() {
            sub_queries.push(query.to_string());
        }

        Ok(sub_queries)
    }

    pub fn synthesize(&self, query: &str, sub_answers: &[String]) -> String {
        if sub_answers.len() == 1 {
            return sub_answers[0].clone();
        }

        let mut synthesis = format!(
            "Based on your question about \"{}\", here's what I found:\n\n",
            query
        );

        for (i, answer) in sub_answers.iter().enumerate() {
            let _ = writeln!(synthesis, "{}. {}\n", i + 1, answer);
        }

        synthesis
    }
}
pub(super) fn read_bot_config_from_vault(bot_id: &Uuid) -> Option<HashMap<String, String>> {
    use botcoresecrets::manager::SecretsManager;
    let sm = SecretsManager::get_clone().ok()?;
    if !sm.is_enabled() {
        return None;
    }
    let path = format!("gbo/{}/{}/{}", uuid::Uuid::nil(), uuid::Uuid::nil(), bot_id);
    let sm_clone = sm.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = if let Ok(rt) = rt {
            rt.block_on(async move { sm_clone.get_secret(&path).await.ok() })
        } else {
            None
        };
        let _ = tx.send(result);
    });
    rx.recv().ok().flatten()
}
