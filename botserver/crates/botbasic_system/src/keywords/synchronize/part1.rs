use super::*;

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::String => write!(f, "string"),
            FieldType::Integer => write!(f, "integer"),
            FieldType::Float => write!(f, "float"),
            FieldType::Boolean => write!(f, "boolean"),
            FieldType::DateTime => write!(f, "datetime"),
            FieldType::Json => write!(f, "json"),
            FieldType::Uuid => write!(f, "uuid"),
        }
    }
}
