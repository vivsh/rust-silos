# rust-silos

[![crates.io](https://img.shields.io/crates/v/rust-silos.svg)](https://crates.io/crates/rust-silos)
[![docs.rs](https://docs.rs/rust-silos/badge.svg)](https://docs.rs/rust-silos)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/rust-silos)](./LICENSE)

Minimal, robust file embedding for Rust. Efficient and reliable.

---

## Features

- Embed entire directories or individual files at compile time using `include_bytes!`.
- Simple API for file iteration and access by path.
- No overlays, no virtual filesystem abstraction—just embedded files.
- Robust error handling and path sanitization.

---



## Example Usage

```rust
use rust_silos::Silo;
use std::io::Read;

// Embed the "assets" directory at compile time
static ASSETS: Silo = rust_silos::embed_silo!("assets");

fn main() {
    // Access embedded files by relative path
    if let Some(file) = ASSETS.get_file("logo.png") {
        let mut bytes = Vec::new();
        file.reader().unwrap().read_to_end(&mut bytes).unwrap();
        // ... use bytes ...
    }
}
```

### Macro Usage and Options

The `embed_silo!` macro can be used as:

```rust
static ASSETS: Silo = rust_silos::embed_silo!("assets");
```

By default, in debug mode, files are read from disk for hot-reload; in release mode, files are embedded in the binary. This can be overridden:

- `force = true` — always embed files, even in debug mode.
- `force = false` — always use disk, even in release mode.
- `crate = path` — use a custom crate path for the runtime (needed if you re-export or rename the crate).

Example with options:

```rust
static ASSETS: Silo = rust_silos::embed_silo!("assets", force = true, crate = my_runtime_crate);
```


#### Why use the `crate` argument?
If you re-export or rename the `rust-silos` crate in your project, set `crate = my_runtime_crate` to ensure the macro-generated code uses the correct path to the runtime API.

---

## Switching Between Embedded and Dynamic Modes

After creating a `Silo` with the macro, you can control whether it uses embedded files or reads from disk at runtime:

- `into_dynamic()`: Always use disk (dynamic) mode, even in release builds. Useful for tests, hot-reload, or CLI tools.
- `auto_dynamic()`: Use disk in debug mode, embedded in release mode. This is the default for development–production parity. *Should be used only on an embedded silo; for other modes it is a no-op.*

Example:

```rust
let dir = ASSETS.auto_dynamic(); // disk in debug, embedded in release
let dir = ASSETS.into_dynamic(); // always disk
```

---

## Debug vs Release Behavior

- **Debug mode:** Reads files from disk at runtime (hot-reload for development).
- **Release mode:** Embeds files in the binary for maximum reliability.
- **Override:** Use the `force` argument in the macro to control this behavior explicitly.

---

## SiloSet and Override Behavior

You can compose multiple silos using `SiloSet` to support overlays and override semantics. Later silos in the set override files from earlier ones with the same relative path.

```rust
use rust_silos::{Silo, SiloSet};

static BASE: Silo = rust_silos::embed_silo!("base");
static THEME: Silo = rust_silos::embed_silo!("theme");

let set = SiloSet::new(vec![BASE, THEME]);
if let Some(file) = set.get_file("index.html") {
    // File from THEME if it exists, otherwise BASE
}
```

Overlay precedence is left-to-right. Only the highest-precedence file for each path is returned by `iter_override()`.

---

## Silo and SiloSet API

### Silo

The `Silo` struct provides a simple API for accessing embedded files:

- `new(path: &str) -> Self`: Creates a new dynamic `Silo` from the given path.
- `get_file(path: &str) -> Option<File>`: Retrieve a file by its relative path.
- `iter() -> Box<dyn Iterator<Item = File>>`: Iterate over all files in the silo.
- `is_embedded() -> bool`: Returns `true` if the silo is embedded in the binary.
- `is_dynamic() -> bool`: Returns `true` if the silo is dynamic (filesystem-backed).
- `auto_dynamic(self) -> Self`: Converts the silo to dynamic mode in debug builds; no-op in release builds. *Should be used only on an embedded silo; for other modes it is a no-op.*
- `into_dynamic(self) -> Self`: Converts the silo to dynamic mode if it is embedded; no-op otherwise.


### SiloSet

The `SiloSet` struct allows composing multiple `Silo` instances to support overlays and override semantics:

- `new(silos: Vec<Silo>) -> SiloSet`: Create a new `SiloSet` from a list of `Silo` instances.
- `get_file(path: &str) -> Option<File>`: Retrieve the highest-precedence file for a given path.
- `iter() -> impl Iterator<Item = File>`: Iterate over all files in the `SiloSet`.
- `iter_override() -> impl Iterator<Item = File>`: Iterate over files with override precedence.

Example:

```rust
use rust_silos::{Silo, SiloSet};

static BASE: Silo = rust_silos::embed_silo!("base");
static THEME: Silo = rust_silos::embed_silo!("theme");

let set = SiloSet::new(vec![BASE, THEME]);
if let Some(file) = set.get_file("index.html") {
    // File from THEME if it exists, otherwise BASE
}
```

---

## When to Use rust-silos

- You want minimal, robust, efficient and safe file embedding.
- You only need to iterate or access files by path (no overlays, no dynamic/disk mode).
- You want maximum reliability and minimal dependencies.

If you need overlays, dynamic/disk mode, or a virtual filesystem abstraction, see [`fs-embed`](https://crates.io/crates/fs-embed).

---


## Tests

This crate includes comprehensive tests for all public API. To run tests:

```sh
cargo test -p rust-silos
```

---

## License

This crate is dual-licensed under either the MIT or Apache-2.0 license, at your option.
See [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE) for details.

---

## Links

- [Documentation (docs.rs)](https://docs.rs/rust-silos)
- [Crate (crates.io)](https://crates.io/crates/rust-silos)
- [Repository (GitHub)](https://github.com/vivsh/fs-embed)
