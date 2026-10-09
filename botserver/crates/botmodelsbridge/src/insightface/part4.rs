use super::*;

impl FaceQuality {
    pub fn is_acceptable(&self, min_score: f32) -> bool {
        self.overall_score >= min_score
    }
}
