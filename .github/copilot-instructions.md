# Rust Project Setup

This is a Rust project initialized with Cargo.

## Project Structure

- `src/` - Source code directory
  - `main.rs` - Entry point for the binary application
- `Cargo.toml` - Project manifest with dependencies and metadata
- `target/` - Build output directory (generated)

## Building and Running

Build the project:
```bash
cargo build
```

Run the project:
```bash
cargo run
```

Run tests:
```bash
cargo test
```

Check code without building:
```bash
cargo check
```

## Development

- Add dependencies in `Cargo.toml` under `[dependencies]` section
- Edit source code in `src/main.rs` or add new modules in `src/`
- Use `cargo fmt` to format code
- Use `cargo clippy` for linting suggestions
