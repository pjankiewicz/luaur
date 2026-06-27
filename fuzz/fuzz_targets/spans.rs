// Diagnostic-span oracle (doc: "error spans should be valid", "diagnostics
// should not panic"). Type-check diverse programs and assert every diagnostic
// has a sane source span (1-based, end not before begin, line within the
// program) and does NOT leak an internal compiler assertion / ICE into its
// user-facing message. Out-of-range spans crash editors/LSPs; an assertion
// string in a diagnostic is an internal-compiler-error leak. (The reusable
// Checker doesn't catch_unwind, so a checker panic also surfaces as a crash.)

use std::cell::RefCell;

#[cfg(feature = "afl-runtime")]
use afl::fuzz;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

thread_local! {
    static CHECKER: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
}

fn exercise_input(data: &[u8]) {
    // Mix type-rich generated programs with spliced real scripts — both produce
    // lots of diagnostics, exercising the span/message paths broadly.
    let src = if data.first().map(|b| b & 1 == 0).unwrap_or(true) {
        luaur_fuzz::generate_typed(data)
    } else {
        luaur_fuzz::generate_spliced(data)
    };
    let n_lines = src.lines().count() as u32;

    if let Err(diags) = CHECKER.with(|c| c.borrow_mut().check(&src)) {
        for d in &diags {
            assert!(
                d.line >= 1 && d.column >= 1 && d.end_line >= 1 && d.end_column >= 1,
                "diagnostic span is not 1-based: {d:?}\n--- src ---\n{src}"
            );
            assert!(
                d.end_line >= d.line,
                "diagnostic end line before begin line: {d:?}\n--- src ---\n{src}"
            );
            // Lenient upper bound (allow an at-EOF position) — only flag spans
            // pointing well outside the program.
            assert!(
                d.line <= n_lines + 2 && d.end_line <= n_lines + 2,
                "diagnostic line {} out of range (program has {n_lines} lines): {d:?}\n--- src ---\n{src}",
                d.line
            );
            let m = d.message.to_ascii_lowercase();
            assert!(
                !m.contains("luau_assert") && !m.contains("internal error") && !m.contains("ice:"),
                "diagnostic message leaks an internal compiler assertion: {:?}\n--- src ---\n{src}",
                d.message
            );
        }
    }
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    fuzz!(|data: &[u8]| {
        exercise_input(data);
    });
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
