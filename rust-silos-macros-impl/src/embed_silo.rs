//! Function-like macro expansion for embedding a directory through a supplied runtime path.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::fs;
use std::path::Path;
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
};
use walkdir::WalkDir;

type EmbedMeta = (String, String, usize, u64);
type CollectResult = (Vec<EmbedMeta>, Vec<TokenStream>);

/// Expands an `embed_silo!` invocation using `default_runtime_path` when no `crate = path` override is supplied.
///
/// Framework proc-macro crates should pass a facade path that exposes `Silo` and `EmbedEntry`.
pub fn expand(input: TokenStream, default_runtime_path: TokenStream) -> TokenStream {
    let input = match syn::parse2::<SiloMacroInput>(input) {
        Ok(input) => input,
        Err(error) => return error.into_compile_error(),
    };
    let context = match EmbedContext::resolve(&input.path) {
        Ok(context) => context,
        Err(error) => return error.into_compile_error(),
    };
    let force_embed = input.force_embed();
    let crate_root = input
        .crate_path
        .map(|path| quote! { #path })
        .unwrap_or(default_runtime_path);
    let (entries, errors) = collect_embed_entries(&context.absolute_path, context.call_span);
    if !errors.is_empty() {
        return quote! { #(#errors)* };
    }
    expand_entries(&context, force_embed, &crate_root, &entries)
}

/// Parsed `embed_silo!` arguments.
struct SiloMacroInput {
    path: LitStr,
    force: Option<syn::LitBool>,
    crate_path: Option<syn::Path>,
}

impl SiloMacroInput {
    /// Returns whether the invocation requested embedding in every build mode.
    fn force_embed(&self) -> bool {
        self.force.as_ref().is_some_and(|value| value.value)
    }
}

impl Parse for SiloMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        let mut force = None;
        let mut crate_path = None;
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.peek(Token![crate]) {
                input.parse::<Token![crate]>()?;
                input.parse::<Token![=]>()?;
                crate_path = Some(input.parse()?);
                continue;
            }
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            if ident == "force" {
                force = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    "Unknown argument to embed_silo!",
                ));
            }
        }
        Ok(Self {
            path,
            force,
            crate_path,
        })
    }
}

/// Canonical source-directory data required by the expansion.
struct EmbedContext {
    absolute_path: String,
    absolute_root: LitStr,
    call_span: Span,
}

impl EmbedContext {
    /// Resolves an invocation path and confirms that it stays within the invoking crate.
    fn resolve(path: &LitStr) -> syn::Result<Self> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|_| syn::Error::new(path.span(), "embed_silo!: CARGO_MANIFEST_DIR not set"))?;
        let manifest_dir = Path::new(&manifest_dir).canonicalize().map_err(|_| {
            syn::Error::new(
                path.span(),
                "embed_silo!: failed to resolve CARGO_MANIFEST_DIR",
            )
        })?;
        let absolute = manifest_dir
            .join(path.value())
            .canonicalize()
            .map_err(|_| {
                syn::Error::new(
                    path.span(),
                    format!("embed_silo!: failed to resolve path: {}", path.value()),
                )
            })?;
        Self::from_absolute(path, manifest_dir, absolute)
    }

    /// Validates a canonical directory path against the canonical crate root.
    fn from_absolute(
        path: &LitStr,
        manifest_dir: std::path::PathBuf,
        absolute: std::path::PathBuf,
    ) -> syn::Result<Self> {
        let absolute_path = absolute
            .to_str()
            .ok_or_else(|| syn::Error::new(path.span(), "embed_silo!: path must be valid UTF-8"))?;
        if !absolute.starts_with(&manifest_dir) {
            return Err(syn::Error::new(
                path.span(),
                format!(
                    "embed_silo!: directory not found:\n  {}\n  expected to be inside crate root:\n  {}\n  relative path: {}",
                    absolute_path,
                    manifest_dir.display(),
                    path.value()
                ),
            ));
        }
        Ok(Self {
            absolute_path: absolute_path.to_owned(),
            absolute_root: LitStr::new(absolute_path, path.span()),
            call_span: path.span(),
        })
    }
}

/// Collects sorted file metadata and compile diagnostics from one directory.
fn collect_embed_entries(dir: &str, span: Span) -> CollectResult {
    let root = Path::new(dir);
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(compile_error(
                    format!("embed_silo!: failed to read entry: {error}"),
                    span,
                ));
                continue;
            }
        };
        match collect_entry(entry, root, span) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
    }
    entries.sort_by(|(left, ..), (right, ..)| left.cmp(right));
    (entries, errors)
}

