use super::*;

impl AdvisorEngine {
    pub async fn analyze(
        state: &AppState,
        campaign_id: Uuid,
    ) -> Result<Vec<Recommendation>, String> {
        let mut recommendations = Vec::new();

        let campaign = Self::get_campaign(state, campaign_id).await?;
        let branch_id = campaign.branch_id;
        let metrics = Self::get_campaign_metrics(state, campaign_id).await?;

        recommendations.extend(Self::check_bounce_rate(&metrics));
        recommendations.extend(Self::check_open_rate(&metrics));
        recommendations.extend(Self::check_spam_complaints(&metrics));
        recommendations.extend(Self::check_unsubscribe_rate(&metrics));
        recommendations.extend(Self::check_delivery_rate(&metrics));
        recommendations.extend(Self::check_engagement_trend(&metrics));

        Self::store_recommendations(state, branch_id, campaign_id, &recommendations).await?;

        Ok(recommendations)
    }

    async fn get_campaign(
        state: &AppState,
        campaign_id: Uuid,
    ) -> Result<CrmCampaign, String> {
        use crate::schema::marketing_campaigns::dsl::*;
        let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
        marketing_campaigns
            .filter(id.eq(campaign_id))
            .first(&mut conn)
            .map_err(|e| format!("Campaign not found: {e}"))
    }

    async fn get_campaign_metrics(
        state: &AppState,
        campaign_id_val: Uuid,
    ) -> Result<DbMetricsRow, String> {
        use crate::schema::campaign_metrics::dsl::*;
        let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
        let rows: Vec<(String, BigDecimal)> = campaign_metrics
            .filter(campaign_id.eq(campaign_id_val))
            .select((metric_type, metric_value))
            .load(&mut conn)
            .unwrap_or_default();
        let mut m = DbMetricsRow {
            sent_count: 0,
            delivered_count: 0,
            bounce_count: 0,
            open_count: 0,
            click_count: 0,
            complaint_count: 0,
            unsubscribe_count: 0,
            reply_count: 0,
        };
        for (mtype, mvalue) in rows {
            let val: i64 = mvalue.to_string().parse().unwrap_or(0);
            match mtype.as_str() {
                "sent" => m.sent_count = val,
                "delivered" => m.delivered_count = val,
                "bounce" => m.bounce_count = val,
                "open" => m.open_count = val,
                "click" => m.click_count = val,
                "complaint" => m.complaint_count = val,
                "unsubscribe" => m.unsubscribe_count = val,
                "reply" => m.reply_count = val,
                _ => {}
            }
        }
        Ok(m)
    }

    async fn store_recommendations(
        state: &AppState,
        branch_id_val: Uuid,
        campaign_id_val: Uuid,
        recommendations: &[Recommendation],
    ) -> Result<(), String> {
        use crate::schema::advisor_recommendations::dsl::{advisor_recommendations, campaign_id, dismissed};
        let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
        let now = Utc::now();

        diesel::delete(
            advisor_recommendations
                .filter(campaign_id.eq(campaign_id_val).and(dismissed.eq(false))),
        )
        .execute(&mut conn)
        .map_err(|e| format!("Delete error: {e}"))?;

        for rec in recommendations {
            let severity_str = match rec.severity {
                RecommendationSeverity::Critical => "critical",
                RecommendationSeverity::Warning => "warning",
                RecommendationSeverity::Info => "info",
            };
            let new_rec = AdvisorRecommendation {
                id: Uuid::new_v4(),
                branch_id: branch_id_val,
                campaign_id: Some(campaign_id_val),
                recommendation: rec.check_name.clone(),
                reason: Some(rec.message.clone()),
                check_name: rec.check_name.clone(),
                severity: severity_str.to_string(),
                message: rec.message.clone(),
                details: rec.details.clone(),
                dismissed: false,
                created_at: now,
            };

            diesel::insert_into(advisor_recommendations)
                .values(&new_rec)
                .execute(&mut conn)
                .map_err(|e| format!("Insert error: {e}"))?;
        }

        Ok(())
    }

    pub(super) fn check_bounce_rate(metrics: &DbMetricsRow) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        if metrics.sent_count == 0 {
            return recs;
        }

        let bounce_rate = metrics.bounce_count as f64 / metrics.sent_count as f64;

