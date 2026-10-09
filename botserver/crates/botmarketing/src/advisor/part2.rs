use super::*;

impl CheckType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SpfDkimDmarc => "spf_dkim_dmarc",
            Self::BounceRate => "bounce_rate",
            Self::OpenRate => "open_rate",
            Self::SpamComplaints => "spam_complaints",
            Self::NewIp => "new_ip",
            Self::ListAge => "list_age",
            Self::UnsubscribeRate => "unsubscribe_rate",
            Self::DeliveryRate => "delivery_rate",
            Self::EngagementTrend => "engagement_trend",
        }
    }
}
