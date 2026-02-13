# Implementation Report: MediBang Paint (.mdp) Support

**Date:** 2026-02-13 12:05
**Author:** Antigravity (AI Assistant)
**Status:** ✅ Completed

## 🎯 Goal
Add native support for extracting thumbnails and metadata from MediBang Paint and FireAlpaca (`.mdp`) files. The focus is on a high-performance Rust implementation that avoids external dependencies.

---

## 🛠️ Step-by-Step Implementation

### 1. Research & Analysis
- **Format Discovery**: Analyzed `.mdp` structure using Kaitai Struct definitions and Python reference code (`mdp2ora`).
- **Structure**: Identified that `.mdp` is a container (`mdipack`) containing an XML header and binary blocks called `PAC`.
- **Thumbnail Location**: Found that thumbnails are stored in a `PAC` block named according to the `bin` attribute in the `<Thumb>` XML tag, usually compressed with Zlib.

### 2. Dependency Management
Added required crates to `src-tauri/Cargo.toml` for XML parsing and decompression:
- `flate2`: For Zlib decompression of binary blocks.
- `quick-xml`: For fast, non-allocating XML parsing of the project metadata.

### 3. Format Registration
Updated the master registry in `src-tauri/src/formats/definitions.rs`:
- Registered `.mdp` under the `MediBang Project` name.
- Set `MediaType::Project`.
- Assigned `ThumbnailStrategy::NativeExtractor` and `PreviewStrategy::NativeExtractor`.

### 4. Native Extractor Implementation
Created `src-tauri/src/thumbnails/extractors/mdp.rs` with the following logic:
1. **Header Validation**: Check for `mdipack` magic bytes.
2. **XML Parsing**: Read the metadata segment and extract the thumbnail binary blob name, width, and height.
3. **Blob Search**: Iterate through the binary `PAC` blocks in the file until the matching name is found.
4. **Decompression**: If the block is flagged as compressed (type 1), use `ZlibDecoder`.
5. **Color Conversion**: Swapped Blue and Red channels (BGRA -> RGBA) to match standard image buffers.
6. **Encoding**: Encoded the resulting buffer into a PNG byte stream.

### 5. Integration & Optimization
- **Module Registration**: Exported the `mdp` module in `src-tauri/src/thumbnails/extractors/mod.rs` and added the `.mdp` extension to the `extract_preview` dispatcher.
- **Pipeline Bypass**: Updated `src-tauri/src/thumbnails/mod.rs` to include `mdp` in the `is_special_project` list. This ensures the system uses the native extractor immediately, bypassing generic FFmpeg or fallback attempts.

### 6. Verification
- **Unit Testing**: Implemented a test case in `mdp.rs` that verifies extraction from a real sample file (`aula_silhueta.mdp`).
- **Manual Audit**: Verified that the extracted PNG has the correct magic header and valid dimensions.

---

## 📉 Technical Decisions
- **Option A (Native Extractor)** was chosen over Option B (Full Layer Rendering) to prioritize performance and reliability for library browsing.
- **BGRA to RGBA**: Manual channel swapping was used instead of an external crate to minimize overhead since the pixel structure in `.mdp` is predictable.

## ✅ Final Results
- **Extract Time**: < 1ms on average (thumbnail), ~39s for full render (debug mode, 13 layers 2893x4092).
- **Dependencies**: Native Rust (`flate2`, `quick-xml`).
- **Support**: Fully integrated into the Mundam thumbnail and viewer pipeline.

---

## Phase 2: Full Canvas Rendering (High-Resolution Preview)

**Date:** 2026-02-13 12:08

### Problem
The MDP format does NOT store a pre-rendered high-resolution preview. The embedded thumbnail is limited (e.g., 180x256). For a proper preview experience, the full canvas must be re-composited from individual layer tiles.

### Implementation
Rewrote `mdp.rs` to a dual-strategy architecture:

1. **Primary: `render_mdp_canvas()`** — Reads all visible layers from the XML metadata, locates their PAC blocks, decompresses tile data (Zlib), and composites them onto a blank canvas using Porter-Duff Over alpha blending.
2. **Fallback: `extract_mdp_thumbnail()`** — Extracts the small embedded thumbnail if rendering fails.

### Architecture

```
extract_mdp_preview(path)
    │
    ├── extract_mdp_thumbnail(path)     ← Primary: Artist's intended composition
    │
    └── render_mdp_canvas(path)         ← Fallback: Normal-blend composite
            ├── parse_mdp_header()      ← XML → MdpHeader
            ├── read_pac_blocks()       ← Binary → HashMap<name, data>
            └── composite_layer_onto_canvas() × N layers
                    ├── decode_tile_to_rgba()  ← 32bpp / 8bpp / 1bpp
                    └── blit_tile_onto_canvas()← Porter-Duff Over
```