        if bounce_rate > 0.05 {
            recs.push(Recommendation {
                check_name: CheckType::BounceRate.as_str().to_string(),
                severity: RecommendationSeverity::Critical,
                message: format!(
                    "Bounce rate is {:.1}%, above 5% threshold",
                    bounce_rate * 100.0
                ),
                details: Some("High bounce rates damage sender reputation.".to_string()),
                action_items: vec![
                    "Clean list - remove hard bounces".to_string(),
                    "Verify email addresses before sending".to_string(),
                    "Implement double opt-in".to_string(),
                ],
            });
        } else if bounce_rate > 0.03 {
            recs.push(Recommendation {
                check_name: CheckType::BounceRate.as_str().to_string(),
                severity: RecommendationSeverity::Warning,
                message: format!(
                    "Bounce rate is {:.1}%, approaching 5% threshold",
                    bounce_rate * 100.0
                ),
                details: Some("Monitor bounce rates closely.".to_string()),
                action_items: vec!["Review recent list imports for quality".to_string()],
            });
        }

        recs
    }

    pub(super) fn check_open_rate(metrics: &DbMetricsRow) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        if metrics.delivered_count == 0 {
            return recs;
        }

        let open_rate = metrics.open_count as f64 / metrics.delivered_count as f64;

        if open_rate < 0.15 {
            recs.push(Recommendation {
                check_name: CheckType::OpenRate.as_str().to_string(),
                severity: RecommendationSeverity::Warning,
                message: format!(
                    "Open rate is {:.1}%, below 15% benchmark",
                    open_rate * 100.0
                ),
                details: Some(
                    "Low open rates may indicate poor subject lines or timing.".to_string(),
                ),
                action_items: vec![
                    "A/B test subject lines".to_string(),
                    "Optimize send times for your audience".to_string(),
                    "Segment your list for more relevant content".to_string(),
                    "Check spam folder placement".to_string(),
                ],
            });
        }

        recs
    }

    pub(super) fn check_spam_complaints(metrics: &DbMetricsRow) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        if metrics.sent_count == 0 {
            return recs;
        }

        let complaint_rate = metrics.complaint_count as f64 / metrics.sent_count as f64;

        if complaint_rate > 0.001 {
            recs.push(Recommendation {
                check_name: CheckType::SpamComplaints.as_str().to_string(),
                severity: RecommendationSeverity::Critical,
                message: format!(
                    "Spam complaint rate is {:.2}%, above 0.1% threshold",
                    complaint_rate * 100.0
                ),
                details: Some("Complaints severely damage sender reputation.".to_string()),
                action_items: vec![
                    "Remove complainers immediately".to_string(),
                    "Review sending frequency".to_string(),
                    "Ensure clear unsubscribe options".to_string(),
                    "Verify opt-in consent".to_string(),
                ],
            });
        }

        recs
    }

    pub(super) fn check_unsubscribe_rate(metrics: &DbMetricsRow) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        if metrics.delivered_count == 0 {
            return recs;
        }

        let unsub_rate = metrics.unsubscribe_count as f64 / metrics.delivered_count as f64;

        if unsub_rate > 0.005 {
            recs.push(Recommendation {
                check_name: CheckType::UnsubscribeRate.as_str().to_string(),
                severity: RecommendationSeverity::Warning,
                message: format!(
                    "Unsubscribe rate is {:.2}%, above 0.5% threshold",
                    unsub_rate * 100.0
                ),
                details: Some("High unsubscribes may indicate content mismatch.".to_string()),
                action_items: vec![
                    "Review content relevance".to_string(),
                    "Check sending frequency".to_string(),
                    "Verify list targeting".to_string(),
                ],
            });
        }

        recs
    }

    pub(super) fn check_delivery_rate(metrics: &DbMetricsRow) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        if metrics.sent_count == 0 {
            return recs;
        }

        let delivery_rate = metrics.delivered_count as f64 / metrics.sent_count as f64;

        if delivery_rate < 0.95 {
            recs.push(Recommendation {
                check_name: CheckType::DeliveryRate.as_str().to_string(),
                severity: RecommendationSeverity::Critical,
                message: format!(
                    "Delivery rate is {:.1}%, below 95% benchmark",
                    delivery_rate * 100.0
                ),
                details: Some(
                    "Low delivery indicates reputation or technical issues.".to_string(),
                ),
                action_items: vec![
                    "Check bounce reasons".to_string(),
                    "Verify sender authentication".to_string(),
                    "Review blocklist status".to_string(),
                    "Check for IP reputation issues".to_string(),
                ],
            });
        }

        recs
    }
}
