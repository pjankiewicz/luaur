//! `#[wasm_bindgen]` browser API for the Luau playground.
//!
//! These wrappers sit on top of the crate's existing `extern "C"` entry points
//! (`execute_script`, `check_script`) and the lower-level `run_code` /
//! `setup_state` logic. They are gated behind the `wasm` feature and present a
//! plain `&str` -> `String` interface that JavaScript can call directly.
//!
//! Two functions are exported:
//!
//! * [`run`]    — compile + execute the source on the VM, returning captured
//!                `print` output (and any error text).
//! * [`check`]  — type-check the source with the analyzer, returning the
//!                newline-joined diagnostics, or `"No errors."` when clean.
//!
//! ## Capturing `print`
//!
//! The VM's default `print` (`lua_b_print`) writes through `writestring` to
//! `std::io::stdout()`. On `wasm32-unknown-unknown` there is no real stdout, so
//! that output would be silently discarded. To make a browser playground
//! useful, [`run`] installs a capturing `print` global *before* the sandbox
//! freezes the global table; the capturing function appends to a thread-local
//! buffer that is drained and returned to JavaScript after the script finishes.
//! The capture mirrors `lua_b_print` exactly (tab-separated args, trailing
//! newline, `luaL_tolstring` coercion) so observable behavior is unchanged.

use alloc::string::String;
use alloc::string::ToString;
use core::cell::RefCell;
use core::ffi::c_char;
use core::ffi::c_int;
use core::ffi::CStr;

use wasm_bindgen::prelude::wasm_bindgen;

use luaur_vm::records::lua_exception::lua_exception;

use crate::functions::check_script::check_script;
use crate::functions::run_code::run_code;

use luaur_common::set_luau_bool_flags;
use luaur_vm::functions::lua_close::lua_close;
use luaur_vm::functions::lua_gettop::lua_gettop;
use luaur_vm::functions::lua_l_newstate::lua_l_newstate;
use luaur_vm::functions::lua_l_openlibs::lua_l_openlibs;
use luaur_vm::functions::lua_l_sandbox::lua_l_sandbox;
use luaur_vm::functions::lua_l_sandboxthread::lua_l_sandboxthread;
use luaur_vm::functions::lua_l_tolstring::lua_l_tolstring;
use luaur_vm::functions::lua_pushcclosurek::lua_pushcclosurek;
use luaur_vm::macros::lua_pop::lua_pop;
use luaur_vm::macros::lua_setglobal::lua_setglobal;
use luaur_vm::type_aliases::lua_state::lua_State;

#[wasm_bindgen]
extern "C" {
    // app.js installs `globalThis.__luaurOnRuntimeError` before any run. A Lua
    // runtime error is emulated with `panic_any` (luaD_throw); on stable
    // wasm32-unknown-unknown (`panic = "abort"`) that traps the instance and the
    // message would otherwise be lost. The panic hook hands the recovered error
    // text to JS HERE, before the abort — so the playground reports a typed,
    // messaged runtime error (classified by the trap being a `lua_exception`),
    // never by scanning output text.
    #[wasm_bindgen(js_namespace = globalThis, js_name = __luaurOnRuntimeError)]
    fn on_runtime_error(message: &str);
}

/// Module start hook. A Lua runtime error reaches this panic hook as a
/// `lua_exception` payload (`luaD_throw` -> `panic_any`). We recover its message
/// (`what()` reads the error object off the still-intact stack — `panic=abort`
/// does not unwind) and hand it to JS *before* the abort traps the instance, so
/// runtime errors surface with their text instead of an opaque `unreachable`
/// trap. Any other panic keeps the `console.error` diagnostic.
///
/// ## Hook ordering — why this is more than a single `set_hook`
///
/// The VM installs its OWN process-wide hook ([`install_lua_exception_panic_hook`],
/// fired lazily on the first `luaD_rawrunprotected` during state setup) that
/// *silently swallows* every `lua_exception` payload: those panics are its
/// `longjmp` emulation, not crashes, and the CLI must not print "thread panicked"
/// for a normal `error()`. That same swallowing, however, also hides the error
/// *message* — the VM hook `take_hook()`s whatever we install and then `return`s
/// early for `lua_exception`, so a naive hook here would never be reached.
///
/// So we deliberately build the chain with OUR hook outermost (it runs first):
///
/// ```text
///   ours (lua_exception -> JS bridge)  ->  VM hook  ->  console_error_panic_hook
/// ```
///
/// Force `console_error_panic_hook` as the base, force the VM hook to install on
/// top of it *now* (so its captured `previous` is the console hook, not ours),
/// then wrap that with our hook. A `lua_exception` is intercepted here and its
/// message forwarded to JS; any real Rust bug falls through to the VM hook, which
/// delegates to `console_error_panic_hook` unchanged.
#[wasm_bindgen(start)]
pub fn wasm_start() {
    use luaur_vm::functions::install_lua_exception_panic_hook::install_lua_exception_panic_hook;
    use std::panic;

    panic::set_hook(Box::new(|info| console_error_panic_hook::hook(info)));
    install_lua_exception_panic_hook();
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if let Some(exc) = info.payload().downcast_ref::<lua_exception>() {
            let p = exc.what();
            let msg = if p.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
            };
            on_runtime_error(&msg);
        } else {
            previous(info);
        }
    }));
}

