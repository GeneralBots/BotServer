use super::*;

impl AdvisorEngine {
    pub(super) fn check_engagement_trend(metrics: &DbMetricsRow) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        if metrics.delivered_count == 0 {
            return recs;
        }

        let click_rate = metrics.click_count as f64 / metrics.delivered_count as f64;

        if click_rate < 0.02 {
            recs.push(Recommendation {
                check_name: CheckType::EngagementTrend.as_str().to_string(),
                severity: RecommendationSeverity::Info,
                message: format!(
                    "Click rate is {:.2}%, below 2% benchmark",
                    click_rate * 100.0
                ),
                details: Some(
                    "Low click rates suggest content optimization opportunities.".to_string(),
                ),
                action_items: vec![
                    "Improve call-to-action clarity".to_string(),
                    "Test different content formats".to_string(),
                    "Personalize content".to_string(),
                ],
            });
        }

        recs
    }

    pub async fn get_pending_recommendations(
        state: &AppState,
        campaign_id_val: Uuid,
    ) -> Result<Vec<AdvisorRecommendation>, String> {
        use crate::schema::advisor_recommendations::dsl::*;
        let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
        advisor_recommendations
            .filter(campaign_id.eq(campaign_id_val))
            .filter(dismissed.eq(false))
            .order(created_at.desc())
            .load(&mut conn)
            .map_err(|e| format!("Query error: {e}"))
    }

    pub async fn dismiss_recommendation(
        state: &AppState,
        recommendation_id: Uuid,
    ) -> Result<(), String> {
        let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
        diesel::update(
            crate::schema::advisor_recommendations::table
                .filter(crate::schema::advisor_recommendations::id.eq(recommendation_id))
        )
            .set(crate::schema::advisor_recommendations::dismissed.eq(true))
            .execute(&mut conn)
            .map_err(|e| format!("Update error: {e}"))?;
        Ok(())
    }
}
