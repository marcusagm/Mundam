# Mundam

**Mundam** is a high-performance, local-first image reference manager designed specifically for artists, concept designers, and illustrators. It allows you to organize, tag, and view massive collections of reference images with zero lag, keeping your workflow uninterrupted.

Built with **Tauri v2**, **Rust**, and **SolidJS**, Mundam combines the raw power of native code with the reactivity of modern web interfaces.

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
*   **Rust** (v1.70+)
*   **macOS / Linux / Windows** (Build tools required)

### Getting Started

1.  **Clone the repository**
    ```bash
    git clone https://github.com/marcusagm/Mundam.git
    cd mundam
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
    The binary will be available in `src-tauri/target/release/bundle/`.

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
*   [ ] **File Actions**: "Open in Explorer", "Copy to Clipboard".

### 4. Search & Filtering
*   [x] **Basic Search**: By filename.
*   [x] **Advanced Criteria**: Filter by resolution, file type, dates, or tag logic (AND/OR).
*   [x] **Smart Collections**: Saved searches that auto-update (Smart Folders).

### 5. Metadata & Extras
*   [ ] **EXIF/IPTC**: Auto-read camera data and creation dates.
*   [ ] **Custom Properties**: User-defined fields (Notes, URL source).
*   [ ] **Web Clipper**: Browser extension integration for direct imports.

### 6. Infrastructure & Internals
*   [x] **Parallel Indexing**: Rayon-powered background worker for thumbnails.
*   [x] **Resilient Database**: SQLite Upsert logic for crash recovery.
*   [ ] **Backup System**: Automated database snapshots.
*   [x] **Format Support**: Extensive support for 3D, Fonts, RAW, and Vectors.

---

## 🎨 Supported Formats

Elleven Library supports a wide range of formats out of the box, with specialized rendering for specific media types.

| Category | Formats | Support Status | Notes |
| :--- | :--- | :--- | :--- |
| **Standard Images** | JPG, PNG, WebP, GIF, BMP, ICO, TGA, TIFF | ✅ Full Support | Native performance. |
| **Vector & Design** | SVG | ✅ Full Support | Scalable rendering via WebView. |
| **Fonts** | TTF, OTF, WOFF, WOFF2, TTC | ✅ Full Support | Live preview with custom text. |
| **3D Models** | GLB, GLTF, OBJ, FBX, STL, BLEND, DAE | ✅ Full Support | Interactive 3D viewer with auto-rotate and grid. |
| **Video** | MP4, WebM, MOV | ✅ Full Support | Auto-play on hover, full controls on inspect. |
| **Raw Photos** | ARW, CR2, NEF, DNG, RAF, ORF, RW2 | 🚧 Visual Only | High-quality thumbnails generated; Full view pending. |
| **Professional Design** | PSD, AI, EPS | 🚧 Visual Only | Extracts embedded previews via Ffmpeg. |
| **Creative Apps** | Affinity (All), Clip Studio, XMind | 🚧 Visual Only | Extracts internal preview image from container. |
| **Modern Formats** | HEIC, HEIF, AVIF | ⚠️ Experimental | Logic present; depends on OS codec availability. |
| **Pro Video/HDR** | MXF, EXR, HDR | 🚧 Visual Only | Thumbnails generated; Player support varies. |

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
