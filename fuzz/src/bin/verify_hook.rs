//! Prove `install_afl_panic_hook` routes correctly:
//!   verify_hook compileerror  -> CompileError unwinds + is catchable (exit 0)
//!   verify_hook realpanic     -> any other panic aborts (SIGABRT, exit 134)
//! Run both and check exit codes — confirms real bugs still surface to AFL while
//! the compiler's intentional CompileError throw does not become a false crash.
use luaur_ast::records::location::Location;

fn main() {
    luaur_fuzz::install_afl_panic_hook();
    match std::env::args().nth(1).as_deref() {
        Some("compileerror") => {
            let r = std::panic::catch_unwind(|| {
                // the real raise path: panic_any(CompileError)
                luaur_compiler::methods::compile_error_raise::compile_error_raise(
                    Location::default(),
                    format_args!("boom"),
                );
            });
            // hook returned (no abort) -> the panic unwound and was caught here.
            eprintln!("compileerror: caught = {}", r.is_err());
            std::process::exit(0);
        }
        Some("realpanic") => {
            // hook should abort BEFORE this catch_unwind ever sees it.
            let _ = std::panic::catch_unwind(|| panic!("genuine bug"));
            eprintln!("realpanic: NOT aborted (BUG — hook let a real panic through)");
            std::process::exit(0);
        }
        _ => {
            eprintln!("usage: verify_hook compileerror|realpanic");
            std::process::exit(2);
        }
    }
}
