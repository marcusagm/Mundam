//! Continuous executors grouped by responsibility; features retain application algorithms.

pub mod colors;
pub mod duplicates;
pub mod thumbnails;
pub mod tools;
pub mod trash;

pub use colors::ColorWorker;
pub use duplicates::DuplicateWorker;
pub use thumbnails::ThumbnailWorker;
pub use trash::AutoEmptyTrashWorker;

#[cfg(test)]
pub(crate) mod test_support;
