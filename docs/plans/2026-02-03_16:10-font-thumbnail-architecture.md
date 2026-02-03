# Plan: Font Thumbnail Generation via SVG/Resvg

## 1. Context & Objective
Currently, font files (`.ttf`, `.otf`, `.woff`, etc.) in Elleven Library are displayed with a generic file icon. The user context requires a more elegant solution: generating a visual preview (Thumbnail) of the font itself, typically showing characters like "Aa" or "ABC...".

We agreed to proceed with **Option A**: Using an SVG Template rendered via `resvg` (Rust), which is already a dependency in the project.

## 2. Architecture

### The Workflow
1. **Detection**: `FileFormat::detect` identifies a file as `MediaType::Font` and assigns `ThumbnailStrategy::Font` (new strategy).
2. **Dispatch**: `thumbnails::generate_thumbnail` routes this strategy to a new module `thumbnails::font`.
3. **Generation (`thumbnails::font::generate_font_thumbnail`)**:
   - Create a temporary `usvg::fontdb::Database`.
   - Load the target font file into this database.
   - Query the database to retrieve the **PostScript Name** or **Family Name** of the loaded font.
   - Inject this family name into an SVG template string (e.g., `<text font-family="{name}">Aa</text>`).
   - Parse this SVG using `usvg`.
   - Render to a Pixmap using `resvg`/`tiny-skia`.
   - Encode to WebP.

### The SVG Template
A minimalist and elegant card design, 400x500px, consistent with other thumbnails.
- **Background**: Subtle gradient or solid off-white/dark depending on theme (fixed for now, maybe dynamic later).
- **Content**:
  - Large "Aa" in the center.
  - Optional small "The quick brown fox..." below if space permits.
  - Font styling applied via the injected font family.

## 3. Implementation Steps

### Step 1: Update Domain Models (`src-tauri/src/formats.rs`)
- Add `Font` to `ThumbnailStrategy` enum.
- Update `SUPPORTED_FORMATS` for font entries (`ttf`, `otf`, `woff`, `woff2`) to use `ThumbnailStrategy::Font` instead of `ThumbnailStrategy::Icon`.

### Step 2: Create Font Renderer (`src-tauri/src/thumbnails/font.rs`)
Implement the core logic.
- **Dependencies**: Uses `resvg`, `usvg`, `tiny_skia`, `webp`.
- **Function Signature**:
  ```rust
  pub fn generate_font_thumbnail(
      input_path: &Path,
      output_path: &Path,
      size_px: u32
  ) -> Result<(), Box<dyn std::error::Error>>
  ```
- **Key Logic**:
  ```rust
  let mut db = usvg::fontdb::Database::new();
  db.load_font_file(input_path)?;
  // We need to find the family name of the font we just loaded.
  // Since we load a specific file, we can inspect db.faces() matching that source or just take the last added.
  let family = db.faces().last().unwrap().family.clone();
  
  let svg = format!(r#"
    <svg ...>
       <text font-family="{}" ...>Aa</text>
    </svg>
  "#, family);
  ```

### Step 3: Integrate (`src-tauri/src/thumbnails/mod.rs`)
- Add `mod font;`.
- Update the match arm in `generate_thumbnail` to handle `ThumbnailStrategy::Font`.

### Step 4: Template Refinement
- Design the SVG string.
- Colors: `rgb(30, 30, 30)` text on `rgb(240, 240, 240)` background, or similar high-contrast "generic" look.
- Dimensions: 400x500.

## 4. Verification
1. **Manual Test**:
   - Find or copy a `.ttf` file into a watched folder.
   - Run the app (or trigger re-indexing).
   - Verify the thumbnail is generated and looks like text, not an icon.
2. **Error Handling**:
   - Verify that if the font is corrupt, it falls back to the generic icon (the existing robust fallback logic in `mod.rs` should handle this).

## 5. Future Improvements
- **Dark Mode**: Generate transparent background thumbnails so they look good in both modes? (WebP supports transparency).
- **Custom Text**: Allow user configuration for preview text.

---

## 6. Implementation Log - WOFF/WOFF2 Support (2026-02-03)

### The Challenge
Standard `.ttf` and `.otf` files are natively supported by `fontdb` (and consequentially `resvg`). However, web-optimized formats (`.woff`, `.woff2`) are compressed containers that `fontdb` (via `ttf-parser`) does not support natively in the version we are using, or they require specific features to be enabled.

### The Solution: Hybrid Decoding
To support `woff` and `woff2` without complex C++ bindings or heavy external tools, we implemented a hybrid decoding step in Rust using the `wuff` crate.

1.  **WOFF (v1) & WOFF2 (v2)**:
    - We added the `wuff` crate (v0.2.3) dependency.
    - Before loading the font into `fontdb`, we detect if the file extension is `woff` or `woff2`.
    - If detected, we read the file into memory and use `wuff::decompress_woff1` or `wuff::decompress_woff2` to extract the underlying SFNT (TTF/OTF) data.
    - The raw decompressed binary is then passed to `fontdb.load_font_source(Source::Binary(...))`.
    - This allows `resvg` to render the font preview transparently, as it thinks it's dealing with a standard TTF.

### Code Changes
- **`src-tauri/Cargo.toml`**: Added `wuff = "0.2.3"`.
- **`src-tauri/src/thumbnails/font.rs`**: Added import `wuff`. Added conditional logic to decompress WOFF/WOFF2 files before loading.
- **`src-tauri/src/formats.rs`**: Updated `ThumbnailStrategy` for "Web Open Font Format" and "Web Open Font Format 2" from `Icon` to `Font`.

### Result
All major font formats (`ttf`, `otf`, `ttc`, `woff`, `woff2`) now generate high-quality visual previews (thumbnails) instead of generic icons.

