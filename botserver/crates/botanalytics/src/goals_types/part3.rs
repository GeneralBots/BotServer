use super::*;

impl MetricType {
    pub fn from_label(s: &str) -> Self {
        match s {
            "percentage" => Self::Percentage,
            "currency" => Self::Currency,
            "boolean" => Self::Boolean,
            _ => Self::Number,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Percentage => "percentage",
            Self::Number => "number",
            Self::Currency => "currency",
            Self::Boolean => "boolean",
        }
    }
}
