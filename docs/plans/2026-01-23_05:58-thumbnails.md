# PLAN: High-Performance Thumbnail Generation & UX Polish

## Goal
Implement a background thumbnail generation system to optimize gallery performance and fix UI "stuttering" during interactions. Thumbnails will be stored in the System AppData directory to keep user folders clean.

## Phase 1: Infrastructure & Dependencies (Rust)
- [x] **Task 1: Add Image Processing Crates** → Add `image` (with `webp` features) and `fast_image_resize` to `src-tauri/Cargo.toml`.
- [x] **Task 2: AppData Management** → Create a utility to resolve and maintain the `thumbnails/` directory within the application's local data folder.
- [x] **Task 3: Schema Update** → Update the SQLite schema to include `thumbnail_path`, `width`, and `height` columns.

## Phase 2: Background Processor (Rust)
- [x] **Task 1: Thumbnail Worker** → Implement a low-priority background thread that watches for images missing thumbnails in the database.
- [x] **Task 2: Resize Logic** → Generate **300px** (width-bound) WebP thumbnails using `fast_image_resize`.
- [x] **Task 3: Dispatcher** → Emit a `thumbnail:ready` event to the frontend whenever a new batch of thumbnails is persisted.

## Phase 3: Frontend Refinement & Lazy Loading (SolidJS)
- [x] **Task 1: Smart Image Component** → Create a `ReferenceImage` component that handles:
    - Displaying a **Placeholder/Skeleton** while the thumbnail is missing.
    - Transitioning smoothly using aspect-ratio stability.
- [x] **Task 2: Logic Update** → Update `App.tsx` and `ImageGrid.tsx` to use optimized data structures (Stores + Reconcile).

## Phase 4: UX Polish & Animation Fix
- [x] **Task 1: Hover Animation Optimization** → Refactor CSS to use `transform` and `opacity` transition properties only, avoiding layout-recalculating properties like `all`.
- [x] **Task 2: Reveal Animation** → Add a subtle Fade-In when images load.

## Done When
- [x] Gallery handles indexing and thumbnail generation smoothly.
- [x] Hover effects feel "buttery smooth".
- [x] Background processing works for new folders.
- [x] Disk usage is optimized via WebP.

---

## 🧠 Unplanned Implementations & Technical Decisions

During the implementation, several critical roadblocks required expanding the original scope. Below is the summary of these decisions:

### 1. Custom URI Protocols (`thumb://` and `orig://`)
- **Problem**: The standard `asset://` protocol from Tauri proved unreliable and restricted. It had issues with URI encoding (especially spaces in "Application Support") and often failed to load assets from outside the immediate bundle scope on macOS (Image failed to load: 2).
- **Decision**: Implemented two custom protocols: `thumb://localhost/` for thumbnails and `orig://localhost/` for original user files.
- **Reasoning**: This bypasses browser security sandboxes while providing a high-performance, controlled bridge that returns correct MIME types (`image/webp`, `image/jpeg`) and supports CORS directly from Rust.

### 2. Aspect-Ratio Stability (Metadata-Driven Layout)
- **Problem**: The Masonry grid suffered from significant "flickering" and layout shifts as thumbnails loaded and substituted placeholders, because the height of the elements changed dynamically.
- **Decision**: Expanded indexing to capture image dimensions (`width`, `height`) and applied `aspect-ratio` CSS property dynamically in the `ReferenceImage` component.
- **Reasoning**: By reserving the exact geometric space *before* the image loads, we eliminate Cumulative Layout Shift (CLS), making the scroll experience rock-solid.

### 3. SolidJS Store + Reconcile Integration
- **Problem**: Using simple Signals for the image list caused full grid re-renders on every update (new indexing progress or thumbnail ready), which reset the "loaded" state of every image component, causing repeated flickering.
- **Decision**: Switched to `createStore` with `reconcile(data, { key: 'id' })`.
- **Reasoning**: This allows SolidJS to perform granular DIFF updates on the DOM. If only the `thumbnail_path` of an image changes, only that component is updated, preserving the scroll position and the state of all other images.

### 4. Advanced URI Decoding (Percent-Encoding)
- **Problem**: User files often contain special characters (accents, spaces, emoji) that get garbled when passed over the IPC as URIs.
- **Decision**: Added the `percent-encoding` crate in Rust to manually decode paths in the custom protocol handlers.
- **Reasoning**: Ensures that images like `mujer-con-cuerpo-atlético.webp` load correctly across all platforms regardless of filename complexity.