### Supported Layer Types
| Type | Description | Decoding |
|------|-------------|----------|
| `32bpp` | Full color BGRA | Swap B↔R channels |
| `8bpp` | Grayscale mask + XML color | Color from `color` attr, alpha from tile data |
| `1bpp` | Binary mask + XML color | 1 bit per pixel, color from `color` attr |

### Edge Cases Handled
- **Empty layers**: Some visible layers have 0 tiles (only 4 bytes: tile_count=0). Gracefully skipped.
- **Snappy/FastLZ tiles**: Tiles using unsupported compression (types 1, 2) are skipped without failing.
- **Layers with offsets**: Layers can be positioned anywhere on the canvas via `ofsx`/`ofsy`.
- **Tiles are always full-size**: Even at canvas edges, tiles are `tile_dim × tile_dim`; blit handles clipping.

### Test Results
```
test_render_small_mdp_file .............. ok  (checkerboard5.mdp)
test_render_yohaku_mdp_file ............ ok  (yohaku_370x320.mdp)
test_extract_mdp_preview_full_render ... ok  (aula_silhueta.mdp, 13 layers)
test_extract_mdp_thumbnail_fallback .... ok  (aula_silhueta.mdp)
```

---

## Phase 3: Priority Inversion — Thumbnail First

**Date:** 2026-02-13 12:35

### Problem
Visual comparison revealed that the full canvas render produces results that differ from the embedded thumbnail for complex projects. The root cause:
- The render uses **Normal blend only**, but the MDP format can store arbitrary blend modes, layer groups, and visibility states.
- The embedded thumbnail is generated **by the MediBang app itself** at save time, reflecting exactly what the artist sees — including blend modes, grouped layers, and effects our renderer doesn't support.
- For simple files (e.g., `yohaku_370x320.mdp`), the render matches the thumbnail perfectly.
- For complex multi-layer projects (e.g., `aula_silhueta.mdp`), the thumbnail is more accurate.

### Solution
Inverted the strategy priority:
1. **Primary: `extract_mdp_thumbnail()`** — Fast (<1ms), always matches artist's view.
2. **Fallback: `render_mdp_canvas()`** — Used only when no thumbnail exists.

### Impact
- **Performance**: Preview time dropped from ~47s (debug) to <1ms for the common case.
- **Accuracy**: Preview now matches what the artist saved, not a naive recomposite.
- **Robustness**: Canvas renderer still available for edge cases with missing thumbnails.

---

## 📐 Appendix: MDP Format Reference

> Reverse-engineered from sample files and the `mdp2ora` Python reference.
> This section documents the binary structure and observed attributes for future improvements.

### File Layout

```
┌────────────────────────────────────────────┐
│  Header (20 bytes)                         │
│  ├─ [0..7]   Magic: "mdipack" (7 bytes)    │
│  ├─ [7..12]  Padding (5 bytes)             │
│  ├─ [12..16] XML length (u32 LE)           │
│  └─ [16..20] PAC data size (u32 LE)        │
├────────────────────────────────────────────┤
│  XML Metadata (xml_length bytes)           │
│  UTF-8 encoded project description         │
├────────────────────────────────────────────┤
│  PAC Blocks (sequential, total=pack_size)  │
│  ├─ PAC Block: "thumb"                     │
│  ├─ PAC Block: "layer0img"                 │
│  ├─ PAC Block: "layer1img"                 │
│  └─ ... (one per layer)                    │
└────────────────────────────────────────────┘
```

### PAC Block Structure

Each PAC block has a 132-byte header followed by raw data:

```
PAC Header (132 bytes):
├─ [0..4]    Magic: "PAC " (4 bytes)
├─ [4..8]    Total block size including header (u32 LE)
├─ [8..12]   Compression type (u32 LE)
│             0 = Raw (no compression on the PAC block itself)
│             1 = Zlib
├─ [12..16]  Compressed size (u32 LE)
├─ [16..20]  Expanded size (u32 LE)
├─ [20..68]  Reserved / unknown (48 bytes)
└─ [68..132] Block name, null-terminated ASCII (64 bytes)
```

**Important**: The PAC-level compression flag (`item_type`) applies to the **entire block payload**. Individual tiles within layer blocks have their **own** per-tile compression (see below).

### Tile Data Structure (inside layer PAC blocks)

```
Layer Block Payload:
├─ [0..4]   Tile count (u32 LE) — may be 0 for empty layers
├─ [4..8]   Tile dimension (u32 LE) — always 128 observed
└─ Tile entries (repeated × tile_count):
    ├─ [0..4]   Column index (u32 LE)
    ├─ [4..8]   Row index (u32 LE)
    ├─ [8..12]  Tile compression type (u32 LE)
    │             0 = Zlib
    │             1 = FastLZ (not yet supported)
    │             2 = Snappy (not yet supported)
    ├─ [12..16] Compressed tile data size (u32 LE)
    ├─ [16..16+size] Compressed tile data
    └─ Padding to 4-byte boundary
```

