# Repository Guidelines

## Project Structure & Module Organization
CmdHub is a Rust workspace with multiple binaries and a shared core. The main crates live in `core/`, `cli/`, `server/`, and `desktop/`. Each crate has its own `Cargo.toml` and `src/` folder. High-level product planning and architecture notes live in `docs/` and `PLAN.md`. Runtime task definitions are stored in the root `config.toml`.

## Build, Test, and Development Commands
- `cargo build`: build all workspace crates.
- `cargo run -p cmdhub-cli`: run the TUI client.
- `cargo run -p cmdhub-server`: start the Axum server.
- `cargo run -p cmdhub-desktop`: run the desktop binary placeholder.
- `cargo fmt`: format Rust code (uses rustfmt defaults).

## Coding Style & Naming Conventions
- Rust edition is 2021 across all crates.
- Follow standard Rust style: 4-space indentation, `snake_case` for functions/modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Keep module boundaries aligned with crate intent (core logic in `core/`, UI/UX in `cli/` or `desktop/`, web server in `server/`).

## Testing Guidelines
There are no dedicated tests yet. If adding tests, use Rust’s built-in test framework in `src/` or a `tests/` directory per crate, and run with `cargo test` or `cargo test -p <crate>`.

## Commit & Pull Request Guidelines
- Commit messages currently follow a simple prefix style (example: `feat: 初始化项目结构并实现核心功能原型`). Keep messages short, present-tense, and scoped.
- Pull requests should describe the change, link related issues if available, and mention any user-facing behavior changes.

## Configuration Tips
- Task definitions live in `config.toml`. Use unique `id` values, and keep commands portable.
- Example task entry:
  ```toml
  [[tasks]]
  id = "echo-hello"
  name = "Echo Hello"
  command = "echo 'Hello from CmdHub'"
  ```
