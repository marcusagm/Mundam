//! Registry of filesystem paths currently being modified by the application itself.
//!
//! The Ledger performs physical file moves (trash / restore) as post-commit
//! Saga steps. The filesystem watcher cannot tell those moves apart from
//! external changes and would report them later (after its debounce delay) as
//! deletions or creations, racing with the already-committed database state.
//! Paths registered here are ignored by the watcher-driven indexer.

use dashmap::DashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use unicode_normalization::UnicodeNormalization;

/// How long a registered path keeps suppressing watcher events. It must exceed
/// the watcher pipeline latency (deletion guard + debounce tick + indexer delay).
const SUPPRESSION_WINDOW: Duration = Duration::from_secs(15);

/// Thread-safe, time-bounded set of paths owned by the Ledger.
///
/// Keys are NFC-normalized: on macOS the same file may be reported as NFC or NFD
/// (accented names), so comparisons must be insensitive to the normalization form.
#[derive(Debug, Default)]
pub struct LedgerPathGuard {
    entries: DashMap<String, Instant>,
}

fn normalized_key(path: &Path) -> String {
    path.to_string_lossy().nfc().collect()
}

impl LedgerPathGuard {
    /// Creates an empty guard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers (or refreshes) a path as being modified by the application.
    pub fn mark(&self, path: &Path) {
        self.entries.retain(|_, marked_at| marked_at.elapsed() < SUPPRESSION_WINDOW);
        self.entries.insert(normalized_key(path), Instant::now());
    }

    /// Returns `true` if the path was registered within the suppression window.
    pub fn is_marked(&self, path: &Path) -> bool {
        self.entries
            .get(&normalized_key(path))
            .map(|marked_at| marked_at.elapsed() < SUPPRESSION_WINDOW)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn marked_path_is_reported() {
        let guard = LedgerPathGuard::new();
        let path = PathBuf::from("/library/image.png");
        assert!(!guard.is_marked(&path));
        guard.mark(&path);
        assert!(guard.is_marked(&path));
        assert!(!guard.is_marked(Path::new("/library/other.png")));
    }

    #[test]
    fn matches_across_unicode_normalization_forms() {
        let guard = LedgerPathGuard::new();
        let composed = PathBuf::from("/library/caf\u{e9}.png");
        let decomposed = PathBuf::from("/library/cafe\u{301}.png");
        guard.mark(&decomposed);
        assert!(guard.is_marked(&composed));
    }
}