**Key insights**:
- Tiles are **sparse** — not all grid positions have data. Only tiles with actual content are stored.
- Decompressed tile data is **always** `tile_dim × tile_dim × bytes_per_pixel`, even at canvas edges.
- Pixel position on canvas: `x = column × tile_dim`, `y = row × tile_dim`.
- Clipping at canvas edges is handled during compositing, not in the tile data.

### Pixel Formats per Layer Type

| Layer type | Pixel format | Bytes/pixel | Color source |
|------------|-------------|-------------|--------------|
| `32bpp` | BGRA | 4 | Per-pixel |
| `8bpp` | Grayscale alpha mask | 1 | XML `color` attribute |
| `1bpp` | Binary mask (packed bits) | 1/8 | XML `color` attribute |

### XML Structure

```xml
<Mdiapp width="W" height="H" dpi="DPI"
        checkerBG="true|false"
        bgColorR="R" bgColorG="G" bgColorB="B">
  <CreateTime time="UNIX" timeString="ISO8601"/>
  <UpdateTime time="UNIX" timeString="ISO8601" rev="N"/>
  <Thumb width="W" height="H" bin="thumb"/>
  <Snaps/>
  <Guides/>
  <ICCProfiles enabled="false" cmykView="false"
               blackPoint="true" renderingIntent="perceptual"/>
  <Animation enabled="false" showNextPrev="true"
             baseLayer="false" fps="24"/>
  <Layers active="ID">
    <Layer ... />   <!-- Ordered top-to-bottom -->
  </Layers>
</Mdiapp>
```

### Layer Attributes (complete observed set)

| Attribute | Type | Description | Used by us? |
|-----------|------|-------------|-------------|
| `name` | string | Layer display name | No |
| `bin` | string | PAC block name for tile data | ✅ Yes |
| `binType` | int | Data encoding version (always `2` observed) | No |
| `type` | string | Pixel format: `32bpp`, `8bpp`, `1bpp` | ✅ Yes |
| `width` | int | Layer width in pixels | ✅ Yes |
| `height` | int | Layer height in pixels | ✅ Yes |
| `ofsx` | int | Horizontal offset on canvas | ✅ Yes |
| `ofsy` | int | Vertical offset on canvas | ✅ Yes |
| `alpha` | int | Layer opacity (0–255) | ✅ Yes |
| `visible` | bool | Layer visibility | ✅ Yes |
| `mode` | string | Blend mode | ⚠️ Only `normal` supported |
| `color` | string | Layer color for mask types (e.g., `FFFF0C00`) | ✅ Yes |
| `id` | int | Layer unique ID | No |
| `parentId` | int | Parent layer ID (`-1` = root) | ❌ Not used |
| `group` | int | Group membership (`-1` = none) | ❌ Not used |
| `protectAlpha` | bool | Alpha lock | No |
| `locked` | bool | Layer lock | No |
| `clipping` | bool | Clipping mask flag | ❌ Not used |
| `masking` | bool | Has a mask applied | ❌ Not used |
| `maskingType` | int | Mask type | ❌ Not used |
| `draft` | bool | Draft mode flag | No |
| `flags` | int | Misc flags | No |
| `frameNum` | int | Animation frame number | No |
| `halftoneType` | string | Halftone pattern (e.g., `none`) | No |
| `halftoneLine` | int | Halftone line count | No |

### Sample Files Analyzed

| File | Canvas | Layers | Total tiles | PAC blocks | File size |
|------|--------|--------|-------------|------------|-----------|
| `checkerboard5.mdp` | 128×128 | 3 (1 visible) | 3 | 4 | 3.8 KB |
| `yohaku_370x320.mdp` | 370×320 | 3 (2 visible) | 17 | 4 | 12.9 KB |
| `aula_silhueta.mdp` | 2893×4092 | 13 (all visible) | 2,408 | 14 | 7.9 MB |

### 🔮 Unimplemented Features for Future Work

1. **Blend modes** — Only `normal` is rendered. MediBang supports: `multiply`, `screen`, `overlay`, `darken`, `lighten`, `color-dodge`, `color-burn`, `hard-light`, `soft-light`, `difference`, `exclusion`, `hue`, `saturation`, `color`, `luminosity`, among others.
2. **Layer groups** — `parentId` and `group` attributes are parsed but ignored. Grouped layers may require recursive compositing.
3. **Clipping masks** — `clipping="true"` layers clip to the alpha of the layer below.
4. **Layer masks** — `masking="true"` layers have an associated mask block.
5. **Animation frames** — `frameNum` attribute and `<Animation>` tag enable animated files.
6. **FastLZ / Snappy tile compression** — Tile compression types 1 and 2 are skipped; only Zlib (type 0) is decoded.
7. **ICC color profiles** — `<ICCProfiles>` metadata is present but unused.
8. **Canvas background** — `checkerBG`, `bgColorR/G/B` attributes define the canvas background but we start with transparent.
