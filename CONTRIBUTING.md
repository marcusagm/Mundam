# 🧩 Contributing to Mundam

Thank you for your interest in contributing to **Mundam**!  
This document serves as the entry point for our contribution guidelines. To ensure the project remains high-quality and maintainable, we have split our standards into focused guides.

---

## 🧭 Guidelines

Please read the specific guide relevant to your contribution:

### 🎨 [Frontend Guidelines (Solid.js + TS)](./docs/guidelines/frontend-solid.md)
*For everything related to the UI, components, state management, and styling.*

### 🦀 [Backend Guidelines (Rust + Tauri)](./docs/guidelines/backend-rust.md)
*For Tauri commands, file system operations, image processing, and core logic.*

### 📚 [Documentation Standards](./docs/guidelines/documentation.md)
*How to write TSDoc, Rustdoc, and general project documentation.*

### 🌲 [Git Workflow](./docs/guidelines/git-workflow.md)
*Branch naming, conventional commits, and Pull Request process.*

---

## 🚀 Getting Started

1. **Clone the repository**
    ```bash
    git clone https://github.com/marcusagm/Mundam.git
    cd Mundam
    ```

2. **Install dependencies**
    ```bash
    npm install
    ```
    *Note: Ensure you have Rust and Cargo installed.*

3. **Run the development server**
    ```bash
    npm run tauri dev
    ```

---

## ⚙️ Quality Checklist

Before submitting a Pull Request, please ensure:

- [ ] **Linting**: No lint errors in console (`npm run lint` / `cargo clippy`).
- [ ] **Formatting**: Code matches Prettier/Rustfmt standards.
- [ ] **Tests**: New logic is covered by tests (`npm test` / `cargo test`).
- [ ] **Build**: The project builds successfully (`npm run build`).

---

## 🤝 Code of Conduct

We are committed to providing a friendly, safe, and welcoming environment for all, regardless of level of experience, gender identity and expression, sexual orientation, disability, personal appearance, body size, race, ethnicity, age, religion, nationality, or other similar characteristic.

Please be respectful and constructive in all code reviews and discussions.

---

🧡 _Marcus Maia_  
Creator & Maintainer — [Mundam](https://github.com/marcusagm/Mundam)
