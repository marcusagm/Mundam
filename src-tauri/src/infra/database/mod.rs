//! SQLite composition: repository facades delegate projections and transactional mutations.
//!
//! Query handlers contain read models. Mutation handlers preserve the existing transaction
//! and Ledger boundaries; shared selection policy keeps admission and queue counts aligned.

pub mod duplicates;
pub mod handlers;
pub mod ledger;
mod library_operations;
pub mod manager;
pub mod models;
pub mod processing;
pub mod queries;
pub mod query_handlers;
pub mod saga_recovery;
pub mod search_builder;
