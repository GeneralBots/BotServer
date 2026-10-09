use super::*;

impl FacePose {
    pub fn is_frontal(&self, threshold: f32) -> bool {
        self.yaw.abs() < threshold && self.pitch.abs() < threshold && self.roll.abs() < threshold
    }
}
