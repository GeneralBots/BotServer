pub mod bot;
pub mod cli;
pub mod desktop;
pub mod fixtures;
mod harness;
pub mod mocks;
mod ports;
pub mod services;
pub mod web;

pub use harness::{
    BotServerInstance, BotUIInstance, Insertable, TestConfig, TestContext, TestHarness,
};
pub use ports::PortAllocator;

pub mod prelude {
    pub use crate::bot::*;
    pub use crate::fixtures::*;
    pub use crate::harness::{
        BotServerInstance, BotUIInstance, Insertable, TestConfig, TestContext, TestHarness,
    };
    pub use crate::mocks::*;
    pub use crate::services::*;

    pub use chrono::{DateTime, Utc};
    pub use serde_json::json;
    pub use tokio;
    pub use uuid::Uuid;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_library_loads() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }
}
