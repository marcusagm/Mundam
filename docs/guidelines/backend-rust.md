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

### Documentation (Rustdoc)
- Use `///` for function/struct documentation.
- Document **Panics**: If a function can panic, explicitly state under what conditions.
- Document **Errors**: Explain what errors can be returned.

```rust
/// Configures the FFmpeg process.
/// 
/// # Errors
/// Returns an error if the executable path is invalid.
pub fn configure_ffmpeg() -> Result<Config, Error> { ... }
```
