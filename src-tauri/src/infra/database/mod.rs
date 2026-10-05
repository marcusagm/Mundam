pub mod handlers;
/// Module for database operations.
///
/// This module contains the implementation of the database operations.
///
/// # Submodules
///
/// * `ledger` - The ledger operations.
/// * `manager` - The database manager.
/// * `models` - The database models.
/// * `queries` - The database queries.
/// * `search_builder` - The search builder.
///
/// # Exports
///
/// * `ledger::LedgerRepository` - The ledger repository.
/// * `manager::DatabaseManager` - The database manager.
/// * `models::AssetDb` - The asset database model.
/// * `models::FolderDb` - The folder database model.
/// * `models::TagDb` - The tag database model.
/// * `queries::AssetQueryHandler` - The asset query handler.
/// * `search_builder::SearchBuilder` - The search builder.
///
/// # Example
///
/// ```rust
/// use crate::infra::database::DatabaseManager;
/// use crate::infra::database::AssetQueryHandler;
/// use crate::infra::database::SearchBuilder;
/// use crate::infra::database::LedgerRepository;
/// use crate::infra::database::models::AssetDb;
/// use crate::infra::database::models::FolderDb;
/// use crate::infra::database::models::TagDb;
/// use crate::core::models::AssetSummaryDto;
/// use crate::core::models::Folder;
/// use crate::core::models::Tag;
/// use crate::core::models::AppResult;
/// use std::sync::Arc;
///
/// let database_manager = DatabaseManager::new();
/// let asset_query_handler = AssetQueryHandler::new(database_manager.clone());
/// let search_builder = SearchBuilder::new(database_manager.clone());
/// let ledger_repository = LedgerRepository::new(database_manager.clone());
/// let asset_db = AssetDb::new();
/// let folder_db = FolderDb::new();
/// let tag_db = TagDb::new();
/// let asset_summary_dto = AssetSummaryDto::new();
/// let folder = Folder::new();
/// let tag = Tag::new();
/// let app_result = AppResult::new();
/// ```
pub mod ledger;
pub mod manager;
pub mod models;
pub mod queries;
pub mod query_handlers;
pub mod duplicates;
pub mod saga_recovery;
pub mod search_builder;
