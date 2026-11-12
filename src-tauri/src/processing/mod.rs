//! Módulo de processamento de mídia

/// Heartbeat lifecycle for durable processing reservations.
pub mod lease;
pub mod media;
mod shutdown_diagnostics;
pub mod transcoding;
pub mod watcher;
pub mod workers;
