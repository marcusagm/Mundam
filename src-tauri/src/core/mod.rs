//! Module containing core functionality for the application.

pub mod error;
pub mod events;
pub mod formats;

pub use error::{AppError, AppResult, Context};
pub use events::{AppEventBus, DomainEvent};

pub mod ledger;
pub mod models;
pub mod path_guard;
pub mod processing;
pub mod repository;
pub mod settings;
pub mod trash;
pub mod workflows;

pub use ledger::{LedgerCommand, TransactionalAssetLedger};
