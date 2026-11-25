//! Inspects host-prepared content off the async executor before publication.
use crate::{
    core::{
        error::{AppError, AppResult},
        imports::publication::PublishPreparedImport,
        library::FileIdentity,
    },
    infra::filesystem::identity,
};
use image::{GenericImageView, ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::Path,
    time::SystemTime,
};

pub(super) struct InspectedFile {
    pub identity: FileIdentity,
    pub length: u64,
    pub modified: SystemTime,
    pub width: u32,
    pub height: u32,
}

pub(super) async fn inspect(
    source: &Path,
    command: &PublishPreparedImport,
) -> AppResult<InspectedFile> {
    let path = Path::new(&command.item.file_name);
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        || command.item.file_name.len() > 240
    {
        return Err(AppError::ValidationFailed(
            "Expected a portable PNG file name".into(),
        ));
    }

    let source = source.to_path_buf();
    let expected_length = command.item.byte_length;
    let expected_digest = command.sha256;
    tokio::task::spawn_blocking(move || {
        let metadata = file_metadata(&source)?;
        let file_identity = identity::read_sync(&source)?;
        let modified = metadata.modified()?;
        let file = File::open(&source)?;
        let mut bytes = Vec::new();
        file.take(expected_length + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != expected_length || Sha256::digest(&bytes)[..] != expected_digest {
            return Err(AppError::ValidationFailed(
                "Prepared import length or digest differs from the sealed upload".into(),
            ));
        }
        if image::guess_format(&bytes).ok() != Some(ImageFormat::Png) {
            return Err(AppError::ValidationFailed(
                "Prepared import content is not PNG".into(),
            ));
        }
        let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
        let mut limits = Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().map_err(|_| {
            AppError::ValidationFailed("Prepared PNG is corrupt or exceeds decoding limits".into())
        })?;
        let (width, height) = image.dimensions();
        Ok(InspectedFile {
            identity: file_identity,
            length: expected_length,
            modified,
            width,
            height,
        })
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?
}

fn file_metadata(path: &Path) -> AppResult<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(AppError::ValidationFailed(
            "Prepared entry is not a regular file".into(),
        ));
    }
    Ok(metadata)
}

pub(super) async fn recheck_source(path: &Path, inspected: &InspectedFile) -> AppResult<()> {
    let path = path.to_path_buf();
    let expected = inspected.identity.clone();
    let expected_length = inspected.length;
    let expected_modified = inspected.modified;
    tokio::task::spawn_blocking(move || {
        let metadata = file_metadata(&path)?;
        if identity::read_sync(&path)? != expected
            || metadata.len() != expected_length
            || metadata.modified()? != expected_modified
        {
            return Err(AppError::ValidationFailed(
                "Prepared file changed after inspection".into(),
            ));
        }
        Ok(())
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?
}

pub(super) async fn recheck_file(path: &Path, expected: &FileIdentity) -> AppResult<()> {
    if &identity::read(path).await? != expected {
        return Err(AppError::ValidationFailed(
            "Published file identity differs from the prepared file".into(),
        ));
    }
    Ok(())
}

pub(super) async fn recheck_directory(path: &Path, expected: &FileIdentity) -> AppResult<()> {
    if !tokio::fs::symlink_metadata(path).await?.is_dir()
        || &identity::read(path).await? != expected
    {
        return Err(AppError::ValidationFailed(
            "Import destination identity changed".into(),
        ));
    }
    Ok(())
}
