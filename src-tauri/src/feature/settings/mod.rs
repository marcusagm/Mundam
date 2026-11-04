/// Module for managing application settings.
pub mod service;

pub use service::SettingsService;

#[cfg(test)]
mod processing_tests;
