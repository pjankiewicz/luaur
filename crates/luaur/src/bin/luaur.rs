//! `luaur` — the installable command-line entry point (`cargo install luaur`).
//!
//! Thin wrapper over `luaur-repl-cli`'s `main` (the Luau script runner / REPL,
//! a faithful port of the upstream `luau` CLI). Gated behind the `cli` feature
//! so library-only users of the umbrella crate don't build the CLI stack.

fn main() {
    luaur_repl_cli::functions::main::main();
}
