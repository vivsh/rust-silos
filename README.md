
# Workspace: fs-embed & rust-silos


This workspace contains four crates:

- [`rust-silos`](https://crates.io/crates/rust-silos): Minimal, robust file embedding ([README](rust-silos/README.md))
- [`rust-silos-macros`](https://crates.io/crates/rust-silos-macros): Proc-macro crate for `rust-silos` (provides the embedding macro)
- [`fs-embed`](https://crates.io/crates/fs-embed): Unified virtual filesystem API ([README](fs-embed/README.md))
- [`fs-embed-macros`](https://crates.io/crates/fs-embed-macros): Proc-macro crate for `fs-embed` (provides the embedding macro)

Each crate's README contains details, API, and installation instructions.

This workspace separates minimal embedding (`rust-silos`) from full virtual filesystem abstraction (`fs-embed`), depending on your needs.