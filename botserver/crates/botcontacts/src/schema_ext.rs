//! #1441 — diesel definitions this crate needs but that do not belong in
//! `schema.rs`: the `users` identity table, referenced by the CRM owner
//! columns (`crm_deals.owner_id`, `crm_contacts.owner_id`) so grid fragments
//! can render an owner label instead of a bare UUID.
//!
//! `schema.rs` is already at the 450-line budget ceiling, so this lives in its
//! own module. Columns mirror `6.5.15.1-consolidated`
//! (`CREATE TABLE users`); CRM only ever reads `id`/`email` here — writes stay
//! with the auth crate.

pub use botschema::users;
