# Native GoPro RAW Decoder

## Goal

Decode GoPro `.gpr` images on Windows, macOS, and Linux without compiling or linking the GoPro GPR SDK. The application parses the TIFF/DNG container in Rust and uses FlashRaw for VC-5 sensor decoding and RAW development.

## License boundary

- FlashRaw `0.1.1` is licensed under either MIT or Apache-2.0. Its published package includes both license texts. The dependency is pinned in `src-tauri/Cargo.toml` so an update is reviewed deliberately.
- The decoder is consumed as a Rust crate; no GoPro SDK or Adobe DNG SDK source is copied into Mundam.
- The GPR SDK repository contains Adobe-derived DNG, XMP, and Expat components with separate notices. Removing the SDK submodule also removes those sources from this build.
- VC-5 is an industry codec standard with GoPro/CineForm bitstream differences. Copyright licensing does not resolve any patent questions; those remain separate from this dependency implementation.

## Processing path

1. `extractors/gpr.rs` opens the file with FlashRaw and reads normalized DNG metadata, including active and raw dimensions, camera details, orientation, exposure, and GPS when present.
2. For image output, the same module requests display-ready RGBA pixels. FlashRaw performs VC-5 decoding, demosaicing, white balance, color conversion, and orientation.
3. Thumbnail generation uses an orientation-safe embedded thumbnail when available; otherwise it decodes the RAW image. The result is resized and encoded as WebP. Full preview output is encoded as JPEG.
4. The GPR provider runs these synchronous operations inside Tokio's blocking pool.

## Validation corpus

Local GoPro samples live under `file-samples/Arquivo para testes/Image/gpr/`. They are development inputs and are not bundled as application resources. The existing GPR tests cover container dimensions, metadata, thumbnail output, and JPEG preview output. They should be run against files from multiple camera generations when changing the decoder version.

## Current implementation

The thumbnail, preview, and metadata paths use FlashRaw, with `rexif` adding EXIF fields not covered by its normalized metadata. A separate TIFF parser was removed after it failed to recognize valid GoPro image directories and incorrectly blocked metadata requests. The C FFI, SDK CMake build, and vendor submodule are removed. FlashRaw is a new dependency and its GPR output should be validated against the local sample corpus on each supported desktop platform before release.
