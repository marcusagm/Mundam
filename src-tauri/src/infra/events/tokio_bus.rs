use async_trait::async_trait;
use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::core::error::AppResult;
use crate::core::events::bus::AppEventBus;
use crate::core::events::payloads::DomainEvent;

/// Default capacity configuration for the broadcast channel.
///
/// 2048 events is a safe value for bursts in a desktop DAM.
/// If the backlog overflows, receivers will receive a `Lagged` error.
const EVENT_BUS_CAPACITY: usize = 2048;

/// Concrete Event Bus adapter using `tokio::sync::broadcast`.
///
/// This implementation provides a thread-safe and high-performance bus.
pub struct TokioEventBus {
    sender: broadcast::Sender<DomainEvent>,
}

/// Implementation of the TokioEventBus struct.
impl TokioEventBus {
    /// Creates a new TokioEventBus instance with default capacity.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BUS_CAPACITY);
        debug!(
            "TokioEventBus initialized with capacity of {}",
            EVENT_BUS_CAPACITY
        );
        Self { sender }
    }
}

impl Default for TokioEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of the AppEventBus trait for TokioEventBus.
#[async_trait]
impl AppEventBus for TokioEventBus {
    /// Dispatches an event to all logged-in listeners.
    ///
    /// # Arguments
    /// * `event` - The domain event to be published.
    ///
    /// # Errors
    /// Returns `AppError::EventBus` in case of critical publication failure.
    fn publish(&self, event: DomainEvent) -> AppResult<()> {
        // Broadcast returns the number of active receivers.
        // If it returns 0, it means no one is currently listening.
        match self.sender.send(event) {
            Ok(receiver_count) => {
                debug!(
                    "Event published successfully to {} subscribers",
                    receiver_count
                );
                Ok(())
            }
            Err(send_error) => {
                // In broadcast context, SendError occurs if all receivers were dropped.
                // Since we don't guarantee persistence, we just log a warning.
                warn!(
                    "Failed to publish event: No active subscribers. Details: {:?}",
                    send_error
                );
                Ok(()) // We don't return an error as the Bus should be resilient to missing listeners
            }
        }
    }

    /// Subscribes to receive an event stream.
    ///
    /// Returns a Receiver that can be used in an asynchronous loop.
    /// Note: Due to the nature of `tokio::sync::broadcast`, the receiver
    /// may throw a `Lagged` error if the consumer is slower than the producer.
    fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }
}

/// Unit tests for the TokioEventBus.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Test that the event bus can send events to multiple subscribers.
    #[tokio::test]
    async fn test_fan_out_multiple_subscribers() -> Result<(), Box<dyn std::error::Error>> {
        let event_bus = Arc::new(TokioEventBus::new());
        let mut receiver_1 = event_bus.subscribe();
        let mut receiver_2 = event_bus.subscribe();

        let event = DomainEvent::ScanStarted {
            library_id: "test_lib".to_string(),
        };

        event_bus.publish(event)?;

        let received_1 = receiver_1.recv().await?;
        let received_2 = receiver_2.recv().await?;

        match (received_1, received_2) {
            (
                DomainEvent::ScanStarted { library_id: id1 },
                DomainEvent::ScanStarted { library_id: id2 },
            ) => {
                assert_eq!(id1, "test_lib");
                assert_eq!(id2, "test_lib");
            }
            _ => panic!("Received events do not match expected events"),
        }

        Ok(())
    }

    /// Test that the event bus can handle backpressure.
    #[tokio::test]
    async fn test_backpressure_lagged_error() -> Result<(), Box<dyn std::error::Error>> {
        // We use a tiny capacity for the test
        let (sender, mut receiver) = broadcast::channel(1);

        sender.send(DomainEvent::FsPathDeleted {
            path: "1".to_string(),
        })?;
        sender.send(DomainEvent::FsPathDeleted {
            path: "2".to_string(),
        })?;
        sender.send(DomainEvent::FsPathDeleted {
            path: "3".to_string(),
        })?;

        // The first recv should fail with Lagged because we skipped 2 messages
        let result = receiver.recv().await;
        assert!(result.is_err());
        match result.err().ok_or("Expected an error result")? {
            broadcast::error::RecvError::Lagged(skipped) => assert_eq!(skipped, 2),
            _ => panic!("Should have returned Lagged error"),
        }

        Ok(())
    }
}
