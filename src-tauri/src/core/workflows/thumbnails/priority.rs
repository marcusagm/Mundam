use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

/// State management for thumbnail generation priority.
///
/// Under Hexagonal Architecture, this resides in the Core Domain (Workflows)
/// as it defines the business logic for how assets should be prioritized
/// when requested by the UI.
pub struct ThumbnailPriorityState {
    /// Internal queue of asset IDs that should be processed with priority.
    /// Uses a `VecDeque` to support LIFO behavior (push_front/pop_front).
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
    /// Adds a list of asset IDs to the priority queue.
    ///
    /// New IDs are pushed to the front (LIFO) because they usually represent
    /// items that just scrolled into the user's viewport.
    ///
    /// # Arguments
    /// * `ids` - Vector of unique asset IDs (UUIDs).
    pub fn push_priorities(&self, ids: Vec<String>) {
        if ids.is_empty() {
            return;
        }

        if let Ok(mut deque) = self.priority_ids.lock() {
            // To maintain LIFO for the batch, we push them in reverse order.
            // If the UI sends [A, B, C] (where A is top of viewport),
            // pushing reversed (C, then B, then A) puts A at the very front of the queue,
            // so pop_front will yield A, then B, then C.
            for id in ids.into_iter().rev() {
                // If it's already in the queue, we can leave it to be skipped,
                // but for performance, we just push to the front. 
                // Deduplication happens at pop_batch time.
                deque.push_front(id);
            }

            // Safety limit: if the queue gets too large (e.g., user scrolling fast),
            // truncate it to avoid excessive memory usage or stale priorities.
            if deque.len() > 200 {
                deque.truncate(200);
            }
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

        // A newer batch takes precedence over older pending IDs.
        state.push_priorities(vec!["3".to_string()]);
        state.push_priorities(vec!["4".to_string()]);
        assert_eq!(state.pop_batch(2), vec!["4".to_string(), "3".to_string()]);
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
