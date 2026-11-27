//! Exercises the SDK's preview resource through generated guest bindings.

wit_bindgen::generate!({
    path: "../../wit",
    world: "preview-check",
});

use mundam::worker::image_preview::open_selected_preview;

struct PreviewCheck;

impl Guest for PreviewCheck {
    fn read_preview_signature() -> Result<Vec<u8>, PreviewError> {
        let preview = open_selected_preview()?;
        if preview.byte_count() < 12 {
            return Err(PreviewError::InvalidRange);
        }
        let mut signature = Vec::with_capacity(8);
        for offset in [0, 8] {
            let marker = preview.read(offset, 4)?;
            if marker.len() != 4 {
                return Err(PreviewError::Unavailable);
            }
            signature.extend(marker);
        }
        Ok(signature)
    }
}

export!(PreviewCheck);
