//! HTTP surface for botfraud.
//!
//! Split from the former monolithic `handlers.rs`. The public surface is
//! unchanged: `botfraud::handlers::{DbPool, FraudState, configure_fraud_routes,
//! FraudTransaction}` all still resolve.

mod rows;
mod rule_handlers;
mod routes;
mod state;
mod stats;
mod transactions;

pub use routes::configure_fraud_routes;
pub use state::{DbPool, FraudState};
pub use transactions::FraudTransaction;
