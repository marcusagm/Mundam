//! Read-only file extraction shared by automatic and manually requested duplicate processing.

use crate::core::{
    formats::FormatRegistry,
    models::{Asset, DuplicateFingerprint},
    AppError, AppResult,
};
use crate::feature::duplicates::fingerprints::{
    compute_multiscale_dhash_256, compute_perceptual_dhash_64, compute_spatial_block_hash,
    is_image_media, resolve_image_via_format_provider,
};
use image::GenericImageView;
use std::{
    fs::File,
    io::Read,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};
use tokio_util::sync::CancellationToken;

/// Version five hashes retain their existing mathematical representation.
pub const FINGERPRINT_VERSION: i32 = 5;

/// Performs file reads and provider calls off the async executor.
///
/// # Errors
/// Returns IO errors or a blocking-task failure. Cancellation returns no result.
pub async fn generate(
    asset: Asset,
    registry: Arc<FormatRegistry>,
    token: CancellationToken,
) -> AppResult<Option<DuplicateFingerprint>> {
    let phase = Arc::new(AtomicU8::new(0));
    let worker_phase = phase.clone();
    let worker_token = token.clone();
    let task = tokio::task::spawn_blocking(move || {
        generate_blocking(asset, &registry, &worker_token, &worker_phase)
    });
    let result = crate::processing::shutdown_diagnostics::observe(
        &token,
        &phase,
        fingerprint_phase,
        "fingerprint",
        task,
    )
    .await;
    result.map_err(|error| AppError::Internal(format!("Fingerprint task failed: {error}")))?
}

fn fingerprint_phase(phase: u8) -> &'static str {
    match phase {
        1 => "content_hash",
        2 => "image_decode",
        3 => "provider_extraction",
        4 => "visual_hash",
        _ => "starting",
    }
}

fn generate_blocking(
    asset: Asset,
    registry: &FormatRegistry,
    token: &CancellationToken,
    phase: &AtomicU8,
) -> AppResult<Option<DuplicateFingerprint>> {
    if token.is_cancelled() {
        return Ok(None);
    }
    phase.store(1, Ordering::Relaxed);
    let mut file = File::open(&asset.path)?;
    let metadata = file.metadata()?;
    let Some(content_hash) = hash_reader(&mut file, token)? else {
        return Ok(None);
    };
    let mut fingerprint = DuplicateFingerprint {
        asset_id: asset.id.clone(),
        content_hash: Some(content_hash),
        file_size: Some(metadata.len() as i64),
        format_family: Some(asset.family.clone()),
        fingerprint_version: FINGERPRINT_VERSION,
        perceptual_hash: None,
        block_hash: None,
        thumb_hash: None,
        width: None,
        height: None,
        mime_type: None,
        color_profile: None,
        orientation: None,
        updated_at: chrono::Utc::now(),
    };
    if token.is_cancelled() {
        return Ok(None);
    }
    if is_image_media(&asset.family, &asset.format_type) {
        phase.store(2, Ordering::Relaxed);
        let image = image::open(&asset.path).ok().or_else(|| {
            if token.is_cancelled() {
                None
            } else {
                phase.store(3, Ordering::Relaxed);
                resolve_image_via_format_provider(registry, &asset.path, &asset.id, token)
            }
        });
        if token.is_cancelled() {
            return Ok(None);
        }
        if let Some(image) = image {
            phase.store(4, Ordering::Relaxed);
            let (width, height) = image.dimensions();
            fingerprint.perceptual_hash =
                Some(format!("{:016x}", compute_perceptual_dhash_64(&image)));
            fingerprint.block_hash = Some(compute_spatial_block_hash(&image));
            fingerprint.thumb_hash = Some(compute_multiscale_dhash_256(&image));
            fingerprint.width = Some(width as i32);
            fingerprint.height = Some(height as i32);
        }
    }
    if token.is_cancelled() {
        return Ok(None);
    }
    Ok(Some(fingerprint))
}

fn hash_reader(reader: &mut impl Read, token: &CancellationToken) -> AppResult<Option<String>> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 8192];
    loop {
        if token.is_cancelled() {
            return Ok(None);
        }
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    if token.is_cancelled() {
        return Ok(None);
    }
    Ok(Some(hasher.finalize().to_hex().to_string()))
}

#[cfg(test)]
#[path = "fingerprint_tests.rs"]
mod tests;
