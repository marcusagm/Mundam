//! Background executors share one entry point for bootstrap, lifecycle and processing controls.
//!
//! Feature modules retain domain algorithms and commands; workers own continuous consumption.

pub mod auto_empty_trash_worker;
pub mod color_worker;
pub mod duplicate_worker;
pub mod thumbnail_worker;

pub use auto_empty_trash_worker::AutoEmptyTrashWorker;
pub use color_worker::ColorWorker;
pub use duplicate_worker::DuplicateWorker;
pub use thumbnail_worker::ThumbnailWorker;

#[cfg(test)]
pub(crate) mod test_support;
