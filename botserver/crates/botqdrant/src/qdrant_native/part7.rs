use super::*;

impl Condition {
    pub fn matches(field: &str, value: Value) -> Self {
        Self {
            condition_type: ConditionType::Field {
                key: field.to_string(),
                value,
            },
        }
    }
}
