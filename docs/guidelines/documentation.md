# 📚 Documentation Standards

High-quality documentation is as important as high-quality code. This guide covers how to write effective documentation for **Mundam**.

---

## 📝 General Principles

1.  **Audience First**: Write for the person who will read it. Are they a new contributor? An end-user? A maintainer?
2.  **Keep it Fresh**: Outdated documentation is worse than no documentation. Update docs in the same PR that changes the code.
3.  **Examples**: Always provide examples. Code snippets are worth a thousand words.

---

## 🔧 Code Documentation

### JavaScript/TypeScript (TSDoc)
Use TSDoc style comments (`/** ... */`) for exported functions, classes, and interfaces.

- **@param**: Describe each parameter.
- **@returns**: Describe the return value.
- **@example**: Provide a usage example.

```ts
/**
 * Calculates the aspect ratio of an image.
 *
 * @param width - The width of the image in pixels.
 * @param height - The height of the image in pixels.
 * @returns The aspect ratio as a decimal.
 *
 * @example
 * const ratio = calculateAspectRatio(1920, 1080); // Returns 1.777...
 */
export function calculateAspectRatio(width: number, height: number): number { ... }
```

### Rust (Rustdoc)
Use triple slash (`///`) for doc comments on public items.

- Use generic Markdown for formatting.
- Include `# Examples` sections which are automatically tested via `cargo test --doc`.

```rust
/// resizing an image to fit within a bounding box.
///
/// # Arguments
///
/// * `img` - The source image buffer.
/// * `max_dim` - The maximum dimension (width or height).
///
/// # Returns
///
/// A new image buffer resized to fit.
pub fn resize_image(img: &DynamicImage, max_dim: u32) -> DynamicImage { ... }
```

---

## 📖 README Files
Each major directory (`src-tauri/src/*` or modules) should ideally have a `README.md` if the complexity warrants it.

**Structure of a good README:**
1.  **Title & Description**: What is this module?
2.  **Usage**: How do I use it?
3.  **Configuration**: What options are available?
4.  **Troubleshooting**: Common pitfalls.

---

## 📂 Project Docs (`/docs`)
- **Plans**: Use `docs/plans/` for rigorous technical planning (RFCs).
- **Ideas**: Use `docs/idea/` for brainstorming.
- **Reports**: Use `docs/report/` for post-mortems or analysis.
