//! Prepared native effects remain separate from SQL mutations and filesystem execution.
use std::path::PathBuf;
/// A transaction's physical changes, reversed if a later step fails.
#[derive(Default)]
pub(crate) struct DiskChanges {
    pub(crate) moves: Vec<(PathBuf, PathBuf)>,
    pub(crate) create: Option<PathBuf>,
    pub(crate) remove_empty: Option<PathBuf>,
    pub(crate) removed_asset_ids: std::collections::HashSet<String>,
    pub(crate) restored_assets: Vec<(String, String, String)>,
    /// Logical byte estimate measured before acquiring the SQLite writer.
    pub(crate) archive_bytes: Option<i64>,
    /// Trusted runtime quota loaded before catalog preparation.
    pub(crate) archive_budget: i64,
}

impl DiskChanges {
    /// Describes public effects without leaking temporary case-rename stages.
    pub(crate) fn confirmed_effects(&self) -> Vec<crate::core::library::LibraryPathEffect> {
        use crate::core::library::LibraryPathEffect;
        let mut effects: Vec<LibraryPathEffect> = Vec::new();
        for (source, destination) in &self.moves {
            if let Some(previous) = effects
                .iter_mut()
                .find(|effect| effect.destination.as_ref() == Some(source))
            {
                previous.destination = Some(destination.clone());
            } else {
                effects.push(LibraryPathEffect {
                    source: Some(source.clone()),
                    destination: Some(destination.clone()),
                });
            }
        }
        if let Some(path) = &self.create {
            effects.push(LibraryPathEffect {
                source: None,
                destination: Some(path.clone()),
            });
        }
        if let Some(path) = &self.remove_empty {
            effects.push(LibraryPathEffect {
                source: Some(path.clone()),
                destination: None,
            });
        }
        effects
    }
}
