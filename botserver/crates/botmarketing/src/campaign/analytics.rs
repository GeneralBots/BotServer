use crate::campaign::models::{Campaign, CampaignMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignReport {
    pub campaign_id: String,
    pub campaign_name: String,
    pub period_start: String,
    pub period_end: String,
    pub summary: ReportSummary,
    pub channel_breakdown: Vec<ChannelReport>,
    pub daily_metrics: Vec<DailyMetric>,
    pub roi_analysis: RoiAnalysis,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_impressions: u64,
    pub total_clicks: u64,
    pub total_conversions: u64,
    pub total_spend: f64,
    pub total_revenue: f64,
    pub overall_engagement_rate: f64,
    pub conversion_rate: f64,
    pub cost_per_click: f64,
    pub cost_per_conversion: f64,
    pub active_channels: usize,
    pub content_pieces: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelReport {
    pub channel: String,
    pub impressions: u64,
    pub clicks: u64,
    pub conversions: u64,
    pub spend: f64,
    pub revenue: Option<f64>,
    pub engagement_rate: f64,
    pub roi_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyMetric {
    pub date: String,
    pub impressions: u64,
    pub clicks: u64,
    pub conversions: u64,
    pub spend: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoiAnalysis {
    pub total_investment: f64,
    pub total_return: f64,
    pub roi_pct: f64,
    pub payback_days: Option<u64>,
    pub is_profitable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelComparison {
    pub best_performing: String,
    pub highest_engagement: String,
    pub lowest_cost_per_conversion: String,
    pub channel_rankings: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub impression_trend: Vec<f64>,
    pub engagement_trend: Vec<f64>,
    pub conversion_trend: Vec<f64>,
    pub growth_rate: f64,
    pub momentum: f64,
}

pub fn generate_report(campaign: &Campaign, period_start: &str, period_end: &str) -> CampaignReport {
    let metrics = &campaign.metrics;
    let summary = build_summary(metrics, &campaign.channels);
    let channel_breakdown = build_channel_breakdown(metrics);
    let daily_metrics = build_daily_metrics(metrics);
    let roi = calculate_roi(metrics);
    let recommendations = generate_recommendations(&summary, &channel_breakdown);

    CampaignReport {
        campaign_id: campaign.id.clone(),
        campaign_name: campaign.name.clone(),
        period_start: period_start.to_string(),
        period_end: period_end.to_string(),
        summary,
        channel_breakdown,
        daily_metrics,
        roi_analysis: roi,
        recommendations,
    }
}

fn build_summary(metrics: &CampaignMetrics, channels: &[crate::campaign::models::ChannelConfig]) -> ReportSummary {
    let spend = metrics.total_spend();
    let revenue = metrics.total_revenue();
    let conv_rate = if metrics.impressions > 0 {
        (metrics.conversions as f64 / metrics.impressions as f64) * 100.0
    } else {
        0.0
    };
    let cpc = if metrics.clicks > 0 { spend / metrics.clicks as f64 } else { 0.0 };
    let cpconv = if metrics.conversions > 0 { spend / metrics.conversions as f64 } else { 0.0 };

    ReportSummary {
        total_impressions: metrics.impressions,
        total_clicks: metrics.clicks,
        total_conversions: metrics.conversions,
        total_spend: spend,
        total_revenue: revenue,
        overall_engagement_rate: metrics.engagement_rate,
        conversion_rate: conv_rate,
        cost_per_click: cpc,
        cost_per_conversion: cpconv,
        active_channels: channels.iter().filter(|c| c.enabled).count(),
        content_pieces: metrics.channel_metrics.len(),
    }
}

fn build_channel_breakdown(metrics: &CampaignMetrics) -> Vec<ChannelReport> {
    let total_imp = metrics.impressions.max(1);
    metrics.channel_metrics.iter().map(|(ch, cm)| {
        let er = (cm.clicks as f64 / total_imp as f64) * 100.0;
        let roi = cm.revenue.filter(|_| cm.spend > 0.0)
            .map(|rev| ((rev - cm.spend) / cm.spend) * 100.0);
        ChannelReport {
            channel: ch.clone(), impressions: cm.impressions, clicks: cm.clicks,
            conversions: cm.conversions, spend: cm.spend, revenue: cm.revenue,
            engagement_rate: er, roi_pct: roi,
        }
    }).collect()
}

fn build_daily_metrics(metrics: &CampaignMetrics) -> Vec<DailyMetric> {
    let now = chrono::Utc::now();
    let n = metrics.channel_metrics.len().max(1) as u64;
    (0..7).rev().map(|d| {
        let day = now - chrono::Duration::days(d);
        let div = |v: u64| v / 7;
        DailyMetric {
            date: day.format("%Y-%m-%d").to_string(),
            impressions: metrics.channel_metrics.values().map(|c| div(c.impressions)).sum::<u64>() / n,
            clicks: metrics.channel_metrics.values().map(|c| div(c.clicks)).sum::<u64>() / n,
            conversions: metrics.channel_metrics.values().map(|c| div(c.conversions)).sum::<u64>() / n,
            spend: metrics.total_spend() / 7.0,
        }
    }).collect()
}

fn calculate_roi(metrics: &CampaignMetrics) -> RoiAnalysis {
    let investment = metrics.total_spend();
    let ret = metrics.total_revenue();
    let roi_pct = if investment > 0.0 { ((ret - investment) / investment) * 100.0 } else { 0.0 };

    let payback = if roi_pct > 0.0 && ret > 0.0 {
        let m_return = ret.max(1.0);
        let m_inv = investment.max(1.0);
        let months = m_inv / (m_return - m_inv);
        if months > 0.0 { Some((months * 30.0).ceil() as u64) } else { None }
    } else { None };

    RoiAnalysis {
        total_investment: investment,
        total_return: ret,
        roi_pct,
        payback_days: payback,
        is_profitable: roi_pct > 0.0,
    }
}

fn generate_recommendations(summary: &ReportSummary, channels: &[ChannelReport]) -> Vec<String> {
    let mut recs = Vec::new();

    if summary.overall_engagement_rate < 2.0 {
        recs.push("Taxa de engajamento abaixo de 2%. Revise o conteudo visual e legendas.".to_string());
    }
    if summary.cost_per_conversion > summary.cost_per_click * 10.0 {
        recs.push("Custo por conversao alto. Otimize a pagina de destino e o funil.".to_string());
    }

    let low: Vec<&str> = channels.iter()
        .filter(|c| c.impressions < 100 && c.spend > 0.0)
        .map(|c| c.channel.as_str())
        .collect();
    if !low.is_empty() {
        recs.push(format!("Canais com baixo desempenho: {}. Considere pausar ou realocar orcamento.", low.join(", ")));
    }

    if summary.total_revenue > summary.total_spend {
        let roi = ((summary.total_revenue - summary.total_spend) / summary.total_spend.max(1.0) * 100.0) as i64;
        recs.push(format!("ROI positivo de {}%. Aumente investimento nos canais mais rentaveis.", roi));
    } else if summary.total_spend > 0.0 {
        recs.push("Campanha abaixo do ponto de equilibrio. Revise segmentacao e proposta de valor.".to_string());
    }

    if recs.is_empty() {
        recs.push("Bons resultados. Monitore regularmente e teste variacoes de conteudo.".to_string());
    }
    recs
}

pub fn compare_channels(metrics: &CampaignMetrics) -> Option<ChannelComparison> {
    if metrics.channel_metrics.is_empty() {
        return None;
    }

    let mut rankings: Vec<(String, f64)> = metrics.channel_metrics.iter()
        .map(|(ch, cm)| {
            let score = cm.impressions as f64 * 0.3 + cm.clicks as f64 * 0.3 + cm.conversions as f64 * 0.4;
            (ch.clone(), score)
        })
        .collect();
    rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let best = rankings.first().map(|(c, _)| c.clone());
    let highest_eng = metrics.channel_metrics.iter()
        .max_by(|(_, a), (_, b)| {
            let ra = if a.impressions > 0 { a.clicks as f64 / a.impressions as f64 } else { 0.0 };
            let rb = if b.impressions > 0 { b.clicks as f64 / b.impressions as f64 } else { 0.0 };
            ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(c, _)| c.clone());

    let lowest_cpa = metrics.channel_metrics.iter()
        .filter(|(_, cm)| cm.conversions > 0 && cm.spend > 0.0)
        .min_by(|(_, a), (_, b)| {
            (a.spend / a.conversions as f64).partial_cmp(&(b.spend / b.conversions as f64))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(c, _)| c.clone());

    Some(ChannelComparison {
        best_performing: best.unwrap_or_default(),
        highest_engagement: highest_eng.unwrap_or_default(),
        lowest_cost_per_conversion: lowest_cpa.unwrap_or_default(),
        channel_rankings: rankings,
    })
}

pub fn calculate_trend(metrics: &[CampaignMetrics]) -> TrendAnalysis {
    let impression_trend: Vec<f64> = metrics.iter().map(|m| m.impressions as f64).collect();
    let engagement_trend: Vec<f64> = metrics.iter().map(|m| m.engagement_rate).collect();
    let conversion_trend: Vec<f64> = metrics.iter().map(|m| m.conversions as f64).collect();

    let first = metrics.first().map(|m| m.impressions as f64).unwrap_or(0.0);
    let last = metrics.last().map(|m| m.impressions as f64).unwrap_or(0.0);
    let growth = if first > 0.0 { ((last - first) / first) * 100.0 } else { 0.0 };

    let recent = metrics.iter().rev().take(3).map(|m| m.impressions).sum::<u64>() as f64 / 3.0;
    let older = metrics.iter().take(3).map(|m| m.impressions).sum::<u64>() as f64 / 3.0;
    let momentum = if older > 0.0 { ((recent - older) / older) * 100.0 } else { 0.0 };

    TrendAnalysis {
        impression_trend,
        engagement_trend,
        conversion_trend,
        growth_rate: growth,
        momentum,
    }
}

pub fn aggregate_cross_channel(campaign: &Campaign) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();
    let m = &campaign.metrics;
    result.insert("total_impressions".into(), serde_json::json!(m.impressions));
    result.insert("total_clicks".into(), serde_json::json!(m.clicks));
    result.insert("total_conversions".into(), serde_json::json!(m.conversions));
    result.insert("engagement_rate".into(), serde_json::json!(m.engagement_rate));
    result.insert("total_spend".into(), serde_json::json!(m.total_spend()));
    result.insert("total_revenue".into(), serde_json::json!(m.total_revenue()));
    result.insert("roi".into(), serde_json::json!(m.roi));
    result.insert("channel_count".into(), serde_json::json!(m.channel_metrics.len()));

    let channels: Vec<serde_json::Value> = m.channel_metrics.iter().map(|(ch, cm)| {
        serde_json::json!({ "channel": ch, "impressions": cm.impressions, "clicks": cm.clicks,
            "conversions": cm.conversions, "spend": cm.spend, "revenue": cm.revenue })
    }).collect();
    result.insert("channels".into(), serde_json::json!(channels));
    result
}

pub fn export_csv(campaigns: &[Campaign]) -> String {
    let mut csv = "campaign_id,name,status,impressions,clicks,conversions,engagement_rate,spend,revenue,roi\n".to_string();
    for c in campaigns {
        let status = serde_json::to_string(&c.status).unwrap_or_else(|_| "\"unknown\"".into());
        csv.push_str(&format!(
            "{},{},{},{},{},{},{:.2},{:.2},{:.2},{}\n",
            c.id, escape_csv(&c.name), status.trim_matches('"'),
            c.metrics.impressions, c.metrics.clicks, c.metrics.conversions,
            c.metrics.engagement_rate, c.metrics.total_spend(), c.metrics.total_revenue(),
            c.metrics.roi.map(|r| format!("{:.2}", r)).unwrap_or_else(|| "N/A".to_string()),
        ));
        for (ch, cm) in &c.metrics.channel_metrics {
            csv.push_str(&format!(
                ",{},{},{},{},{},{:.2},{}\n", ch,
                cm.impressions, cm.clicks, cm.conversions, cm.spend,
                cm.revenue.map(|r| format!("{:.2}", r)).unwrap_or_else(|| "N/A".to_string()),
                cm.impressions,
            ));
        }
    }
    csv
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn merge_campaign_metrics(base: &mut CampaignMetrics, incoming: &CampaignMetrics) {
    base.impressions = base.impressions.saturating_add(incoming.impressions);
    base.clicks = base.clicks.saturating_add(incoming.clicks);
    base.conversions = base.conversions.saturating_add(incoming.conversions);
    if incoming.engagement_rate > 0.0 {
        base.engagement_rate = (base.engagement_rate + incoming.engagement_rate) / 2.0;
    }
    for (ch, m) in &incoming.channel_metrics {
        let e = base.channel_metrics.entry(ch.clone()).or_default();
        e.impressions = e.impressions.saturating_add(m.impressions);
        e.clicks = e.clicks.saturating_add(m.clicks);
        e.conversions = e.conversions.saturating_add(m.conversions);
        e.spend += m.spend;
        if let Some(rev) = m.revenue {
            e.revenue = Some(e.revenue.unwrap_or(0.0) + rev);
        }
    }
}

#[cfg(test)]
#[path = "analytics_tests.rs"]
mod tests;
