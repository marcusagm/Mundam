# 🦀 Backend Guidelines (Rust + Tauri)

This document outlines the coding standards, patterns, and best practices for the **Mundam** backend, built with [Rust](https://www.rust-lang.org/) and [Tauri](https://tauri.app/).

---

## 🏗️ Architecture

### Command Pattern
- **Thin Commands**: Tauri commands (`#[tauri::command]`) should be thin wrappers. They should validate input, call business logic functions, and handle errors. They should rarely contain complex business logic themselves.
- **Modules**: Organize code into semantic modules (e.g., `transcoding`, `thumbnails`, `library`).
- **State Management**: Use `app.manage()` to inject state. Access state in commands using `tauri::State<T>`.

```rust
// ✅ Correct
#[tauri::command]
pub async fn generate_thumbnail(
    path: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    thumbnails::generator::process(&path, &state.config).map_err(|e| e.to_string())
}
```

---

## 📝 Coding Standards

### Naming Conventions
- **Never abbreviate variable names.** Each variable name must describe **exactly** its responsibility.
  ```rust
  // ✅ Correct
  let processed_image_buffer = ...;
  // ❌ Avoid
  let buf = ...;
  ```
- **Variables & Functions**: snake_case (e.g., `process_image`, `user_id`)
- **Types & Traits**: PascalCase (e.g., `ThumbnailStrategy`, `AppState`)
- **Constants**: SCREAMING_SNAKE_CASE (e.g., `MAX_RETRY_ATTEMPTS`)

### Coding Principles

- **Single Responsibility Principle (SRP):**
  Each function or module must have **only one clear purpose**.

- **Readability over cleverness:**
  Favor code that is **easy to understand** over complex iterator chains or macros unless necessary for performance.
  - **Avoid deeply nested `match` statements**: Break them into functions.

- **Explicit Error Handling:**
  Always handle errors. Never swallow them without logging or context.

### Error Handling & Centralized Management

We use a centralized error management system to ensure consistency, tipability, and clear communication with the frontend.

- **Centralized Enum (`AppError`)**: All errors are defined in `src/error.rs` using the `thiserror` crate. This allows for automatic conversion from external errors (SQLx, IO, Tauri) using `#[from]`.
- **Primary Result Type (`AppResult<T>`)**: Almost all functions and commands should return `AppResult<T>`, which is an alias for `Result<T, AppError>`.
- **No `unwrap()`**: Avoid `.unwrap()` or `.expect()` in production code (except in tests or unavoidable initialization). Use the `?` operator for clean propagation.
- **Frontend Serialization**: `AppError` implements `serde::Serialize` to return structured information to the JavaScript/TypeScript frontend instead of raw strings.

```rust
// ✅ Correct Example: Using AppResult in a command
#[tauri::command]
pub async fn get_data(db: State<'_, Arc<Db>>) -> AppResult<Vec<Data>> {
    // Automatic conversion from sqlx::Error to AppError via ?
    let result = db.fetch_all().await?; 
    Ok(result)
}

// ✅ Correct Example: Handling specific failures
if !path.exists() {
    return Err(AppError::NotFound(format!("Path missing: {}", path)));
}
```

- **Logging**: Errors should be logged at the point of origin or in the command handler if they indicate critical failures.

### Async/Await
- Use `.await` responsibly. Avoid blocking the async runtime (Tokio) with heavy CPU-bound tasks. Use `tokio::task::spawn_blocking` for heavy synchronous operations like image processing or file I/O.

```rust
// ✅ Correct handling of heavy work
pub async fn heavy_processing() -> Result<(), String> {
    let result = tokio::task::spawn_blocking(|| {
        // CPU intensive work here
        compute_hash()
    }).await.map_err(|e| e.to_string())?;
    
    Ok(result)
}
```

---

## 🛠️ Tooling & Quality

### Clippy
We adhere to **Clippy** lints. Warnings should be treated as errors.
Run strict checks locally:
```bash
cargo clippy -- -D warnings
```

### Formatting
- Follow standard `rustfmt` rules.
- Run `cargo fmt` before every commit.

## 📖 Documentation Patterns

### Public API & Internal Structures (Rustdoc)
All code (functions, structs, enums, traits) **MUST** have robust documentation via `///`. The goal is for any developer to understand the intent and risks without having to read the implementation.

- **Summary**: A short, direct line describing the purpose.
- **`# Errors` Section**: Mandatory if the function returns `Result`. Must list failure conditions.
- **`# Panics` Section**: Mandatory if the function contains `unwrap()`, `expect()`, or could logically panic.
- **`# Examples` Section**: Highly recommended for complex modules or global utilities.

```rust
/// Processes image resizing while maintaining aspect ratio.
///
/// # Arguments
/// * `buffer` - Byte vector of the original image.
/// * `dimensions` - Desired tuple (width, height).
///
/// # Errors
/// Returns `Err` if the image format is unsupported or if the buffer is corrupted.
///
/// # Examples
/// ```rust
/// let resized = processor::resize(my_bytes, (300, 300)).await?;
/// ```
pub async fn resize(buffer: Vec<u8>, dimensions: (u32, u32)) -> Result<Vec<u8>, Error> { ... }
```

### Module Documentation
Use `//!` at the top of files to describe the module's responsibility and how it integrates into the system. This provides the "Big Picture" necessary before diving into specific functions.

### Implementation Comments (Clean Code)
Comments within functions must follow the **"Why, not What"** rule. Code should be self-explanatory (What); comments should explain business logic or technical constraints (Why).

- **What to COMMENT**:
    - Non-obvious technical decisions (e.g., "We use 600ms debounce because the macOS file system takes time to release the lock").
    - Complex algorithms or mathematical formulas.
    - Security or performance notes.
    - `TODO` or `FIXME` with a clear description.
- **What NOT to COMMENT (Pollution)**:
    - The obvious: `let x = 10; // sets x to 10`.
    - Commented-out code: If it's not useful, delete it. History is in Git.
    - Variable descriptions: Use descriptive names instead of comments next to them.

```rust
// ❌ POLLUTION: Redundant comment
let images = db.get_images().await?; // fetches images from database

// ✅ CORRECT: Explains technical motivation
// We start a manual transaction here to avoid multiple disk flushes,
// which is critical for performance on traditional HDDs.
let mut transaction = pool.begin().await?;
```

---
 
## 🗄️ Database & SQLx
 
### Modular Database Architecture
The database layer is organized into the `src/db/` module. **NEVER** place database logic in commands or multiple flat files. Follow the domain-driven modular structure:
 
- **`db/mod.rs`**: Entry point, `Db` struct, initialization, and maintenance.
- **`db/models.rs`**: **Single source of truth** for all database-related structs (DTOs). No duplication across the system.
- **`db/images.rs`**: Image/File persistence.
- **`db/folders.rs`**: Hierarchy and location management.
- **`db/tags.rs`**: Taxonomy and relationships.
- **`db/search.rs`**: Dynamic query building with `QueryBuilder`.
 
### SQLx Macro Safety
- **Use `sqlx::query!` and `sqlx::query_as!`**: Favor macros for compile-time validation.
- **Non-Null Indicators**: SQLite nullability detection can fail. Use the "force non-null" syntax for columns you know are `NOT NULL`:
  ```sql
  SELECT id AS "id!", name FROM tags
  ```
- **Type Overrides**: For custom types (like `chrono::DateTime`), use explicit type hints in the query if detection fails:
  ```sql
  SELECT created_at AS "created_at: DateTime<Utc>" FROM images
  ```
- **Case Consistency**: Backend models should **avoid** `#[serde(rename_all = "camelCase")]` unless strictly required, to maintain consistency with the SQL schema and existing frontend property expectations (which often use `snake_case` from the API).
 
### Performance & Transactions
- **Batch Operations**: Use dedicated `batch` functions for high-volume operations (indexing).
- **Transactions**: Wrap multi-step logic in `pool.begin()`. Reusable helpers should accept `&mut SqliteConnection` or `&mut SqliteTransaction`.
- **Wal Mode**: Optimized for concurrent reads and single writer.
 
```rust
// ✅ Implementation pattern in db/domain.rs
impl Db {
    pub async fn add_item(&self, name: &str) -> Result<i64, sqlx::Error> {
        let res = sqlx::query!("INSERT INTO items (name) VALUES (?)", name)
            .execute(&self.pool)
            .await?;
        Ok(res.last_insert_rowid())
    }
}
```
