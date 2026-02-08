//! HLS On-the-Fly Streaming Module
//!
//! Provides real-time video transcoding via HLS protocol.
//! Segments are generated on-demand and cached to disk.

pub mod server;
pub mod probe;
pub mod playlist;
pub mod segment;
pub mod process_manager;

pub use server::StreamingServer;
