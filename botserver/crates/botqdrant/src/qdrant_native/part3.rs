use super::*;

impl VectorParamsBuilder {
    pub fn new(size: u64, distance: Distance) -> Self {
        Self {
            size,
            distance: format!("{:?}", distance),
        }
    }
}
