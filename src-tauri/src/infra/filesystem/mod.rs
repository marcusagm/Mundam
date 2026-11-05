//! Filesystem adapters for generated artifacts; original asset media is excluded.

pub mod asset_cache;

/// Exclusive native moves for managed library edits.
pub(crate) mod exclusive_move;

/// Explicit access to app-owned folder recovery storage.
pub(crate) mod folder_recovery;
