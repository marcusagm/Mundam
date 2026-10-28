# Dependency Inventory

This document covers the application's direct dependencies declared in `package.json` and `src-tauri/Cargo.toml`. Versions are controlled by `package-lock.json` and `src-tauri/Cargo.lock`; version ranges in the manifests are not necessarily the exact resolved versions. Transitive packages are intentionally not expanded here: their full inventory is in those lockfiles and their license obligations should be checked as part of release compliance. License identifiers below are taken from installed npm package metadata or the crates' published metadata where known; confirm them against the exact locked release before distributing a binary.

## Backend (Rust / Tauri)

| Dependency | License | Why it is used / important details |
|---|---|---|
| `tauri` | MIT OR Apache-2.0 | Desktop application runtime and native/webview integration. |
| `tauri-build` (build) | MIT OR Apache-2.0 | Generates Tauri build-time integration. |
| `tauri-plugin-opener` | MIT OR Apache-2.0 | Opens URLs and files through the operating system. |
| `tauri-plugin-sql` | MIT OR Apache-2.0 | SQLite access from the application; SQLite is enabled. |
| `tauri-plugin-fs` | MIT OR Apache-2.0 | Scoped filesystem operations. |
| `tauri-plugin-dialog` | MIT OR Apache-2.0 | Native file and folder dialogs. |
| `tauri-plugin-http` | MIT OR Apache-2.0 | HTTP requests from the native side. |
| `tauri-plugin-clipboard-manager` | MIT OR Apache-2.0 | Clipboard integration. |
| `serde`, `serde_json` | MIT OR Apache-2.0 | Serialization and JSON handling. |
| `libc` | MIT OR Apache-2.0 | Platform C library bindings used by native code and dependencies. |
| `cmake` (build) | MIT OR Apache-2.0 | Builds native components that use CMake. |
| `walkdir` | MIT OR Apache-2.0 | Recursive directory traversal. |
| `notify` | CC0-1.0 OR Apache-2.0 OR MIT | Watches folders for changes. |
| `imagesize` | MIT OR Apache-2.0 | Reads image dimensions without full decoding. |
| `chrono` | MIT OR Apache-2.0 | Date/time handling and serialization. |
| `tokio`, `tokio-util` | MIT OR Apache-2.0 | Async runtime and stream utilities. |
| `sqlx` | MIT OR Apache-2.0 | Async SQLite database access and query support. |
| `image` | MIT OR Apache-2.0 | Decodes and encodes raster formats; enabled features include PNG, TIFF, GIF, WebP, HDR, EXR, DDS and TGA. |
| `fast_image_resize` | MIT OR Apache-2.0 | Efficient image resizing, especially for thumbnails. |
| `mime_guess` | MIT OR Apache-2.0 | Guesses MIME types from file names. |
| `percent-encoding`, `urlencoding` | MIT OR Apache-2.0 | Encodes URL and URI components. |
| `rayon` | MIT OR Apache-2.0 | Parallel CPU work. |
| `rexif` | MIT OR Apache-2.0 | Reads EXIF metadata. |
| `infer` | MIT | Detects file types from magic bytes. |
| `strum`, `strum_macros` | MIT | Enum derives and iteration. |
| `blake3` | CC0-1.0 OR Apache-2.0 | Hashing for content identity and duplicate-related operations. |
| `kmeans_colors`, `palette` | MIT OR Apache-2.0 | Color conversion and color clustering/analysis. |
| `zune-jpeg` | MIT OR Apache-2.0 | JPEG decoding/encoding fast path. |
| `webp` | MIT OR Apache-2.0 | WebP encoding support. |
| `zip` | MIT OR Apache-2.0 | Reads archive-based formats. |
| `resvg`, `tiny-skia` | MIT OR Apache-2.0 | SVG rendering and its 2D raster backend. |
| `pdfium-render` | MIT OR Apache-2.0 | Renders PDF pages. PDFium itself is a separate native runtime component; its binary/source licensing and distribution terms should be reviewed separately. |
| `wuff` | MIT OR Apache-2.0 | Decodes WUFF-supported image formats. |
| `psd` | MIT OR Apache-2.0 | Reads Photoshop PSD files. |
| `asefile` | MIT OR Apache-2.0 | Reads Aseprite/ASE files. |
| `axum`, `tower-http` | MIT OR Apache-2.0 | Local HTTP/HLS streaming server and HTTP middleware (including CORS). |
| `uuid` | MIT OR Apache-2.0 | Generates unique identifiers. |
| `thiserror` | MIT OR Apache-2.0 | Derives typed error implementations. |
| `memmap2` | MIT OR Apache-2.0 | Memory-mapped file access for large media. |
| `rsraw` | MIT OR Apache-2.0 | Reads camera RAW formats. Check the locked crate metadata for any format-specific notices. |
| `base64` | MIT OR Apache-2.0 | Base64 encoding and decoding. |
| `wait-timeout` | MIT OR Apache-2.0 | Waits for child process completion with a timeout. |
| `byteorder` | MIT OR Unlicense | Explicit endian-aware binary parsing. |
| `flate2` | MIT OR Apache-2.0 | DEFLATE compression/decompression. |
| `quick-xml` | MIT | XML parsing and writing. |
| `zstd` | MIT OR Apache-2.0 | Zstandard compression/decompression. Native Zstandard components may be included through its dependency tree. |
| `tracing`, `tracing-subscriber`, `log` | MIT OR Apache-2.0 | Application diagnostics and structured logging. |
| `async-trait`, `async-recursion` | MIT OR Apache-2.0 | Async trait and recursive async function support. |
| `ttf-parser` | MIT OR Apache-2.0 | Parses TrueType/OpenType font metadata. |
| `dashmap` | MIT OR Apache-2.0 | Concurrent hash map for shared runtime state. |
| `unicode-normalization` | MIT OR Apache-2.0 | Unicode normalization. |
| `jxl-oxide` | MIT OR Apache-2.0 | JPEG XL decoding. Review notices for codec dependencies in the lockfile. |
| `icns` | MIT OR Apache-2.0 | Reads/writes macOS icon files. |
| `ddsfile`, `image_dds` | MIT OR Apache-2.0 | DDS texture container support. |
| `midly`, `rustysynth`, `hound` | MIT OR Apache-2.0 | MIDI parsing, software synthesis, and WAV audio processing. |
| `rsraw-sys` (dev) | MIT OR Apache-2.0 | Native bindings used by RAW-format development/testing. |
| `tempfile` (dev) | MIT OR Apache-2.0 | Temporary files in Rust tests. |

