use proc_macro2::TokenStream;
use quote::quote;
use std::path::{Path, PathBuf};
use syn::{Error, LitStr, Result};

pub fn read_manifest_file(path: &LitStr) -> Result<String> {
    let full_path = full_path(path)?;
    std::fs::read_to_string(&full_path).map_err(|err| {
        Error::new(
            path.span(),
            format!("failed to read `{}`: {err}", full_path.display()),
        )
    })
}

pub fn include_str_expr(path: &LitStr) -> TokenStream {
    let value = path.value();
    if Path::new(&value).is_absolute() {
        quote! { include_str!(#path) }
    } else {
        quote! { include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #path)) }
    }
}

fn full_path(path: &LitStr) -> Result<PathBuf> {
    let value = path.value();
    let path_ref = Path::new(&value);
    if path_ref.is_absolute() {
        return Ok(path_ref.to_path_buf());
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|err| Error::new(path.span(), format!("CARGO_MANIFEST_DIR is not set: {err}")))?;
    Ok(Path::new(&manifest_dir).join(path_ref))
}
