//! Atomic publication of derived results under the durable reservation fence.

use super::{duplicates_handler, metadata_handler, thumbnail_handler};
use crate::core::{
    events::DomainEvent,
    ledger::command::UpdateAssetColorsPayload,
    processing::{ProcessingKind, ProcessingOutcome, ProcessingResult, ProcessingTicket},
    AppError, AppResult,
};
use crate::infra::processing::attempts;
use sqlx::{Sqlite, Transaction};

/// Returns no events when ownership is lost; the caller must roll back that transaction.
///
/// # Errors
/// Rejects mismatched payloads and propagates writes, preserving all-or-nothing completion.
pub(crate) async fn commit(
    transaction: &mut Transaction<'_, Sqlite>,
    ticket: &ProcessingTicket,
    result: ProcessingResult,
) -> AppResult<Option<Vec<DomainEvent>>> {
    if !attempts::is_current(transaction, ticket).await? {
        return Ok(None);
    }
    let mut events = Vec::new();
    let outcome = match (ticket.kind, result) {
        (
            ProcessingKind::Thumbnail,
            ProcessingResult::Thumbnail {
                thumbnail_path,
                preview_path,
                metadata,
                unsupported_reason,
            },
        ) => {
            validate_artifacts(ticket, thumbnail_path.as_deref(), preview_path.as_deref())?;
            if let Some(metadata) = metadata {
                if metadata.asset_id != ticket.asset_id {
                    return Err(invalid_result());
                }
                metadata_handler::handle_update_technical_metadata(transaction, metadata).await?;
                events.push(DomainEvent::AssetMetadataUpdated {
                    asset_id: ticket.asset_id.clone(),
                });
            }
            sqlx::query("UPDATE processing_tasks SET preview_path = ? WHERE asset_id = ? AND kind = 'thumbnail'")
                .bind(preview_path).bind(&ticket.asset_id).execute(&mut **transaction).await?;
            if let Some(path) = thumbnail_path {
                let asset = thumbnail_handler::handle_update_thumbnail(
                    transaction,
                    &ticket.asset_id,
                    &path,
                )
                .await?;
                events.push(DomainEvent::ThumbnailGenerated {
                    asset_id: ticket.asset_id.clone(),
                    path,
                    format: asset.format_type,
                });
                ProcessingOutcome::Completed
            } else {
                ProcessingOutcome::Skipped(unsupported_reason.ok_or_else(invalid_result)?)
            }
        }
        (ProcessingKind::Colors, ProcessingResult::Colors(colors)) => {
            metadata_handler::handle_update_asset_colors(
                transaction,
                UpdateAssetColorsPayload {
                    asset_id: ticket.asset_id.clone(),
                    colors,
                },
            )
            .await?;
            events.push(DomainEvent::ExtractionCompleted {
                asset_id: ticket.asset_id.clone(),
                capability: "COLORS".into(),
            });
            ProcessingOutcome::Completed
        }
        (ProcessingKind::Fingerprint, ProcessingResult::Fingerprint(fingerprint)) => {
            if fingerprint.asset_id != ticket.asset_id {
                return Err(invalid_result());
            }
            duplicates_handler::save_fingerprint_in_transaction(transaction, fingerprint).await?;
            ProcessingOutcome::Completed
        }
        _ => return Err(invalid_result()),
    };
    if !attempts::finish(transaction, ticket, outcome).await? {
        return Ok(None);
    }
    Ok(Some(events))
}

fn invalid_result() -> AppError {
    AppError::ValidationFailed("Processing result does not match its reservation".into())
}

fn validate_artifacts(
    ticket: &ProcessingTicket,
    thumbnail: Option<&str>,
    preview: Option<&str>,
) -> AppResult<()> {
    let stem = format!("{}.{}", ticket.asset_id, ticket.lease_id);
    if thumbnail.is_some_and(|path| path != format!("{stem}.webp"))
        || preview
            .is_some_and(|path| path != format!("{stem}.glb") && path != format!("{stem}.gltf"))
    {
        return Err(invalid_result());
    }
    Ok(())
}