/// Converts one filesystem entry into embedding metadata when it is a regular file.
fn collect_entry(
    entry: walkdir::DirEntry,
    root: &Path,
    span: Span,
) -> Result<Option<EmbedMeta>, TokenStream> {
    if !entry.file_type().is_file() {
        return Ok(None);
    }
    let path = entry.path();
    let relative = match path.strip_prefix(root) {
        Ok(path) => path.to_string_lossy().replace('\\', "/"),
        Err(_) => {
            return Err(compile_error(
                "embed_silo!: failed to get relative path",
                span,
            ));
        }
    };
    let absolute = match path.canonicalize() {
        Ok(path) => path.to_string_lossy().to_string(),
        Err(_) => {
            return Err(compile_error(
                format!(
                    "embed_silo!: failed to canonicalize file: {}",
                    path.display()
                ),
                span,
            ));
        }
    };
    let metadata = fs::metadata(path).ok();
    let size = metadata
        .as_ref()
        .map(|metadata| metadata.len() as usize)
        .unwrap_or(0);
    let modified = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Ok(Some((relative, absolute, size, modified)))
}

/// Emits a call-site compile diagnostic with a stable macro error message.
fn compile_error(message: impl AsRef<str>, span: Span) -> TokenStream {
    let message = message.as_ref();
    quote_spanned! {span=> compile_error!(#message); }
}

/// Generates the debug/release expression for collected directory entries.
fn expand_entries(
    context: &EmbedContext,
    force_embed: bool,
    crate_root: &TokenStream,
    entries: &[EmbedMeta],
) -> TokenStream {
    let array_ident = array_identifier(&context.absolute_path);
    let array_entries = generate_sorted_array(entries, crate_root);
    let entry_count = entries.len();
    let root = &context.absolute_root;
    if force_embed {
        return quote! {
            {
                static #array_ident: [(&str, #crate_root::EmbedEntry); #entry_count] = [#array_entries];
                #crate_root::Silo::from_embedded(&#array_ident, #root)
            }
        };
    }
    quote! {
        {
            #[cfg(debug_assertions)]
            let __silo = #crate_root::Silo::from_static(#root);
            #[cfg(not(debug_assertions))]
            let __silo = {
                static #array_ident: [(&str, #crate_root::EmbedEntry); #entry_count] = [#array_entries];
                #crate_root::Silo::from_embedded(&#array_ident, #root)
            };
            __silo
        }
    }
}

/// Produces a collision-resistant static-array identifier for one embedded directory.
fn array_identifier(absolute_path: &str) -> syn::Ident {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    absolute_path.hash(&mut hasher);
    quote::format_ident!(
        "__EMBED_ARRAY_{:x}_{}",
        hasher.finish(),
        absolute_path.len()
    )
}

/// Generates sorted static entry expressions for binary-search lookup.
fn generate_sorted_array(entries: &[EmbedMeta], crate_root: &TokenStream) -> TokenStream {
    let pairs = entries.iter().map(|(relative, absolute, size, modified)| {
        let relative = LitStr::new(relative, Span::call_site());
        let absolute = LitStr::new(absolute, Span::call_site());
        let size = syn::LitInt::new(&size.to_string(), Span::call_site());
        let modified = syn::LitInt::new(&modified.to_string(), Span::call_site());
        quote! {
            (#relative, #crate_root::EmbedEntry {
                path: #relative,
                contents: include_bytes!(#absolute),
                size: #size,
                modified: #modified,
            }),
        }
    });
    quote! { #(#pairs)* }
}

#[cfg(test)]
mod tests {
    use super::SiloMacroInput;
    use quote::quote;

    /// Verifies that the reserved `crate` keyword is accepted as the runtime-path option.
    #[test]
    fn parses_crate_keyword_as_runtime_path_option() {
        let input =
            syn::parse_str::<SiloMacroInput>(r#""assets", force = true, crate = runtime_alias"#)
                .expect("the documented crate option should parse");

        let crate_path = input
            .crate_path
            .expect("crate option should set the runtime path");
        assert_eq!(quote!(#crate_path).to_string(), "runtime_alias");
        assert!(input.force.is_some_and(|value| value.value));
    }
}
