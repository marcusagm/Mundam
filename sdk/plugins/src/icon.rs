//! Bounded PNG validation shared by authoring tools and package admission.

use crate::{ValidationError, MAX_ICON_BYTES};
use std::io::Cursor;

/// Validates a static PNG before it can be shown in an installation prompt.
///
/// # Errors
/// Rejects oversize resources, corrupt data, animation and dimensions outside 16–256 pixels.
pub fn validate_icon(bytes: &[u8]) -> Result<(), ValidationError> {
    if bytes.len() > MAX_ICON_BYTES {
        return Err(ValidationError::new("icon", "PNG exceeds 256 KiB"));
    }

    let invalid_png = || ValidationError::new("icon", "expected a valid static PNG");
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: 2 * 1024 * 1024,
        },
    );
    decoder.set_ignore_text_chunk(true);
    decoder.set_ignore_iccp_chunk(true);
    let header = decoder.read_header_info().map_err(|_| invalid_png())?;
    if !(16..=256).contains(&header.width) || !(16..=256).contains(&header.height) {
        return Err(ValidationError::new(
            "icon",
            "PNG dimensions must be 16–256 pixels",
        ));
    }

    let mut reader = decoder.read_info().map_err(|_| invalid_png())?;
    if reader.info().animation_control.is_some() {
        return Err(invalid_png());
    }
    let output_size = reader.output_buffer_size().ok_or_else(invalid_png)?;
    let mut pixels = vec![0; output_size];
    reader.next_frame(&mut pixels).map_err(|_| invalid_png())?;
    reader.finish().map_err(|_| invalid_png())
}
