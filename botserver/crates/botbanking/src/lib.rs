pub mod platform;
pub mod transaction;
pub mod reconcile;
pub mod billing;
pub mod dashboard;
pub mod cashflow;
pub mod db;
pub mod storage;
pub mod handlers;
pub mod accounts_pix;
pub mod routes;
pub mod intent;

pub use intent::{
    BankTransactionImport, BankTransactionRow, BankingRequest, BankingRequestKind,
    DeliveryMarginsSummary, DeliveryTransactionImport, DeliveryTransactionRow, PlatformRevenue,
    ReconciliationRuleRequest, ReconciliationSummary, RevenueByPlatformReport,
    UnmatchedDeliveriesQuery, UnmatchedFilter,
};

pub use routes::configure;
pub use platform::{DeliveryPlatform, PlatformConnector, ConnectorKind, ConnectorError};
pub use transaction::{BankTransaction, TransactionKind, TransactionSide, ReconcileStatus};
pub use reconcile::{
    Reconciliation, ReconciliationMatch, MatchConfidence, ReconcileError,
    match_transactions,
};
pub use billing::{BillingEntry, BillingKind, BillingPeriod};
pub use dashboard::{BillingDashboard, DashboardSummary, DashboardKpi};
