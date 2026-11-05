//! # Asset Ledger Module
//!
//! The Ledger is the core transactional authority in Mundam.
//! It manages the "Write" side of CQRS, ensuring that any change to an Asset
//! is atomic, valid, and audited.

pub mod command;
pub mod library_operation;
pub mod mock;
pub mod port;

pub use command::LedgerCommand;
pub use port::TransactionalAssetLedger;

/// Pure taxonomy import validation.
pub mod tag_validation;
