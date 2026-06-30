//! Faithful port of the C++ `int replMain(int argc, char** argv)` from
//! `CLI/src/Repl.cpp`. Parses the CLI options, installs the assertion handler,
//! then either starts the interactive REPL (no file arguments) or runs each
//! source file on a fresh `lua_State`, optionally enabling profiling / coverage
//! / counters / native codegen, returning `failed ? 1 : 0`.

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, CStr};
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use luaur_cli_lib::functions::get_source_files::get_source_files;
use luaur_cli_lib::functions::set_luau_flags_flags_alt_b::set_luau_flags_c_char;
use luaur_code_gen::functions::is_supported::is_supported;
use luaur_common::functions::assert_handler::assert_handler;

use luaur_vm::functions::lua_close::lua_close;
use luaur_vm::functions::lua_l_newstate::lua_l_newstate;
use luaur_vm::records::lua_state::lua_State;

use crate::functions::assertion_handler::assertion_handler;
use crate::functions::copts::GlobalOptions;
use crate::functions::counters_dump::counters_dump;
use crate::functions::counters_init::counters_init;
use crate::functions::coverage_dump::coverage_dump;
use crate::functions::coverage_init::coverage_init;
use crate::functions::display_help::display_help;
use crate::functions::profiler_dump::profiler_dump;
use crate::functions::profiler_start::profiler_start;
use crate::functions::profiler_stop::profiler_stop;
use crate::functions::run_file::run_file;
use crate::functions::run_repl::run_repl;
use crate::functions::setup_state::setup_state;

// CLI-level statics from Repl.cpp: `static bool codegen`, `static bool
// codegenCold`, `static int program_argc`, `char** program_argv`.
static REPL_CODEGEN: AtomicBool = AtomicBool::new(false);
static REPL_CODEGEN_COLD: AtomicBool = AtomicBool::new(false);
static PROGRAM_ARGC: AtomicI32 = AtomicI32::new(0);
static mut PROGRAM_ARGV: *mut *mut c_char = core::ptr::null_mut();

/// `static bool codegen` accessor — used by setupState, the requirer and runFile.
pub fn repl_codegen_enabled() -> bool {
    REPL_CODEGEN.load(Ordering::Relaxed)
}

/// `static bool codegenCold` accessor — used by runFile.
pub fn repl_codegen_cold() -> bool {
    REPL_CODEGEN_COLD.load(Ordering::Relaxed)
}

/// `static int program_argc` accessor — used by runFile (setupArguments).
pub fn program_argc() -> i32 {
    PROGRAM_ARGC.load(Ordering::Relaxed)
}

/// `char** program_argv` accessor — used by runFile (setupArguments).
pub fn program_argv() -> *mut *mut c_char {
    unsafe { PROGRAM_ARGV }
}

// `struct GlobalOptions { int optimizationLevel = 1; int debugLevel = 1; }
// globalOptions;` — the definition backing the `extern` declaration in copts.rs.
#[export_name = "luaur_mut"]
pub(crate) static mut globalOptions: GlobalOptions = GlobalOptions {
    optimizationLevel: 1,
    debugLevel: 1,
};

