# Elleven-Library: High-Performance Image Indexer (Rust)

## Goal
Implement a high-performance image indexer using a Producer-Consumer pattern in Rust. The system will perform deep folder scanning, extract metadata via header scanning, and provide adaptive progress updates to the UI.

## Tasks
- [x] Task 1: Backend Dependencies → Add `sqlx`, `walkdir`, `imagesize`, and `serde` to `src-tauri/Cargo.toml`.
- [x] Task 2: Metadata Core Logic → Implement the `header_scanner` module using `imagesize` to extract width, height, and format without reading full files.
- [x] Task 3: Worker Pool Architecture → Create the `Indexer` service in Rust using `tokio::sync::mpsc` for communication between the Scanner (Producer) and the Meta-Extractor (Workers).
- [x] Task 4: Adaptive Progress Report → Implement the scaling logic for progress events (`tauri::Emitter`) that adjusts update frequency based on the total number of files.
- [x] Task 5: Database Integration → Implement Batch Inserts in Rust to save the discovered images into the SQLite database in chunks (e.g., every 100 images) for stability.
- [x] Task 6: Tauri Commands & UI → Expose the `start_indexing` command and update `App.tsx` to display real-time progress and refresh the grid.

## Done When
- [x] Initial scan of 1000+ files completes in under 2 seconds.
- [x] UI receives adaptive progress updates without "lagging" the main thread.
- [x] Metadata (width/height) is correctly stored in the SQLite `images` table.
- [x] File Watcher (notify) successfully detects a new image added to the folder and initiates a single-file index update.

## Notes
- **Performance**: We are avoiding Full File Hashing (P2) to keep IO overhead low.
- **Precision**: Using `imagesize` as a lightweight alternative to full image decoders.
- **UX**: The adaptive chunking ensures fluid animations even with 100k+ assets.
