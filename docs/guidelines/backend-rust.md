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

### Error Handling
- **No `unwrap()`**: Avoid `.unwrap()` or `.expect()` in production code (except in tests/initialization). Use strict error propagation with `Result` and the `?` operator.
- **Custom Errors**: Define custom error enums using `thiserror` (if available) or standard implementation to provide meaningful error context to the frontend.
- **Serialization**: Ensure errors are serializable (`derive(Serialize)`) so they can be returned to the JS frontend.

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

### SQLx Macro Safety
- **Use `sqlx::query!` and `sqlx::query_as!`**: Favor macros over string-based `sqlx::query` for all fixed queries. This ensures compile-time validation of SQL syntax and schema mapping.
- **Type Casting (`!`)**: SQLite types can be ambiguous. Use the force non-null syntax `AS "column!"` for primary keys or columns marked as `NOT NULL` in the schema to ensure the Rust type is not wrapped in `Option`.
- **Compile-time checks**:
  - Keep a `dev.db` file in `src-tauri/` synced with current migrations.
  - Maintain a `.env` file with `DATABASE_URL=sqlite:dev.db`.
  - Use `DATABASE_URL` during development so the compiler can validate queries.

### Performance & Transactions
- **Batch Operations**: NEVER perform mass insertions or updates in a loop without a transaction.
- **Transactions**: Use `pool.begin()` to wrap multiple operations. If implementing a reusable database function that might be called inside a transaction, accept `&mut sqlx::SqliteConnection` as an argument.
- **Write Optimization**: For bulk indexing, prefer `save_images_batch` patterns over individual `save_image` calls to minimize disk I/O overhead.

```rust
// ✅ Reusable internal helper for transactions/connections
async fn internal_save(
    conn: &mut sqlx::SqliteConnection, 
    data: &Data 
) -> Result<(), sqlx::Error> {
    sqlx::query!("INSERT INTO ...", data.id).execute(conn).await?;
    Ok(())
}
```