#[allow(non_snake_case)]
pub fn repl_main(argc: i32, argv: *mut *mut c_char) -> i32 {
    // Luau::assertHandler() = assertionHandler;
    *assert_handler() = Some(assertion_handler_adapter);

    // (Windows) SetConsoleOutputCP(CP_UTF8) — not applicable on this build.

    let mut profile: i32 = 0;
    let mut coverage = false;
    let mut interactive = false;
    let mut codegen_perf = false;
    let mut counters = false;
    let mut program_args = argc;

    // Reset the CLI statics to the C++ defaults for this invocation.
    REPL_CODEGEN.store(false, Ordering::Relaxed);
    REPL_CODEGEN_COLD.store(false, Ordering::Relaxed);
    unsafe {
        globalOptions = GlobalOptions {
            optimizationLevel: 1,
            debugLevel: 1,
        };
    }

    let arg = |i: i32| -> String {
        unsafe {
            let p = *argv.add(i as usize);
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    };
    let argv0 = arg(0);

    let mut i = 1i32;
    while i < argc {
        let a = arg(i);

        if a == "-h" || a == "--help" {
            display_help(&argv0);
            return 0;
        } else if a == "-i" || a == "--interactive" {
            interactive = true;
        } else if a.starts_with("-O") {
            // atoi(argv[i] + 2): parse leading digits, defaulting to 0.
            let level = atoi_like(&a[2..]);
            if level < 0 || level > 2 {
                eprintln!("Error: Optimization level must be between 0 and 2 inclusive.");
                return 1;
            }
            unsafe {
                globalOptions.optimizationLevel = level;
            }
        } else if a.starts_with("-g") {
            let level = atoi_like(&a[2..]);
            if level < 0 || level > 2 {
                eprintln!("Error: Debug level must be between 0 and 2 inclusive.");
                return 1;
            }
            unsafe {
                globalOptions.debugLevel = level;
            }
        } else if a == "--profile" {
            profile = 10000; // default to 10 KHz
        } else if let Some(rest) = a.strip_prefix("--profile=") {
            profile = atoi_like(rest);
        } else if a == "--codegen" {
            REPL_CODEGEN.store(true, Ordering::Relaxed);
        } else if a == "--codegen-cold" {
            REPL_CODEGEN.store(true, Ordering::Relaxed);
            REPL_CODEGEN_COLD.store(true, Ordering::Relaxed);
        } else if a == "--codegen-perf" {
            REPL_CODEGEN.store(true, Ordering::Relaxed);
            codegen_perf = true;
        } else if a == "--coverage" {
            coverage = true;
        } else if a == "--counters" {
            counters = true;
        } else if a == "--timetrace" {
            luaur_common::FFlag::DebugLuauTimeTracing.set(true);
        } else if a.starts_with("--fflags=") {
            // setLuauFlags(argv[i] + 9)
            unsafe {
                let p = *argv.add(i as usize);
                set_luau_flags_c_char(p.add(9));
            }
        } else if a == "--program-args" || a == "-a" {
            program_args = i + 1;
            break;
        } else if a.starts_with('-') {
            eprintln!("Error: Unrecognized option '{}'.\n", a);
            display_help(&argv0);
            return 1;
        }

        i += 1;
    }

    PROGRAM_ARGC.store(argc - program_args, Ordering::Relaxed);
    unsafe {
        PROGRAM_ARGV = argv.add(program_args as usize);
    }

    // #if !defined(LUAU_ENABLE_TIME_TRACE): time tracing is compiled out.
    if luaur_common::FFlag::DebugLuauTimeTracing.get() {
        eprintln!(
            "To run with --timetrace, Luau has to be built with LUAU_ENABLE_TIME_TRACE enabled"
        );
        return 1;
    }

    if codegen_perf {
        // The --codegen-perf perf-map path is Linux-only in C++; on other
        // platforms it errors out. The Rust codegen port does not expose
        // CodeGen::setPerfLog, so we take the unsupported-platform branch.
        eprintln!("--codegen-perf option is only supported on Linux");
        return 1;
    }

    if repl_codegen_enabled() && !is_supported() {
        eprintln!("Warning: Native code generation is not supported in current configuration");
    }

    let files: Vec<String> = get_source_files(argc, argv);

    if files.is_empty() {
        unsafe {
            run_repl();
        }
        0
    } else {
        unsafe {
            let l: *mut lua_State = lua_l_newstate();

            setup_state(l);

            if profile != 0 {
                profiler_start(l, profile);
            }

            if coverage {
                coverage_init(l);
            }

            if counters {
                counters_init(l as *mut core::ffi::c_void);
            }

            let mut failed = 0i32;

            let n = files.len();
            for (idx, file) in files.iter().enumerate() {
                let is_last_file = idx == n - 1;
                let ran = run_file(file, l, interactive && is_last_file);
                failed += (!ran) as i32;
            }

            if profile != 0 {
                profiler_stop();
                profiler_dump(c"profile.out".as_ptr());
            }

            if coverage {
                coverage_dump("coverage.out");
            }

            if counters {
                counters_dump("callgrind.out");
            }

            lua_close(l);

            if failed != 0 {
                1
            } else {
                0
            }
        }
    }
}

// Adapter matching the AssertHandler fn-pointer ABI expected by Common.
unsafe extern "C" fn assertion_handler_adapter(
    expr: *const c_char,
    file: *const c_char,
    line: i32,
    function: *const c_char,
) -> i32 {
    assertion_handler(expr, file, line, function)
}

// Mirrors C's atoi(s): parse the leading optional sign + digits, ignoring any
// trailing characters; non-numeric input yields 0.
fn atoi_like(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut idx = 0;
    let mut sign = 1i32;

    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        if bytes[idx] == b'-' {
            sign = -1;
        }
        idx += 1;
    }

    let mut value: i32 = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        value = value
            .wrapping_mul(10)
            .wrapping_add((bytes[idx] - b'0') as i32);
        idx += 1;
    }

    sign * value
}