## Frontend (runtime)

| Dependency | License | Why it is used / important details |
|---|---|---|
| `solid-js` | MIT | Reactive UI framework. |
| `@tauri-apps/api` | Apache-2.0 OR MIT | JavaScript bridge to Tauri native functionality. |
| `@tauri-apps/plugin-clipboard-manager` | MIT OR Apache-2.0 | Clipboard API. |
| `@tauri-apps/plugin-dialog` | MIT OR Apache-2.0 | Native dialogs. |
| `@tauri-apps/plugin-fs` | MIT OR Apache-2.0 | Frontend filesystem API. |
| `@tauri-apps/plugin-http` | MIT OR Apache-2.0 | HTTP API. |
| `@tauri-apps/plugin-opener` | MIT OR Apache-2.0 | Open URLs/files externally. |
| `@tauri-apps/plugin-sql` | MIT OR Apache-2.0 | Frontend SQL API. |
| `@floating-ui/dom` | MIT | Positions floating menus and popovers. |
| `@google/model-viewer` | Apache-2.0 | Displays interactive 3D model content in the webview. |
| `@hypothesi/tauri-plugin-mcp-bridge` | MIT | Connects the Tauri application to the MCP bridge integration. |
| `@thisbeyond/solid-dnd` | MIT | Drag-and-drop interactions for SolidJS. |
| `fuse.js` | Apache-2.0 | Fuzzy search. |
| `hls.js` | Apache-2.0 | HLS playback in browsers/webviews that need JavaScript playback. |
| `lucide-solid` | ISC | SolidJS icon components. |
| `zod`, `zod-validation-error` | MIT | Runtime schema validation and readable validation errors. |

## External services, native tools, and bundled assets

These dependencies are staged as native executables/libraries or data files, rather than declared as ordinary frontend packages. Their license obligations apply when the corresponding binaries or assets are distributed with Mundam. The repository's setup scripts may download or copy platform-specific builds, so the exact binary and its notices must be tracked for each release.

