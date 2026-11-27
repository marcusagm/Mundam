//! SQLite composition: repository facades delegate projections and transactional mutations.
//!
//! Query handlers contain read models. Mutation handlers preserve the existing transaction
//! and Ledger boundaries; shared selection policy keeps admission and queue counts aligned.

pub mod duplicates;
pub mod handlers;
pub mod import_catalog;
pub mod import_destinations;
pub mod import_pairing;
pub mod import_staging;
pub mod ledger;
pub(crate) mod library_operations;
pub mod manager;
pub mod models;
mod plugin_asset_data;
mod plugin_data_management;
mod plugin_derived_data;
pub mod plugin_export;
pub mod plugins;
pub mod processing;
pub mod queries;
pub mod query_handlers;
pub mod saga_recovery;
pub mod search_builder;
