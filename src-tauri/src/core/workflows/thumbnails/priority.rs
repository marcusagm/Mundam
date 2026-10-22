use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

/// State management for thumbnail generation priority.
///
/// Under Hexagonal Architecture, this resides in the Core Domain (Workflows)
/// as it defines the business logic for how assets should be prioritized
/// when requested by the UI.
pub struct ThumbnailPriorityState {
    /// Internal queue of asset IDs that should be processed with priority.
    /// Holds the IDs of the latest UI viewport, in the order they should be processed.
    priority_ids: Mutex<VecDeque<String>>,
}

/// Default implementation for `ThumbnailPriorityState`.
impl Default for ThumbnailPriorityState {
    /// Creates a new instance of `ThumbnailPriorityState` with an empty priority queue.
    ///
    /// # Returns
    ///
    /// A new instance of `ThumbnailPriorityState`.
    fn default() -> Self {
        Self {
            priority_ids: Mutex::new(VecDeque::new()),
        }
    }
}

/// Implementation of `ThumbnailPriorityState`.
impl ThumbnailPriorityState {
    /// Replaces the priority queue with the IDs currently visible in the UI.
    ///
    /// Each call reflects the latest viewport, so IDs from previous viewports
    /// (already scrolled away) are discarded instead of competing with the new ones.
    /// The first ID is served first.
    ///
    /// # Arguments
    /// * `ids` - Vector of unique asset IDs (UUIDs), ordered from the top of the viewport.
    pub fn push_priorities(&self, ids: Vec<String>) {
        if ids.is_empty() {
            return;
        }

        if let Ok(mut deque) = self.priority_ids.lock() {
            deque.clear();
            deque.extend(ids.into_iter().take(200));
        }
    }

    /// Extracts a batch of unique IDs from the priority queue.
    ///
    /// Useful for the `ThumbnailWorker` to get a chunk of work.
    ///
    /// # Arguments
    /// * `batch_size` - Maximum number of IDs to extract.
    pub fn pop_batch(&self, batch_size: usize) -> Vec<String> {
        let mut batch = Vec::new();
        let mut seen = HashSet::new();

        if let Ok(mut deque) = self.priority_ids.lock() {
            while batch.len() < batch_size {
                if let Some(id) = deque.pop_front() {
                    // Avoid duplicate processing in the same batch
                    if seen.insert(id.clone()) {
                        batch.push(id);
                    }
                } else {
                    break;
                }
            }
        }
        batch
    }

    /// Returns the current number of items in the priority queue.
    pub fn len(&self) -> usize {
        self.priority_ids.lock().map(|d| d.len()).unwrap_or(0)
    }

    /// Checks if the priority queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Tests for the `ThumbnailPriorityState` struct.
#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the LIFO behavior of the priority queue.
    #[test]
    fn test_lifo_behavior() {
        let state = ThumbnailPriorityState::default();
        state.push_priorities(vec!["1".to_string(), "2".to_string()]);

        // The first ID of a batch is the top of the viewport and is served first.
        let batch = state.pop_batch(2);
        assert_eq!(batch, vec!["1".to_string(), "2".to_string()]);

        // A newer batch replaces older pending IDs (viewport moved).
        state.push_priorities(vec!["3".to_string()]);
        state.push_priorities(vec!["4".to_string()]);
        assert_eq!(state.pop_batch(2), vec!["4".to_string()]);
    }

    /// Tests the batch limit of the priority queue.
    #[test]
    fn test_batch_limit() {
        let state = ThumbnailPriorityState::default();
        state.push_priorities(vec!["1".to_string(), "2".to_string(), "3".to_string()]);

        let batch = state.pop_batch(2);
        assert_eq!(batch.len(), 2);
        assert_eq!(state.len(), 1);
    }

    /// Tests the deduplication of the priority queue.
    #[test]
    fn test_deduplication() {
        let state = ThumbnailPriorityState::default();
        // Push duplicate within the same operation (less common but possible)
        state.push_priorities(vec!["1".to_string(), "1".to_string()]);

        let batch = state.pop_batch(10);
        assert_eq!(batch, vec!["1".to_string()]);
    }
}
