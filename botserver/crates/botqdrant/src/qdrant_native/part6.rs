use super::*;

impl Filter {
    pub fn must(conditions: Vec<Condition>) -> Self {
        Self { must: conditions }
    }
}
