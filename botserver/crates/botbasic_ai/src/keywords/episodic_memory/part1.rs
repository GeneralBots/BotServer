use super::*;

impl Default for Sentiment {
    fn default() -> Self {
        Self {
            score: 0.0,
            label: SentimentLabel::Neutral,
            confidence: 0.5,
        }
    }
}
