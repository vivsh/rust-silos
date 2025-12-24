//! Proc-macro for rust-silos: generates a PHF map of static str to EmbedEntry.

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use std::fs;
use std::path::Path;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, LitStr, Token,
};
use walkdir::WalkDir;

type EmbedMeta = (String, String, usize, u64);
type CollectResult = (Vec<EmbedMeta>, Vec<proc_macro2::TokenStream>);

/// Internal: Macro input parser for `silo!` macro. Accepts a path and optional force argument.
/// Path must be a string literal. Force is a bool literal.
struct SiloMacroInput {
    path: LitStr,
    force: Option<(syn::Ident, syn::LitBool)>,
    crate_path: Option<syn::Path>,
}

/// Parse implementation for macro input. Handles path and optional force argument.
impl Parse for SiloMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        let mut force = None;
        let mut crate_path = None;
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if ident == "force" {
                let value: syn::LitBool = input.parse()?;
                force = Some((ident, value));
            } else if ident == "crate" {
                let path: syn::Path = input.parse()?;
                crate_path = Some(path);
            } else {
                return Err(syn::Error::new(ident.span(), "Unknown argument to embed_silo!"));
            }
        }
        Ok(SiloMacroInput { path, force, crate_path })
    }
}

/// Macro to embed all files in a directory as a PHF map for fast, allocation-free access.
///
/// Usage: `let silo = embed_silo!("assets");` or `let silo = embed_silo!("assets", force = true);`
/// In debug mode, uses dynamic loading unless `force = true`.
/// Directory path must exist at build time for embedding.
#[proc_macro]
pub fn embed_silo(input: TokenStream) -> TokenStream {
    let SiloMacroInput { path, force, crate_path } = parse_macro_input!(input as SiloMacroInput);
    let dir_path = path.value();
    let call_span = path.span();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| String::new());
    if manifest_dir.is_empty() {
        return compile_error("embed_silo!: CARGO_MANIFEST_DIR not set", call_span);
    }
    let manifest_dir_canon = match Path::new(&manifest_dir).canonicalize() {
        Ok(p) => p,
        Err(_) => return compile_error("embed_silo!: failed to resolve CARGO_MANIFEST_DIR", call_span),
    };

    let abs_path = manifest_dir_canon.join(&dir_path);
    let abs_path = match abs_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return compile_error(
                format!("embed_silo!: failed to resolve path: {}", dir_path),
                call_span,
            )
        }
    };
    let abs_path_str = match abs_path.to_str() {
        Some(p) => p,
        None => return compile_error("embed_silo!: path must be valid UTF-8", call_span),
    };

    // Path-safe containment check (avoid prefix-string bugs like /foo/bar matching /foo/bar2).
    if !abs_path.starts_with(&manifest_dir_canon) {
        let msg = format!(
            "embed_silo!: directory not found:\n  {}\n  expected to be inside crate root:\n  {}\n  relative path: {}",
            abs_path_str,
            manifest_dir_canon.display(),
            dir_path
        );
        return compile_error(&msg, call_span);
    }

    let force_embed = force.as_ref().is_some_and(|(_, v)| v.value());
    let debug = cfg!(debug_assertions);
    let use_embed = force_embed || !debug;
    let crate_root = crate_path
        .map(|p| quote! { #p })
        .unwrap_or_else(|| quote! { ::rust_silos });

    // Keep a stable absolute root for dynamic fallback and for `into_dynamic()` conversions.
    let abs_root_lit = syn::LitStr::new(abs_path_str, call_span);
    if use_embed {
        // Generate PHF map at compile time
        let (entries, errors) = collect_embed_entries(abs_path_str, call_span);
        if !errors.is_empty() {
            return quote! { #(#errors)* }.into();
        }
        let phf_pairs = generate_phf_map(&entries, &crate_root);
        // Use a hash of the absolute path for uniqueness
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        abs_path_str.hash(&mut hasher);
        let hash = hasher.finish();
        let map_ident = quote::format_ident!("__EMBED_MAP_{:x}", hash);
        let expanded = quote! {
            {
                static #map_ident: #crate_root::phf::Map<&'static str, #crate_root::EmbedEntry> = #crate_root::phf::phf_map! {
                    #phf_pairs
                };
                #crate_root::Silo::from_embedded(&#map_ident, #abs_root_lit)
            }
        };
        expanded.into()
    } else {
        let expanded = quote! {
            #crate_root::Silo::from_static(#abs_root_lit)
        };
        expanded.into()
    }
}

/// Recursively collects all files in the given directory for embedding.
/// Returns (entries, errors):
///   - entries: Vec<(relative_path, abs_path, size, modified)>
///   - errors: Vec<TokenStream> for compile_error!s
fn collect_embed_entries(dir: &str, span: proc_macro2::Span) -> CollectResult {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let root = Path::new(dir);
    for entry in WalkDir::new(root).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                let msg = format!("embed_silo!: failed to read entry: {}", e);
                errors.push(quote_spanned! {span=> compile_error!(#msg); });
                continue;
            }
        };
        if entry.file_type().is_file() {
            let path = entry.path();
            let rel_path = match path.strip_prefix(root) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => {
                    let msg = "embed_silo!: failed to get relative path";
                    errors.push(quote_spanned! {span=> compile_error!(#msg); });
                    continue;
                }
            };
            let abs_path = match path.canonicalize() {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => {
                    let msg = format!("embed_silo!: failed to canonicalize file: {}", path.display());
                    errors.push(quote_spanned! {span=> compile_error!(#msg); });
                    continue;
                }
            };
            let size = match fs::metadata(path) {
                Ok(meta) => meta.len() as usize,
                Err(_) => 0,
            };
            let modified = match fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|mtime| mtime.duration_since(std::time::UNIX_EPOCH).ok())
            {
                Some(d) => d.as_secs(),
                None => 0,
            };
            entries.push((rel_path, abs_path, size, modified));
        }
    }

    // Make builds more reproducible across platforms/filesystems.
    entries.sort_by(|(a, _, _, _), (b, _, _, _)| a.cmp(b));
    (entries, errors)
}

// emit_compile_error removed; use quote_spanned! inline instead

/// Emit compile_error! and return from macro expansion.
fn compile_error<S: AsRef<str>>(msg: S, span: proc_macro2::Span) -> proc_macro::TokenStream {
    let lit = syn::LitStr::new(msg.as_ref(), span);
    let tokens = quote!(compile_error!(#lit));
    tokens.into()
}

/// Generates a PHF map token stream from the collected entries.
/// Used internally by the macro. Expects (rel_path, abs_path, size, modified) tuples.
fn generate_phf_map(entries: &[EmbedMeta], crate_root: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let pairs = entries.iter().map(|(rel_path, abs_path, size, modified)| {
        let rel_path_lit = syn::LitStr::new(rel_path, proc_macro2::Span::call_site());
        let abs_path_lit = syn::LitStr::new(abs_path, proc_macro2::Span::call_site());
        let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
        let mod_lit = syn::LitInt::new(&modified.to_string(), proc_macro2::Span::call_site());
        quote! {
            #rel_path_lit => #crate_root::EmbedEntry {
                path: #rel_path_lit,
                contents: include_bytes!(#abs_path_lit),
                size: #size_lit,
                modified: #mod_lit,
            },
        }
    });
    quote! {
        #(#pairs)*
    }
}
