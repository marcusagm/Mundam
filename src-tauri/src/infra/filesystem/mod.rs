//! Platform adapters for generated caches, exclusive original-media moves and recovery storage.

pub mod asset_cache;
pub mod plugin_previews;

/// Exclusive native moves for managed library edits.
pub(crate) mod exclusive_move;

/// Explicit access to app-owned folder recovery storage.
pub(crate) mod folder_recovery;

/// Stable native identities for originals and recovery plans.
pub(crate) mod identity;

/// Temporary storage used by the staged import coordinator and Ledger.
pub(crate) mod prepared_imports;

/// Handle-anchored publication and compensation for prepared imports.
pub(crate) mod import_move;
