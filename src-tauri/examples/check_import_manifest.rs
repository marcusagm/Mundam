//! Offline preflight for the public request example. Uses an explicit demonstration quota.
use mundam_lib::core::imports::{ImportLimits, ImportManifest};
use std::error::Error;
use std::io::{self, Read};

fn main() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
    io::stdin()
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("Manifest exceeds the 1 MiB demonstration limit".into());
    }
    let manifest: ImportManifest = serde_json::from_slice(&bytes)?;
    let limits = ImportLimits {
        max_items: 100,
        max_tags_per_item: 100,
        max_item_bytes: 1 << 30,
        max_batch_bytes: 4 << 30,
    };
    serde_json::to_writer_pretty(io::stdout().lock(), &manifest.normalize(&limits)?)?;
    Ok(())
}
