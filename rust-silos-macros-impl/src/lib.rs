//! Reusable `embed_silo!` expansion support for framework proc-macro crates.
//!
//! Application crates should depend on `rust-silos`. A framework proc-macro can
//! call [`embed_silo::expand`] with the path of its runtime facade.
//!
//! ```ignore
//! #[proc_macro]
//! pub fn embed_silo(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
//!     rust_silos_macros_impl::embed_silo::expand(
//!         input.into(),
//!         quote::quote!(::my_framework::embed),
//!     )
//!     .into()
//! }
//! ```

pub mod embed_silo;
