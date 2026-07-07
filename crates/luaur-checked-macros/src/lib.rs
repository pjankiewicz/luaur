//! Compile-time checked Luau source macros for `luaur`.
//!
//! This crate is intentionally separate from `luaur-rt-derive`: derive users
//! should not pay for Luau's static typechecker.

use proc_macro::TokenStream;

mod expand_file;
mod expand_inline;
mod file_input;
mod inline_input;
mod module_entry;
mod paths;
mod report;

/// Type-check an inline Luau source string at Rust compile time.
///
/// Supported forms:
///
/// ```ignore
/// luaur::luau!("--!strict\nreturn 1")
///
/// luaur::luau! {
///     source = "--!strict\nlocal M = require(\"@m\")\nreturn M.x",
///     module = "main",
///     modules = {
///         "@m" => "--!strict\nreturn { x = 1 }",
///     },
/// }
/// ```
#[proc_macro]
pub fn luau(input: TokenStream) -> TokenStream {
    expand_inline::expand(input.into()).into()
}

/// Type-check a Luau source file at Rust compile time and expand to
/// `include_str!(...)`.
///
/// Supported forms:
///
/// ```ignore
/// luaur::luau_file!("scripts/main.luau")
///
/// luaur::luau_file! {
///     root = "scripts/main.luau",
///     module = "game/Main",
///     modules = {
///         "game/Math" => "scripts/math.luau",
///         "@config" => "scripts/config.luau",
///     },
/// }
/// ```
#[proc_macro]
pub fn luau_file(input: TokenStream) -> TokenStream {
    expand_file::expand(input.into()).into()
}
