# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs`: Single entry point; keeps argument parsing, scheduling, and HTTP interaction logic together. Add new functionality via small helper modules in `src/` to keep `main.rs` lean.
- `Cargo.toml` / `Cargo.lock`: Dependency and edition (2024) definitions; keep in sync with reproducible locks.
- `target/`: Build artifacts; not committed.
- Docker context expects `Cargo.toml`, `Cargo.lock`, and `src/` at repo root.

## Build, Test, and Development Commands
- `cargo build` / `cargo build --release`: Local debug or optimized build. Docker image builds release by default.
- `cargo test`: Run the Rust test suite (unit/integration).
- `cargo fmt && cargo clippy --all-targets --all-features`: Format and lint before pushing.
- `docker build -t tianyi-auto .`: Build container image using the nightly toolchain baked in `Dockerfile`.
- `docker-compose up --build -d`: Build and start the service with host networking; respects `ROUTER_PASSWORD` env var.

## Coding Style & Naming Conventions
- Rust 2024 edition with stable `rustfmt` defaults; prefer explicit `use` paths and small functions.
- Name binaries, flags, and env vars in kebab-case; Rust identifiers in snake_case; types in UpperCamelCase.
- Prefer `anyhow::Result` for fallible flows and `log` macros for reporting; initialize logging early.

## Testing Guidelines
- Co-locate unit tests in the same module using `#[cfg(test)]`; name tests after the behavior (`handles_invalid_cookie`).
- For CLI behaviors, add integration tests under `tests/` using `assert_cmd`-style patterns where practical.
- Run `cargo test` before PRs; add edge cases around scheduling, credential handling, and HTTP failures.

## Commit & Pull Request Guidelines
- Commits: concise, present-tense summaries; scope one logical change (e.g., `feat: add cron flag validation`).
- PRs: include what/why, key commands run (`cargo test`, `cargo fmt`, `cargo clippy`), and behavior notes for cron/auth changes. Link issues when available.
- Avoid force-pushes over shared branches; rebase onto main before opening PRs when possible.

## Security & Configuration Tips
- Keep router credentials in env vars (`ROUTER_PASSWORD`) or secrets managers; avoid committing real values.
- Host networking is required to reach the LAN router; review `docker-compose.yml` before changing network mode.
- Docker build uses nightly Rust for edition 2024; if you bump Rust, ensure the toolchain matches.
