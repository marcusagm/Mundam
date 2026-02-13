# SAI/SAI2 Support Implementation Plan

## Overview
Added native support for PaintTool SAI (`.sai`) and PaintTool SAI v2 (`.sai2`) file formats. This allows Mundam to generate thumbnails and previews for these files without external dependencies.

## Implementation Details

### 1. SAI v1 Support (`extractors/sai.rs`)
- **Encryption**: Ported the ECB-style XOR cipher from `libsai` (C++) to pure Rust.
- **VFS**: Implemented a virtual file system reader to navigate the encrypted FAT entries.
- **Thumbnail**: Extracts the `/thumbnail` file (raw BGRA), converts it to RGBA, and encodes it as PNG.
- **Status**: Full support for standard encrypted `.sai` files using the static UserKey.

### 2. SAI v2 Support (`extractors/sai2.rs`)
- **Structure**: Implemented a chunk-based parser for the `SAI-CANVAS` format.
- **Thumbnail**: Extracts lossy JPEG thumbnails from `thum` chunks wrapped in `JSSF` containers.
- **Limitations**: Lossless DPCM thumbnails are currently skipped (placeholder added for future implementation).
- **Status**: Supports modern `.sai2` files with embedded JPEG thumbnails.

## Verification

### Automated Checks
- `cargo check` passes with no warnings for the new modules.
- Format definitions registered in `src/formats/definitions.rs`.

### Manual Testing Guide
1. **Prepare Test Files**:
   - Locate `.sai` and `.sai2` files in your library.
   - Ideally, use the provided samples in `file-samples/Imagens/Design/Paint tool SAI`.

2. **Run Mundam**:
   - Start the application: `npm run tauri dev`.
   - Navigate to the folder containing the SAI files.

3. **Verify Previews**:
   - **.sai**: Should generate a clear thumbnail. If it fails (icon only), the decryption key might differ (rare) or the file is corrupted.
   - **.sai2**: Should generate a clear thumbnail. If it fails, the file might use "lossless" saving which uses DPCM encoding (currently logged as a skip).

## Future Work
- **SAI v1**: Implement full layer reading to render high-res previews if the embedded thumbnail is too small.
- **SAI v2**: Implement DPCM decoding for lossless thumbnails.
