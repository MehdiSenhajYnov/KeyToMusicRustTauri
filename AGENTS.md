# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the React 18 + TypeScript UI. Keep UI code split by responsibility: `components/` for views, `stores/` for Zustand state, `hooks/` for reusable behavior, `utils/` for helpers and Tauri command wrappers, and `types/` for shared TypeScript models. `src-tauri/src/` contains the Rust backend, organized by domain (`audio/`, `keys/`, `youtube/`, `discovery/`, `mood/`, `storage/`, `import_export/`). Tauri config lives in `src-tauri/tauri.conf.json`. Static assets live in `resources/`; planning notes live in `Tasks/`. Runtime data in `data/` and build output in `src-tauri/target/` are generated and should stay uncommitted.

## Build, Test, and Development Commands
Use `npm install` to install frontend and Tauri CLI dependencies. `npm run dev` starts the Vite frontend on port `5173`; `npm run tauri:dev` runs the full desktop app with Rust + frontend hot reload. `npm run build` runs `tsc` and builds the web bundle into `dist/`. `npm run tauri:build` creates a desktop build. For backend quality checks, use `cargo fmt --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml`.

## Coding Style & Naming Conventions
Follow the existing frontend style: 2-space indentation, double quotes, semicolons, and strict TypeScript. Name React components with `PascalCase` (`Header.tsx`), hooks with `use...` (`useKeyDetection.ts`), Zustand stores with `...Store.ts`, and utility files with descriptive lower camel or domain names such as `tauriCommands.ts`. Rust files use `snake_case`, types use `CamelCase`, and formatting should be left to `cargo fmt`.

## Testing Guidelines
Automated tests are currently Rust-focused. Add unit tests in `#[cfg(test)]` blocks close to the code you change, following examples in `src-tauri/src/keys/chord.rs`, `src-tauri/src/mood/director.rs`, and `src-tauri/src/audio/analysis.rs`. Run `cargo test --manifest-path src-tauri/Cargo.toml` before opening a PR. There is no frontend test runner configured yet, so UI changes should include clear manual verification steps.

## Commit & Pull Request Guidelines
Recent history follows Conventional Commit prefixes: `feat:`, `fix:`, and `chore:` with short imperative subjects, for example `fix: resolve stuck keys in global shortcuts`. Keep commits focused and avoid mixing version bumps or task-file churn into feature commits when possible. PRs should include a brief summary, linked issue or task, commands run, and screenshots or recordings for UI changes. Call out platform-specific effects when touching keyboard capture, audio devices, or Tauri permissions.

## Configuration & Assets
Do not commit `.env*`, `data/`, `src-tauri/target/`, or auto-downloaded binaries such as `yt-dlp` and `ffmpeg`. Put reusable packaged assets in `resources/`; user-specific caches, imported sounds, logs, and generated files belong under `data/` at runtime.
