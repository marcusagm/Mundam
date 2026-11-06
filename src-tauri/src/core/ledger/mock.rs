use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use super::command::LedgerCommand;
use super::port::TransactionalAssetLedger;
use crate::core::error::{AppError, AppResult};
use crate::core::events::{AppEventBus, DomainEvent};
use crate::core::models::asset::Asset;

/// In-memory implementation of the Asset Ledger for testing and rapid prototyping.
///
/// It uses a `HashMap` to simulate the database and an injected `AppEventBus`
/// to publish domain events.
pub struct MockAssetLedger {
    assets: RwLock<HashMap<String, Asset>>,
    event_bus: Arc<dyn AppEventBus>,
}

/// Implementation of the MockAssetLedger.
impl MockAssetLedger {
    /// Creates a new instance of the Mock Ledger.
    ///
    /// # Arguments
    ///
    /// * `event_bus` - The event bus to publish events to.
    ///
    /// # Returns
    ///
    /// * `Self` - The new instance of the Mock Ledger.
    pub fn new(event_bus: Arc<dyn AppEventBus>) -> Self {
        Self {
            assets: RwLock::new(HashMap::new()),
            event_bus,
        }
    }
}

/// Implements the TransactionalAssetLedger trait for the MockAssetLedger.
#[async_trait]
impl TransactionalAssetLedger for MockAssetLedger {
    /// Executes a command on the Mock Ledger.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute.
    ///
    /// # Returns
    ///
    /// * `AppResult<Asset>` - The result of the command execution.
    async fn execute(&self, command: LedgerCommand) -> AppResult<Asset> {
        match command {
            LedgerCommand::CreateAsset(payload) => {
                let asset_id = Uuid::new_v4().to_string();
                let now = Utc::now();

                let asset = Asset {
                    id: asset_id.clone(),
                    name: payload
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    path: payload.path.clone(),
                    state: payload.state_init,
                    format_type: payload.format_type.clone(),
                    family: payload.family.clone(),
                    file_size: payload.file_size,
                    created_at: Some(now),
                    modified_at: Some(now),
                    added_at: Some(now),
                    updated_at: Some(now),
                    width: None,
                    height: None,
                    duration_secs: None,
                    technical_payload: None,
                    semantic_payload: None,
                    dominant_color: None,
                    folder_id: payload.folder_id.clone(),
                    thumbnail_path: None,
                    rating: None,
                    notes: None,
                    is_favorite: false,
                    deleted_at: None,
                };

                // Store in memory
                {
                    let mut assets = self.assets.write().map_err(|_| {
                        AppError::Internal(
                            "Failed to acquire write lock on Mock Ledger".to_string(),
                        )
                    })?;
                    assets.insert(asset_id.clone(), asset.clone());
                }

                // Publish Event
                self.event_bus.publish(DomainEvent::AssetCreated {
                    asset_id,
                    path: payload.path.to_string_lossy().to_string(),
                    format: payload.format_type,
                })?;

                Ok(asset)
            }
            LedgerCommand::UpdateTags(payload) => {
                let assets = self.assets.read().map_err(|_| {
                    AppError::Internal("Failed to acquire read lock on Mock Ledger".to_string())
                })?;

                let _asset = assets
                    .get(&payload.asset_id)
                    .ok_or_else(|| AppError::NotFound(payload.asset_id.clone()))?;

                // For the mock in this sprint, we just simulate the event
                self.event_bus.publish(DomainEvent::AssetTagsUpdated {
                    asset_id: payload.asset_id.clone(),
                    active_tags: payload.tags_to_add, // Simplified for mock
                })?;

                // Return the asset (cloned for the mock)
                Ok(_asset.clone())
            }
            LedgerCommand::CreateTag(payload) => {
                let tag_id = Uuid::new_v4().to_string();
                self.event_bus.publish(DomainEvent::TagCreated {
                    id: tag_id.clone(),
                    name: payload.name.clone(),
                })?;
                Ok(Asset {
                    id: tag_id,
                    name: payload.name,
                    path: std::path::PathBuf::new(),
                    state: crate::core::models::asset::AssetState::Idle,
                    format_type: "tag".to_string(),
                    family: "TAG".to_string(),
                    file_size: 0,
                    created_at: Some(Utc::now()),
                    modified_at: Some(Utc::now()),
                    added_at: Some(Utc::now()),
                    updated_at: Some(Utc::now()),
                    width: None,
                    height: None,
                    duration_secs: None,
                    technical_payload: None,
                    semantic_payload: None,
                    dominant_color: None,
                    folder_id: None,
                    thumbnail_path: None,
                    rating: None,
                    notes: None,
                    is_favorite: false,
                    deleted_at: None,
                })
            }
            _ => Err(AppError::Internal(
                "Command not implemented in Mock".to_string(),
            )),
        }
    }
}

/// Tests for the MockAssetLedger.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::AppResult;
    use crate::core::events::bus::AppEventBus;
    use crate::core::events::DomainEvent;
    use crate::core::ledger::command::CreateAssetPayload;
    use crate::core::models::asset::AssetState;
    use std::path::PathBuf;
    use tokio::sync::broadcast;

    // Minimal implementation of EventBus for testing
    struct TestEventBus {
        sender: broadcast::Sender<DomainEvent>,
    }

    /// Creates a new instance of the TestEventBus.
    impl TestEventBus {
        fn new() -> Self {
            let (sender, _) = broadcast::channel(16);
            Self { sender }
        }
    }

    /// Implements the AppEventBus trait for the TestEventBus.
    #[async_trait]
    impl AppEventBus for TestEventBus {
        fn publish(&self, event: DomainEvent) -> AppResult<()> {
            let _ = self.sender.send(event);
            Ok(())
        }
        fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
            self.sender.subscribe()
        }
    }

    /// Tests the creation of an asset in the MockAssetLedger.
    #[tokio::test]
    async fn test_mock_create_asset_emits_event() {
        let bus = Arc::new(TestEventBus::new());
        let mut receiver = bus.subscribe();
        let ledger = MockAssetLedger::new(bus.clone());

        let payload = CreateAssetPayload {
            path: PathBuf::from("/tmp/test.jpg"),
            file_size: 1024,
            format_type: "image/jpeg".to_string(),
            family: "IMAGE".to_string(),
            state_init: AssetState::Discovered,
            folder_id: None,
            created_at: None,
            modified_at: None,
        };

        let result = ledger.execute(LedgerCommand::CreateAsset(payload)).await;
        assert!(result.is_ok());
        let asset = result.unwrap();

        // Verify state
        assert_eq!(asset.name, "test.jpg");
        assert_eq!(asset.state, AssetState::Discovered);

        // Verify Event
        let event = receiver.try_recv().expect("Should have received an event");
        if let DomainEvent::AssetCreated { asset_id, path, .. } = event {
            assert_eq!(asset_id, asset.id);
            assert_eq!(path, "/tmp/test.jpg");
        } else {
            panic!("Expected DomainEvent::AssetCreated");
        }
    }
}
