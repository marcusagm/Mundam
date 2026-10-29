<p align="center">
  <img src="public/branding/logo.svg" alt="Mundam Logo" width="400">
</p>

# Mundam

**Mundam** is a high-performance, local-first image reference manager designed specifically for artists, concept designers, and illustrators. It allows you to organize, tag, and view massive collections of reference images with zero lag, keeping your workflow uninterrupted.

Built with **Tauri v2**, **Rust**, and **SolidJS**, Mundam combines the raw power of native code with the reactivity of modern web interfaces.

### 💖 Support the Project

**Mundam** is a free, open-source project built to empower artists. If this tool helps your creative process and you'd like to support its continuous development and maintenance, consider making a donation. Your support keeps the project alive, independent, and free for everyone!

[![PayPal](https://www.paypalobjects.com/en_US/i/btn/btn_donate_LG.gif)](https://www.paypal.com/donate/?hosted_button_id=86CZGZQXBBQ6C)

---

## 🚀 Key Features

*   **Extreme Performance**:
    *   **Virtualized Masonry Grid**: Handles folders with thousands of images at a silky smooth 60fps.
    *   **Parallel Processing**: Uses multi-threaded CPU acceleration (Rayon) to generate high-quality WebP thumbnails in the background without freezing the UI.
    *   **Smart Caching**: Thumbnails are generated once and persisted. The incremental indexer tracks file changes instantly.

*   **Artist-Centric Design**:
    *   **Distraction-Free UI**: Minimalist dark mode interface that puts your art first.
    *   **Local First**: No cloud uploads, no subscriptions. Your images stay on your disk.
    *   **Real-Time Watcher**: Drop images into your folder, and they appear in the library immediately.

*   **Efficient Indexing**:
    *   **Duplicate Detection**: (Planned) Hash-based tracking to avoid duplicates.
    *   **Metadata Preserved**: Automatically reads modification dates and file specs.

---

## 🛠 Tech Stack

*   **Frontend**: SolidJS, TypeScript, Vite
*   **Backend**: Rust (Tauri v2)
*   **Database**: SQLite (via `sqlx` & `tauri-plugin-sql`)
*   **Styling**: Vanilla CSS (Scoped, Variable-based)
*   **Architecture**:
    *   Custom `thumb://` and `orig://` protocols for secure asset loading.
    *   Upsert-based indexing for robust crash recovery.
    *   Rayon-powered worker threads for heavy image lifting.

---

## 📦 Installation & Development

### Prerequisites
*   **Node.js** (v18+)
*   **Rust** (v1.85+)
*   **CMake** (Required by the LibRaw build used for other camera RAW formats)
*   **macOS / Linux / Windows** (Build tools required)

### Automated Setup (recommended)

After cloning, run the setup script for your platform. It installs the system prerequisites (compilers, CMake, libclang, WebKitGTK on Linux), **FFmpeg/FFprobe**, **Assimp**, downloads **PDFium**, and runs `npm install`. LibRaw is compiled from source by the `rsraw` crate, so it only needs CMake/libclang/a C++ compiler. GoPro GPR decoding uses the Rust-based FlashRaw crate and does not require a source submodule.

```bash
./scripts/setup.sh                 # macOS (Homebrew) and Linux (apt/dnf/pacman/zypper)
```
```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1   # Windows (winget)
```

Options: `--skip-system` / `-SkipSystem` (don't install packages), `--check` / `-Check` (run `cargo check` at the end), `-y` (non-interactive, Unix). On Windows, open a new terminal afterwards so `PATH` and `LIBCLANG_PATH` are refreshed. Assimp is optional on Windows (no package available): without `assimp.exe` in `src-tauri/resources/assimp/windows-x64/`, 3D previews are disabled.

### Native dependencies

Third-party source and runtime assets are kept separate:

| Path | Contents |
| --- | --- |
| `flashraw` Rust crate | MIT OR Apache-2.0 GoPro VC-5 decoder and RAW development pipeline. |
| `src-tauri/resources/ffmpeg/<platform>/` | Staged FFmpeg and FFprobe executables (`linux`, `macos`, or `windows-x64`). |
| `src-tauri/resources/assimp/<platform>/` | Optional Assimp executable and its libraries. |
| `src-tauri/resources/pdfium/` | PDFium runtime library for the current OS. |
| `src-tauri/resources/soundfonts/` | SoundFont files, bundled with the app for MIDI previews. |
| `src-tauri/resources/mingw/` | Windows GNU runtime DLLs shipped beside the app executable. |

Use `scripts/setup.sh` or `scripts/setup-windows.ps1` to prepare these dependencies. The older one-off download scripts have been removed; the platform setup scripts are now the single setup path.

### Getting Started (manual)

1.  **Clone the repository**
    ```bash
    git clone https://github.com/marcusagm/Mundam.git
    cd Mundam
    ```

2.  **Install Frontend Dependencies**
    ```bash
    npm install
    ```

3.  **Run in Development Mode**
    This command starts the Vite server and the Tauri Rust backend simultaneously with hot-reload enabled.
    ```bash
    npm run tauri dev
    ```

4.  **Build for Production**
    ```bash
    npm run tauri build
    ```
    macOS and Linux use Cargo's normal `src-tauri/target/` directory. On Windows, `scripts/tauri.cjs` selects the GNU toolchain and a no-space Cargo target directory only for that process, so the shared Cargo configuration stays portable.

---

## 🧩 Architecture Highlights

This project intentionally diverges from typical Electron/Web apps to prioritize performance:

1.  **Optimistic Updates**: The indexing worker communicates directly with the UI store via granular events, eliminating database polling and ensuring instant visual feedback.
2.  **Blocking Placeholders**: The UI intelligently hides original high-res images until the lightweight thumbnail is ready, saving significant RAM and CPU during rapid scrolling.
3.  **Self-Healing DB**: The internal SQLite database handles schema creation and migrations automatically on startup (`create_if_missing`).

---

## 🗺 Roadmap

### 1. Library & Location Management
*   [x] **Location Management**: Select and monitor local folders.
*   [x] **Real-time Watcher**: Auto-sync new files, renames, and deletions.
*   [ ] **Drag-and-Drop**: Import folders via drag-and-drop.
*   [ ] **Integrity Checks**: Detect and handle broken paths or moved libraries.

### 2. Tag System (Taxonomy)
*   [x] **Hierarchical Tags**: Parent/Child tag structures (Tag Tree).
*   [ ] **Tag Management**: Rename, merge, and move tags; custom colors.
*   [ ] **Assignment**: Bulk tagging, auto-complete suggestions.
*   [ ] **Tag Search**: Quick filtering of the tag list itself.

### 3. Media Visualization
*   [x] **Masonry Layout**: Optimized virtualized grid for variable aspect ratios.
*   [x] **Progressive Loading**: Async thumbnail generation and "lazy" original loading.
*   [x] **Slide/Inspection Mode**: Fullscreen viewer with zoom/pan and navigation.
*   [x] **File Actions**: "Open in Explorer", "Copy to Clipboard".

### 4. Search & Filtering
*   [x] **Basic Search**: By filename.
*   [x] **Advanced Criteria**: Filter by resolution, file type, dates, or tag logic (AND/OR).
*   [x] **Smart Collections**: Saved searches that auto-update (Smart Folders).

### 5. Metadata & Extras
*   [x] **EXIF/IPTC**: Auto-read camera data and creation dates.
*   [ ] **Custom Properties**: User-defined fields (Notes, URL source).
*   [ ] **Web Clipper**: Browser extension integration for direct imports.

### 6. Infrastructure & Internals
*   [x] **Parallel Indexing**: Rayon-powered background worker for thumbnails.
*   [x] **Resilient Database**: SQLite Upsert logic for crash recovery.
*   [ ] **Backup System**: Automated database snapshots.
*   [x] **Format Support**: Extensive support for 3D, Fonts, RAW, and Vectors.

---

## 🎨 Supported Formats

Mundam provides extensive support for various media types, categorized by their rendering and thumbnail generation capabilities.

Total registered formats: 138 extensions
*   **Native/Full Support**: 100 (Thumbnail processing + Interactive visualization)
*   **Basic Support**: 40 (Visualization available, thumbnails via format icons or stubs)
*   **Testing Base**: 209 formats monitored for future expansion.

### 🖼️ Images
| Category                   | Formats                                                                                                                                                                       | Status | Notes                                                                                              |
| :------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :----: | :------------------------------------------------------------------------------------------------- |
| **Standards**              | `jpg`, `jpeg`, `jpe`, `jfif`, `webp`, `png`, `tiff`, `gif`, `bmp`, `ico`, `tga`                                                                                               |   ✅    | Full support (Thumb + View).                                                                       |
| **Design & Paint**         | `psd`, `psb`, `ai`, `afdesign`, `afphoto`, `afpub`, `xmind`, `aseprite`, `kra`, `xcf`, `clip`, `fig`, `sketch`, `mdp`, `sai`, `sai2`, `reb`, `cdr`                            |   ✅    | Full support (Thumb + View). Some formats (e.g. SAI, MDP, CDR) might have low-res embedded thumbs. |
| **Vectors & Publishing**   | `svg`, `pdf`, `eps`, `ps`                                                                                                                                                     |   ✅    | Full support (Thumb + View).                                                                       |
| **RAW**                    | `cr2`, `cr3`, `crw`, `nef`, `nrw`, `arw`, `srf`, `sr2`, `dng`, `raf`, `orf`, `rw2`, `pef`, `erf`, `3fr`, `fff`, `dcr`, `kdc`, `srw`, `x3f`, `iiq`, `mos`, `rwl`, `mrw`, `gpr` |   ✅    | Full support (Thumb + View).                                                                       |
| **Specialized**            | `avif`, `heic`, `heif`, `cur`, `dds`, `exr`, `hdr`, `pam`, `pbm`, `pgm`, `pnm`, `ppm`                                                                                         |   ✅    | Full support (Thumb + View).                                                                       |
| **Image Projects (Stubs)** | `indd`, `idml`, `jxl`, `icns`                                                                                                                                                 |   🚧    | Basic icon/thumb, inspection/view works partially or is pending.                                   |

### 🧊 3D Models
| Category               | Formats                                                                             | Status | Notes                                                    |
| :--------------------- | :---------------------------------------------------------------------------------- | :----: | :------------------------------------------------------- |
| **Standard 3D**        | `glb`, `gltf`, `obj`, `fbx`, `stl`, `dae`, `3ds`, `dxf`, `lws`, `lwo`               |   👁️    | View only (Interactive 3D viewport, thumbnails pending). |
| **Project**            | `blend`                                                                             |   �️    | Thumbnail/Preview extracted via internal render.         |
| **USD / CAD / Sculpt** | `usdz`, `usd`, `usda`, `usdc`, `step`, `stp`, `iges`, `igs`, `zpr`, `ztl`, `sculpt` |   🚧    | Planned support (Current stub shows generic icon).       |

### 🔡 Fonts
| Formats                              | Status | Notes                                                 |
| :----------------------------------- | :----: | :---------------------------------------------------- |
| `ttf`, `otf`, `ttc`, `woff`, `woff2` |   ✅    | Full support (Extracted glyph thumb + Specimen view). |
| `eof`                                |   🚧    | Planned support.                                      |

### 🎬 Video & Audio
| Category                 | Formats                                                                                                                                      | Status | Method                                        |
| :----------------------- | :------------------------------------------------------------------------------------------------------------------------------------------- | :----: | :-------------------------------------------- |
| **Native Video**         | `mp4`, `m4v`, `mov`, `qt`                                                                                                                    |   ✅    | Native browser playback.                      |
| **Transcoded Video**     | `webm`, `wmv`, `asf`, `mkv`, `flv`, `f4v`, `avi`, `divx`, `mxf`, `ts`, `mts`, `vob`, `m2ts`, `3gp`, `3g2`, `wtv`, `rm`, `rmvb`, `ogv`        |   ✅    | HLS Streaming via background transcode.       |
| **Linear Video**         | `swf`, `mpg`, `mpeg`, `m2v`, `mjpeg`, `mjpg`, `hevc`, `h264`, `h265`, `y4m`                                                                  |   ✅    | Linear/On-the-fly HLS for raw/legacy streams. |
| **Video Projects**       | `aep`, `prproj`, `fcpxml`, `drp`, `braw`, `r3d`, `ari`                                                                                       |   🚧    | Planned support (Stub shows generic icon).    |
| **Native Audio**         | `mp3`, `wav`, `aac`, `m4a`, `m4r`, `flac`, `mp2`                                                                                             |   ✅*   | Native playback with audio visualization.     |
| **Transcoded/HLS Audio** | `ogg`, `oga`, `opus`, `wma`, `ac3`, `dts`, `wv`, `aiff`, `aif`, `aifc`, `spx`, `ra`, `mka`, `amr`, `ape`, `caf`, `aax`, `mid`, `midi`, `bwf` |   ✅*   | Transcoded playback with audio visualization. |

### 📄 Documents (Planned)
| Formats                                   | Status | Notes                                                |
| :---------------------------------------- | :----: | :--------------------------------------------------- |
| `txt`, `md`, `doc`, `docx`, `xls`, `xlsx` |   ❌    | Future expansion to index reference plaintexts/docs. |

 ---
 
 **Legend:**
 *  ✅ **Full Support**: Thumbnail generation and interactive visualization.
 *  ✅* **Audio Support**: Interactive visualization with format-specific icons as thumbnails.
 *  🖼️ **Thumb Only**: Thumbnail available, but no deep inspection/view.
 *  👁️ **View Only**: Interactive visualization available, but no thumbnail.
 *  🚧 **OS Dependent**: Behavior varies depending on system-level codecs/WebView.
 *  ❌ **No Support**: Currently not supported for preview or view.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
