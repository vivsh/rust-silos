//! Public proc-macro adapter for [`rust-silos`](https://crates.io/crates/rust-silos).

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{format_ident, quote};

/// Embeds a directory using the resolved `rust-silos` runtime dependency.
#[proc_macro]
pub fn embed_silo(input: TokenStream) -> TokenStream {
    rust_silos_macros_impl::embed_silo::expand(input.into(), runtime_path()).into()
}

/// Resolves the dependency name selected by the invoking crate.
fn runtime_path() -> proc_macro2::TokenStream {
    match crate_name("rust-silos") {
        Ok(FoundCrate::Itself) => quote! { ::rust_silos },
        Ok(FoundCrate::Name(name)) => {
            let ident = format_ident!("{}", name.replace('-', "_"));
            quote! { ::#ident }
        }
        Err(_) => quote! { ::rust_silos },
    }
}
