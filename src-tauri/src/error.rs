//! Centralized error handling for the Mundam backend.
//!
//! This module defines the `AppError` enum, which encapsulates all possible
//! errors that can occur within the application. It uses `thiserror` for
//! idiomatic error definition and implements `serde::Serialize` to allow
//! returning structured error information to the frontend.

use serde::{Serialize, Serializer};
use thiserror::Error;

/// The primary error type for the application.
#[derive(Debug, Error)]
pub enum AppError {
    /// Error related to database operations.
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),

    /// Error related to database migrations.
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Error related to Tauri framework operations.
    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    /// Error related to filesystem operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error related to image processing or transcoding.
    #[error("Transcoding error: {0}")]
    Transcoding(String),

    /// Error when a resource (file, folder, tag) is not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Error related to application configuration or state.
    #[error("Internal state error: {0}")]
    Internal(String),

    /// Generic error with a custom message.
    #[error("Error: {0}")]
    Generic(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // For now, we serialize the error as its string representation.
        // In the future, we could serialize a structured object with an error code.
        serializer.serialize_str(&self.to_string())
    }
}

/// A specialized `Result` type for Mundam backend operations.
pub type AppResult<T> = Result<T, AppError>;
