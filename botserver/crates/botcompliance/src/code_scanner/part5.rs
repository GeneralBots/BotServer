use super::*;

impl CodeScanner {
    pub async fn scan_bots(&self, bot_ids: &[String]) -> Result<ComplianceScanResult, Box<dyn std::error::Error + Send + Sync>> {
        if bot_ids.is_empty() || bot_ids.contains(&"all".to_string()) {
            return self.scan_all().await;
        }

        let mut full_result = self.scan_all().await?;
        full_result.bot_results.retain(|r| bot_ids.contains(&r.bot_id) || bot_ids.contains(&r.bot_name));

        let mut new_stats = ScanStats::default();
        for bot in &full_result.bot_results {
            new_stats.merge(&bot.stats);
        }
        full_result.stats = new_stats;
        full_result.bots_scanned = full_result.bot_results.len();

        Ok(full_result)
    }
}
