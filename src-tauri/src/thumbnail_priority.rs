use std::collections::HashSet;
use std::sync::Mutex;
use tauri::State;

pub struct ThumbnailPriorityState {
    pub priority_ids: Mutex<HashSet<i64>>,
}

impl Default for ThumbnailPriorityState {
    fn default() -> Self {
        Self {
            priority_ids: Mutex::new(HashSet::new()),
        }
    }
}

impl ThumbnailPriorityState {
    pub fn set_priority(&self, ids: Vec<i64>) {
        if let Ok(mut set) = self.priority_ids.lock() {
            set.clear();
            for id in ids {
                set.insert(id);
            }
        }
    }

    pub fn get_priority(&self) -> Vec<i64> {
        if let Ok(set) = self.priority_ids.lock() {
            set.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn clear(&self) {
        if let Ok(mut set) = self.priority_ids.lock() {
             set.clear();
        }
    }
}
