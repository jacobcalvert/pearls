# Contributing to Pearls

Thanks for your interest in contributing. This project is a lightweight CLI for managing a task graph, and it is intentionally small and focused.

## Development Setup

Requirements:
- Rust stable toolchain (via `rustup`)
- SQLite (for local development and tests)

Clone the repo and build:

```bash
cargo build
```

Run the CLI:

```bash
cargo run -- --help
```

By default, the CLI uses `./pearls.db`. Override the database path with `--db <path>` or `PEARLS_DB`.

## Development Workflow

Common commands:

```bash
cargo run -- tasks list
cargo run -- tasks add --title "Example" --description "Example description"
```

## Quality Checks

Before submitting a change, run:

```bash
cargo clippy
cargo test
```

## Pull Request Notes

Please include:
- A clear summary of the change
- Any relevant context or tradeoffs
- Tests run (or why they were skipped)
