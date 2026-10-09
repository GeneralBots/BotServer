use super::*;

impl EmotionScores {
    pub fn dominant_emotion(&self) -> &'static str {
        let emotions = [
            (self.anger, "anger"),
            (self.contempt, "contempt"),
            (self.disgust, "disgust"),
            (self.fear, "fear"),
            (self.happiness, "happiness"),
            (self.neutral, "neutral"),
            (self.sadness, "sadness"),
            (self.surprise, "surprise"),
        ];

        emotions
            .iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, name)| *name)
            .unwrap_or("unknown")
    }
}
