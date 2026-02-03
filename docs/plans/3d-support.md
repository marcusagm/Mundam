# 3D Support Implementation Plan (Assimp Pipeline)

This plan outlines the implementation of 3D file support using the "Universal Pipeline" strategy (Assimp -> GLTF).

## 🎯 Objective
Enable the application to:
1.  **Ingest** 40+ 3D file formats (FBX, OBJ, DAE, BLEND, etc.).
2.  **Convert** them to a standard web-ready format (GLB) for caching.
3.  **Render** interactive previews in the `ModelViewer`.
4.  **Generate** static thumbnails for the library grid.

## 🏗 Architecture

### 1. The Core Converter (Rust)
We will use `russimp` (Rust bindings for the Open Asset Import Library) as the core engine.
- **Input:** Any Assimp-supported format.
- **Processing:**
    - Import scene.
    - Optimizations (Mesh joining, etc. if needed).
    - Export to `.glb` (binary GLTF) stored in a persistent cache folder.
- **Output:** Path to cached `.glb`.

### 2. The Thumbnailer
Generating a 2D image from a 3D file without a window context is complex.
- **Strategy:**
    - Primary: Convert to GLB first.
    - Secondary: Attempt to extract embedded thumbnails (common in Blender/SketchUp).
    - Tertiary (Rendering): Use a headless rendering approach (e.g., `headless_chrome` pointing to a local specialized HTML render page, or a CLI tool like `f3d` if bundled).
    - **MVP Choice:** For the first iteration, we will focus on **Conversion (GLB)**. If a thumbnail cannot be easily extracted, we will use a "Generated 3D Icon" or a placeholder, but ensure the *Preview* works.

### 3. The Frontend Viewer
- Update `ModelViewer.tsx` to use `<model-viewer>` (Google's component) or `react-three-fiber` (via Solid equivalent or basic Three.js).
- Point it to the local asset URL of the cached GLB.

---

## 📅 Implementation Steps

### Phase 1: Dependencies & Setup
- [ ] **System Requirements:** Verify `Assimp` installation on the dev machine (pbrew install assimp` or standard lib).
- [ ] **Rust Dependencies:** Add `russimp` to `src-tauri/Cargo.toml`.
    - *Note:* `russimp` can be tricky with linking. We might need to configure `build.rs`.

### Phase 2: The Conversion Logic
- [ ] **Create Module:** `src-tauri/src/thumbnails/model.rs`.
- [ ] **Implement Conversion:**
    ```rust
    pub fn convert_to_glb(input: &Path, cache_dir: &Path) -> Result<PathBuf>
    ```
- [ ] **Update Registry:**
    - Modify `src-tauri/src/formats.rs` to register specific formats (FBX, DAE, OBJ) under `ThumbnailStrategy::Model3D`.
    - Generalize `Model3D` handling in `detect()`.

### Phase 3: Integration with Thumbnail Worker
- [ ] **Update `generate_thumbnail` in `mod.rs`**:
    - Add case for `ThumbnailStrategy::Model3D`.
    - Logic:
        1. Check if GLB cache exists. If not, generate it.
        2. (MVP) Generate a generic "3D Asset" thumbnail with the file extension overlay for the grid.
        3. (Advanced) If time permits, implementation of `f3d` or `headless-gl` for snapshot.

### Phase 4: Frontend Visualization
- [ ] **Update `ModelViewer.tsx`**:
    - Install `model-viewer` (Web Component) or set up a basic Three.js scene.
    - Handle loading states (conversion can take seconds).
    - Add error handling for corrupt meshes.

### Phase 5: Testing & Optimization
- [ ] **Test Formats:** FBX, OBJ, GLTF (Native), GLB.
- [ ] **Performance:** Measure conversion time.
- [ ] **Caching:** Ensure re-conversion doesn't happen unless file changes.

---

## ⚠️ Risks & Mitigations
- **Build Failures:** `russimp` relies on C++ libs.
    - *Mitigation:* If `russimp` fails to build on Mac, fallback to calling `assimp` CLI using `std::process::Command`. This is often more stable.
- **Conversion Time:** Large FBX files can be slow.
    - *Mitigation:* Async processing in the worker is already set up.
