//! Clip Studio Paint (.clip) preview extractor.
//!
//! This module implements a parser for the CLIP file format (CSFCHUNK) to locate
//! the internal SQLite database (CHNKSQLi) and extract the rendered canvas preview.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use byteorder::{BigEndian, ReadBytesExt};
use sqlx::sqlite::SqlitePoolOptions;

/// Magic bytes for CLIP Studio Paint files.
const CLIP_MAGIC: &[u8; 8] = b"CSFCHUNK";

/// Chunk identifier for the internal SQLite database.
const SQL_CHUNK_NAME: &[u8; 8] = b"CHNKSQLi";

/// Chunk identifier that marks the end of the file.
const FOOTER_CHUNK_NAME: &[u8; 8] = b"CHNKFoot";

/// Errors that can occur during CLIP parsing.
#[derive(Debug, thiserror::Error)]
pub enum ClipError {
    /// The file does not have the expected CSFCHUNK magic bytes.
    #[error("Invalid CLIP format: missing CSFCHUNK magic")]
    InvalidFormat,

    /// The required CHNKSQLi chunk (SQLite database) was not found.
    #[error("CLIP missing SQLite chunk (CHNKSQLi)")]
    MissingSqlChunk,

    /// Multiple previews found or database structure is unexpected.
    #[error("Database error during preview extraction: {0}")]
    DatabaseError(String),

    /// IO error during parsing.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// SQLx error during extraction.
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

/// Extracts the preview PNG from a .clip file by identifying the internal SQLite database.
pub fn extract_clip_preview(path: &Path) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;

    // 1. Verify Header
    let mut magic_buffer = [0u8; 8];
    file.read_exact(&mut magic_buffer)?;
    if &magic_buffer != CLIP_MAGIC {
        return Err(Box::new(ClipError::InvalidFormat));
    }

    // Skip file length (8 bytes) and offset information (8 bytes)
    file.seek(SeekFrom::Current(16))?;

    // 2. Iterate chunks to locate CHNKSQLi
    let mut sqlite_data_buffer = None;

    loop {
        let mut chunk_name_buffer = [0u8; 8];
        if let Err(error) = file.read_exact(&mut chunk_name_buffer) {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                break;
            }
            return Err(Box::new(error));
        }

        let chunk_total_length = file.read_u64::<BigEndian>()?;
        let chunk_start_position = file.stream_position()?;

        if &chunk_name_buffer == SQL_CHUNK_NAME {
            // Found the SQLite database chunk.
            // We read the entire chunk into memory. Previews are typically in files
            // where this chunk is a few MBs.
            let mut data = vec![0u8; chunk_total_length as usize];
            file.read_exact(&mut data)?;
            sqlite_data_buffer = Some(data);
            break;
        }

        if &chunk_name_buffer == FOOTER_CHUNK_NAME {
            break;
        }

        // Jump to the start of the next chunk.
        file.seek(SeekFrom::Start(chunk_start_position + chunk_total_length))?;
    }

    let database_bytes = sqlite_data_buffer.ok_or(ClipError::MissingSqlChunk)?;

    // 3. Query SQLite database for the preview
    // Since SQLx expects a file path, we write the chunk to a temporary file.
    let temporary_directory = std::env::temp_dir();
    let temporary_database_path = temporary_directory.join(format!("mundam_clip_{}.sqlite", uuid::Uuid::new_v4()));
    std::fs::write(&temporary_database_path, database_bytes)?;

    // Use block_on to execute the async SQL queries in the current thread.
    let result = tauri::async_runtime::block_on(async {
        query_preview_from_sqlite(&temporary_database_path).await
    });

    // Ensure the temporary file is removed even if extraction fails.
    let _ = std::fs::remove_file(&temporary_database_path);

    result.map_err(|error| error.into())
}

/// Connects to the temporary SQLite database and retrieves the ImageData PNG.
async fn query_preview_from_sqlite(database_path: &Path) -> Result<(Vec<u8>, String), ClipError> {
    let path_string = database_path.to_str()
        .ok_or_else(|| ClipError::DatabaseError("Invalid temporary path".to_string()))?;

    // Use a single-connection pool for extraction.
    let connection_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite://{}", path_string))
        .await?;

    // The CanvasPreview table contains the rendered export preview.
    let query_result: (Vec<u8>,) = sqlx::query_as("SELECT ImageData FROM CanvasPreview LIMIT 1")
        .fetch_one(&connection_pool)
        .await
        .map_err(|error| ClipError::DatabaseError(error.to_string()))?;

    connection_pool.close().await;

    Ok((query_result.0, "image/png".to_string()))
}
