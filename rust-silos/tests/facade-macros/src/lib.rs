//! Test-only facade macro backed by the reusable Rust Silos expansion crate.

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;

/// Embeds a directory through the companion `facade-runtime` crate.
#[proc_macro]
pub fn embed_silo(input: TokenStream) -> TokenStream {
    rust_silos_macros_impl::embed_silo::expand(input.into(), quote!(::facade_runtime)).into()
}
