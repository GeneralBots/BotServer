use super::*;

impl VibeState for VibeStateImpl {
    fn db_pool(&self) -> &botvibe::types::DbPool {
        &self.pool
    }

    fn broadcast_progress(&self, event: VibeProgressEvent) {
        if let Some(tx) = &self.progress {
            let _ = tx.send(event);
        }
    }

    fn progress_sender(&self) -> Option<&broadcast::Sender<VibeProgressEvent>> {
        self.progress.as_ref()
    }

    fn active_runs(&self) -> &Arc<RwLock<HashMap<Uuid, VibeRun>>> {
        &self.runs
    }

    fn run_signal_sender(&self) -> Option<&broadcast::Sender<VibeRunSignal>> {
        self.signals.as_ref()
    }

    /// Resolves the bot's LLM settings via ConfigManager (Issue #795).
    /// Sensitive keys (e.g. `llm-key`) come from Vault per-bot paths;
    /// non-sensitive keys (url/model/provider) from the bot's Drive
    /// config.csv via the `bot_configuration` table. `get_config` returns
    /// `Ok("")` on a miss, so empty values are filtered out and fall through
    /// to the environment in the agent loop.
    fn llm_config(&self, bot_id: &Uuid) -> Option<LlmConfig> {
        self.resolve_llm_slot(bot_id, "vibe-llm-")
    }

    /// Resolves the per-agent LLM provider configured in Settings → Vibe.
    /// Each agent slot (`reasoning` | `agentic` | `fast`) owns a complete
    /// `vibe-llm-{agent}-{provider,url,model,key}` set; falls back to the
    /// legacy `vibe-llm-*` set (agentic default) and then the standard chain.
    fn llm_config_for(&self, bot_id: &Uuid, agent: &str) -> Option<LlmConfig> {
        let slot = format!("vibe-llm-{agent}-");
        if agent != "agentic" {
            if let Some(cfg) = self.resolve_llm_slot(bot_id, &slot) {
                return Some(cfg);
            }
        }
        // agentic or a missing slot: the legacy vibe-llm-* set is the
        // agentic default written by older deployments.
        self.resolve_llm_slot(bot_id, "vibe-llm-")
    }
}
impl VibeStateImpl {
    /// Resolves a complete `{prefix}{provider,url,model,key}` config set for
    /// the bot, falling through to the legacy per-bot/global chain when the
    /// prefixed set is absent or incomplete. Returns `None` when nothing
    /// resolves (the agent loop then uses the environment).
    pub(super) fn resolve_llm_slot(&self, bot_id: &Uuid, prefix: &str) -> Option<LlmConfig> {
        let value = |key: &str| {
            self.config
                .get_config(bot_id, key, None)
                .ok()
                .filter(|v| !v.is_empty())
        };
        // The user-selected Vibe LLM provider (Settings → Vibe) wins over
        // everything: a complete set of {prefix}{provider,url,model,key} keys
        // is written per-bot when the user picks a connected LLM provider, so
        // an incomplete set (e.g. a key that failed to persist) falls through
        // to the legacy chain.
        if value(&format!("{prefix}provider")).is_some() {
            let model = value(&format!("{prefix}model"));
            let url = value(&format!("{prefix}url"));
            let key = value(&format!("{prefix}key")).unwrap_or_default();
            if let (Some(model), Some(url)) = (model, url) {
                return Some(LlmConfig { model, key, url });
            }
        }
        if prefix != "vibe-llm-" {
            // A non-agentic slot that is not fully configured must never
            // half-resolve — fall straight to the legacy chain.
            let legacy = self.resolve_llm_slot(bot_id, "vibe-llm-");
            if legacy.is_some() {
                return legacy;
            }
        }
        let per_bot = value("llm-model").map(|model| LlmConfig {
                model,
                key: value("llm-key").unwrap_or_default(),
                url: value("llm-url").unwrap_or_default(),
            });
        // The per-bot path can resolve a model (Drive config.csv) while its
        // Vault secret lacks the key/url — that must NOT shadow the global
        // secret/gbo/llm (tokenrouter) that the bot chat uses, or every run
        // dies with HTTP 401 "Invalid token". Merge: start from per-bot,
        // then fill any empty field from the global fallback.
        //
        // A model-only per-bot config (no llm-url) must NOT be combined with
        // the global endpoint: the model belongs to the per-bot provider and
        // the global endpoint returns 404 for it. Treat a per-bot config
        // without its own url as incomplete and use the global set wholesale.
        if let Ok(sm) = crate::core::secrets::SecretsManager::get() {
            let (g_url, g_model, g_key, _, _) = sm.get_llm_config();
            if let Some(cfg) = per_bot.clone() {
                if !cfg.url.is_empty() {
                    let merged = LlmConfig {
                        model: if cfg.model.is_empty() { g_model.clone() } else { cfg.model.clone() },
                        key: if cfg.key.is_empty() { g_key.clone().unwrap_or_default() } else { cfg.key },
                        url: cfg.url,
                    };
                    if !merged.url.is_empty() && !merged.model.is_empty() {
                        return Some(merged);
                    }
                }
            }
            // No complete per-bot config: fall back to the global set.
            if !g_url.is_empty() && !g_model.is_empty() {
                return Some(LlmConfig {
                    model: g_model,
                    key: g_key.unwrap_or_default(),
                    url: g_url,
                });
            }
        }
        if let Some(cfg) = per_bot {
            if !cfg.url.is_empty() && !cfg.model.is_empty() {
                return Some(cfg);
            }
        }
        log::warn!("Vibe: no llm config for bot {bot_id}; falling back to env");
        None
    }
}
