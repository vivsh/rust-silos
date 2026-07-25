//! Test-only runtime facade that exposes Rust Silos without exposing its dependency name.

pub use rust_silos::{EmbedEntry, Silo};
pub use rust_silos_facade_macros::embed_silo;
