use proc_macro2::{Span, TokenStream};
use syn::Error;

pub fn compile_error(span: Span, message: impl Into<String>) -> TokenStream {
    Error::new(span, message.into()).to_compile_error()
}

pub fn diagnostics_error(span: Span, diagnostics: &[luaur_rt::TypeDiagnostic]) -> TokenStream {
    let mut message = String::from("Luau type check failed");
    for diagnostic in diagnostics {
        message.push('\n');
        message.push_str("  ");
        message.push_str(&diagnostic.to_string());
    }
    compile_error(span, message)
}
