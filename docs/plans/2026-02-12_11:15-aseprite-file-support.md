# Implementation Plan: Aseprite File Support (.ase/.aseprite)

This plan outlines the steps to add support for Aseprite files in **Mundam**, providing high-quality thumbnails and animated previews using the `asefile` crate.

---

## 🛠️ Phase 1: Environment Setup

- [x] Add `asefile` to `src-tauri/Cargo.toml`.
- [x] Run `cargo fetch` to ensure dependencies are available.

## 📝 Phase 2: Format Registration

- [x] Update `src-tauri/src/formats/definitions.rs` to include `.ase` and `.aseprite` in the `SUPPORTED_FORMATS` registry.
- [x] Add magic bytes detection (handled via extension for now).

## ⚙️ Phase 3: Core Implementation (Extraction Logic)

- [x] Create `src-tauri/src/thumbnails/extractors/aseprite.rs`.
- [x] Implement `extract_aseprite_preview` function:
- [x] Register the new extractor in `src-tauri/src/thumbnails/extractors/mod.rs`.

## 🧪 Phase 4: Validation & Testing

- [x] Verify compilation via `cargo check`.
- [ ] Verify static thumbnail generation in UI.
- [ ] Verify animated preview generation in UI.
- [ ] Test with provided samples.

---

## 📅 Timeline & Status
- **Date**: 2026-02-12
- **Assigned to**: Antigravity
- **Status**: 🟡 Testing (Logic Implemented)
