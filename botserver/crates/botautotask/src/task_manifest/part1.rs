use super::*;

impl std::fmt::Display for SectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseModels => write!(f, "Database & Models"),
            Self::SchemaDesign => write!(f, "Database Schema Design"),
            Self::Tables => write!(f, "Tables"),
            Self::Files => write!(f, "Files"),
            Self::Pages => write!(f, "Pages"),
            Self::Tools => write!(f, "Tools"),
            Self::Schedulers => write!(f, "Schedulers"),
            Self::Monitors => write!(f, "Monitors"),
            Self::Validation => write!(f, "Validation"),
            Self::Deployment => write!(f, "Deployment"),
        }
    }
}
