# 🌲 Git Workflow & Version Control

We follow a structured Git workflow to ensure history cleanliness and ease of collaboration.

---

## 🌿 Branch Naming

Use the following prefixes for your branches:

- **`feat/`**: A new feature (e.g., `feat/add-video-player`)
- **`fix/`**: A bug fix (e.g., `fix/memory-leak-thumbnails`)
- **`docs/`**: Documentation changes (e.g., `docs/update-contributing`)
- **`refactor/`**: Code change that neither fixes a bug nor adds a feature (e.g., `refactor/simplify-state`)
- **`chore/`**: Maintenance tasks, dependency updates (e.g., `chore/bump-tauri-version`)

**Example:**
```bash
git checkout -b feat/improved-search-algorithm
```

---

## 💾 Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/). This allows us to generate changelogs automatically.

**Format:**
`<type>(<scope>): <subject>`

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, missing semi-colons (no code change)
- `refactor`: Refactoring production code
- `perf`: Performance improvements
- `test`: Adding tests
- `chore`: Build tools, auxiliary files

**Example:**
```
feat(video): add support for .mkv playback
fix(ui): resolve overlap in sidebar items
docs(readme): update installation steps
```

---

## 🔄 Pull Request (PR) Process

1.  **Sync First**: Always rebase your branch on `main` before opening a PR.
    ```bash
    git fetch origin
    git rebase origin/main
    ```
2.  **Lint & Test**: Ensure all checks pass locally.
    ```bash
    npm run lint
    npm test
    cargo test
    ```
3.  **Description**: Fill out the PR template. Explain **what** changed and **why**.
4.  **Review**: Wait for at least one approval. Address comments constructively.
5.  **Squash**: We typically squash commits on merge to keep `main` clean, unless the specific history is critical.