thread_local! {
    /// Accumulates `print` output for the current `run` call. Drained when the
    /// VM finishes so subsequent runs start clean.
    static PRINT_BUFFER: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Capturing replacement for the VM's `print`. Mirrors `lua_b_print`: each
/// argument is coerced with `luaL_tolstring`, arguments are tab-separated, and a
/// trailing newline is appended — but the bytes go to [`PRINT_BUFFER`] instead
/// of `stdout`.
unsafe fn capturing_print(l: *mut lua_State) -> c_int {
    let n = lua_gettop(l);
    let mut line = String::new();
    for i in 1..=n {
        let mut len: usize = 0;
        let s = lua_l_tolstring(l, i, &mut len);
        if i > 1 {
            line.push('\t');
        }
        if !s.is_null() {
            let bytes = core::slice::from_raw_parts(s as *const u8, len);
            line.push_str(&String::from_utf8_lossy(bytes));
        }
        lua_pop(l, 1);
    }
    line.push('\n');
    PRINT_BUFFER.with(|b| b.borrow_mut().push_str(&line));
    0
}

/// Structured result of a [`run`] call: the program's captured `print` output
/// and, *separately*, any error text (an empty string when the run succeeded).
///
/// Keeping the two apart — rather than concatenating them into one string the
/// caller then has to guess apart — is what lets the playground classify a run
/// correctly. With a single combined string the only signal available to
/// JavaScript was a content heuristic, which both *false-positived* (legitimate
/// output containing the word "error" — e.g. iterating `_G`, which has a global
/// literally named `error` — was painted as a failure) and *false-negatived* (a
/// compile error whose text lacked the magic words was reported as success).
/// `error` non-empty ⇔ the run failed; no scanning of `output` required.
#[wasm_bindgen]
pub struct RunResult {
    output: String,
    error: String,
}

#[wasm_bindgen]
impl RunResult {
    /// The script's captured `print` output (tab-separated arguments, one line
    /// per `print`, each terminated by a newline).
    #[wasm_bindgen(getter)]
    pub fn output(&self) -> String {
        self.output.clone()
    }

    /// Error text, or the empty string when the run succeeded. In the browser
    /// build this is a compile/load error message: a genuine *runtime* error
    /// traps the WebAssembly instance (`panic = "abort"` on
    /// `wasm32-unknown-unknown`) and is surfaced by the caller's trap handler,
    /// so it never reaches here.
    #[wasm_bindgen(getter)]
    pub fn error(&self) -> String {
        self.error.clone()
    }
}

/// Compile and execute `source` on a fresh sandboxed Luau VM, returning the
/// program's captured `print` output and any error text as separate fields of a
/// [`RunResult`].
///
/// This is the browser counterpart of the crate's `extern "C"` `execute_script`
/// — it shares `setup_state` and `run_code`, but installs a capturing `print`.
#[wasm_bindgen]
pub fn run(source: &str) -> RunResult {
    // Enable the `Luau*` bool fast flags, matching `execute_script`.
    set_luau_bool_flags(true);

    unsafe {
        let l: *mut lua_State = lua_l_newstate();

        // Open the standard library + base `print`, then override `print` with
        // the capturing variant *before* the sandbox freezes the global table.
        lua_l_openlibs(l);
        lua_pushcclosurek(l, Some(capturing_print), c"print".as_ptr(), 0, None);
        lua_setglobal(l, c"print".as_ptr());

        // Freeze libraries / proxy the global table, exactly like
        // `setup_state` + `execute_script` do for the C++ web demo.
        lua_l_sandbox(l);
        lua_l_sandboxthread(l);

        // Reset the capture buffer for this run.
        PRINT_BUFFER.with(|b| b.borrow_mut().clear());

        let error = run_code(l, source);

        lua_close(l);

        let output = PRINT_BUFFER.with(|b| core::mem::take(&mut *b.borrow_mut()));
        RunResult { output, error }
    }
}

/// Type-check `source` with the analyzer (old solver) and return the
/// newline-joined `line: message` diagnostics, or `"No errors."` when clean.
///
/// This wraps the crate's `extern "C"` `check_script`, converting the returned
/// C string pointer back into an owned `String`.
#[wasm_bindgen]
pub fn check(source: &str) -> String {
    let c_source = match alloc::ffi::CString::new(source) {
        Ok(s) => s,
        Err(_) => return "error: source contains an interior NUL byte".to_string(),
    };

    let result_ptr = unsafe { check_script(c_source.as_ptr() as *const c_char, 0) };

    if result_ptr.is_null() {
        return "No errors.".to_string();
    }

    unsafe {
        core::ffi::CStr::from_ptr(result_ptr)
            .to_string_lossy()
            .into_owned()
    }
}