| Component | License / terms | Why it is used / important details |
|---|---|---|
| **FFmpeg and FFprobe** (`src-tauri/resources/ffmpeg/<platform>/`) | FFmpeg is LGPL-2.1-or-later by default; a build that enables GPL components is GPL-2.0-or-later. `--enable-nonfree` can make a build non-redistributable. The applicable license depends on the exact binary's configure flags and included libraries. [FFmpeg licensing guidance](https://www.ffmpeg.org/legal.html) | Required external subprocesses for audio/video streaming and transcoding, waveform generation, previews/thumbnails, and technical metadata probing. `ffprobe` is staged as optional in setup, while `ffmpeg` is required. Setup copies the executables from the host/system installation; the Windows script can install Gyan FFmpeg via winget. Record each distributed build's source, version, configure output (`ffmpeg -buildconf`), license files, and third-party notices. Do not assume every binary is LGPL solely because upstream FFmpeg defaults to LGPL. |
| **LibRaw via `rsraw` / `rsraw-sys`** | Upstream LibRaw offers LGPL-2.1 or CDDL-1.0 (dual license). Its source also contains code and notices from other projects, including `dcraw.c`; retain the exact source release's notices. [LibRaw licensing](https://www.libraw.org/about) | RAW camera image metadata, embedded thumbnails, and pixel decoding. The `rsraw` Rust crate compiles LibRaw from source; `rsraw-sys` is also declared for development/tests. Build prerequisites include CMake, libclang, and a C++ compiler. Because this is compiled into native components, include the chosen license and corresponding source/notice materials for the locked LibRaw version in release compliance review. |
| **Assimp command-line tool and native libraries** (`src-tauri/resources/assimp/<platform>/`) | Assimp uses a modified 3-clause BSD license; include its license text. Assimp's bundled/contributed components can carry separate notices. [Assimp license](https://github.com/assimp/assimp/blob/master/LICENSE) | Optional conversion of 3D model formats (for example OBJ, FBX, and DAE) to a format the app can preview. Setup stages the `assimp` executable and associated shared libraries from the host installation. If absent, 3D previews are disabled. Preserve the package version, source, BSD license, and notices for the exact platform binaries. |
| **TimGM6mb SoundFont** (`src-tauri/resources/soundfonts/TimGM6mb.sf2`) | The precise license of the repository's binary is **unverified** because no source/provenance or license file accompanies it. Public sources conflict: Debian's `timgm6mb-soundfont` package marks the asset GPL-2.0 ([Debian copyright records](https://sources.debian.org/src/timgm6mb-soundfont/1.3-5/)), while MuseScore's SoundFonts page reports CC BY 3.0. Do not assume either applies to this exact copy until its origin is established. This is separate from the `rustysynth` crate license. | Sample bank used by `rustysynth` to synthesize MIDI previews and create audio streams/waveforms. The application searches for `.sf2` files in bundled resources and the user's app data folder. `TimGM6mb.sf2` is present in the repository. Confirm its provenance, version, and accompanying license before redistribution; if unavailable, replace it with a SoundFont with clear documented terms. |
| **GoPro GPR SDK** (`src-tauri/vendor/gpr_sdk`, Git submodule) | Third-party GoPro SDK; verify the license and redistribution terms in the checked-out submodule at the exact pinned revision. The repository README identifies it as separately vendored source but this inventory does not treat the absent/uninitialized submodule contents as license evidence. | Native SDK used in the RAW/GPR processing path and compiled with CMake on supported platforms. Record the submodule commit, license text, and any GoPro-specific redistribution conditions for releases. |
| **SQLite** | Public domain (SQLite project dedication). [SQLite copyright notice](https://www.sqlite.org/copyright.html) | Local application database, enabled through `tauri-plugin-sql` and SQLx. |
| **PDFium** (`src-tauri/resources/pdfium/`) | BSD-style license plus notices for third-party components. [PDFium-Binaries project](https://github.com/bblanchon/pdfium-binaries) | Native PDF rendering library used by `pdfium-render`. Setup downloads a platform archive from `bblanchon/pdfium-binaries` at the configured Chromium revision. Keep the revision, source archive, and bundled third-party notices with each release. |
| **Operating-system webview and native APIs** | Platform-specific terms | Tauri relies on the system webview and platform native libraries; these are not npm packages. |
| **User media and codec formats** | Depends on the file, codec, and jurisdiction | The app opens user-selected media. Supporting a format does not grant rights to redistribute user content or codecs; patent and content rights may also apply independently of open-source software licenses. |

## Development and build tools (npm)

These are direct `devDependencies`; they support building, linting, testing, or repository workflows and are not application runtime features.

| Dependency | License | Purpose |
|---|---|---|
| `@solidjs/testing-library` | MIT | Solid component testing helpers. |
| `@tauri-apps/cli` | Apache-2.0 OR MIT | Tauri development and packaging commands. |
| `@testing-library/jest-dom` | MIT | DOM matchers for tests. |
| `@typescript-eslint/eslint-plugin`, `@typescript-eslint/parser` | MIT | TypeScript linting support. |
| `eslint` | MIT | JavaScript/TypeScript linting. |
| `eslint-config-prettier` | MIT | Disables lint rules that conflict with Prettier. |
| `eslint-import-resolver-typescript` | ISC | Resolves TypeScript imports for ESLint. |
| `eslint-plugin-import`, `eslint-plugin-prettier`, `eslint-plugin-solid` | MIT | ESLint rules for imports, Prettier, and SolidJS. |
| `husky` | MIT | Git hooks. |
| `jsdom` | MIT | DOM environment for tests. |
| `lint-staged` | MIT | Runs formatting/linting on staged files. |
| `typescript` | Apache-2.0 | Type checking and compilation. |
| `vite`, `vite-plugin-solid` | MIT | Frontend dev server and production bundling, with SolidJS integration. |
| `vitest` | MIT | Test runner. |

## Maintenance notes

- The repository declares the application itself as MIT licensed in `package.json`. That license does not replace the notices and conditions of third-party dependencies.
- This is a direct-dependency inventory, not a complete software bill of materials (SBOM). Review both lockfiles and generate a release-specific license/SBOM report to capture transitive crates/packages, optional features, native libraries, and platform-specific artifacts.
- Keep this document aligned with `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and `src-tauri/Cargo.lock` when dependencies change. If package metadata conflicts with this summary, the exact locked release's license file and notices are authoritative.
