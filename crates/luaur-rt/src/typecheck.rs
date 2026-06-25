//! Static type-checking of Luau source against the host surface (the
//! `typecheck` feature).
//!
//! This is the unique capability `mlua` cannot offer: because luaur ships
//! Luau's static type checker (`luaur-analysis`), a script you are about to run
//! can be type-checked against exactly the API the host exposes *before* it
//! runs. The Luau VM is dynamically typed, so the runtime does not need any of
//! this — but the *static* checker has no knowledge of the host surface unless
//! you tell it.
//!
//! Modelled exactly on the umbrella `luaur` crate's `check` helper (itself a
//! port of `luaur-web`'s `check_script`): build a [`Frontend`] over an in-memory
//! single-source file resolver, register the Luau builtins, optionally load host
//! type definitions into the same global scope, insert the source as the module
//! `"main"`, and type-check it on the validated **old** solver.
//!
//! The one difference from the umbrella's helper is the diagnostic shape: each
//! diagnostic is surfaced as a structured [`TypeDiagnostic`] carrying its source
//! location (line/column, 1-based) rather than a flat `"<line>: <message>"`
//! string.

use luaur_analysis::enums::solver_mode::SolverMode;
use luaur_analysis::functions::freeze::freeze;
use luaur_analysis::functions::register_builtin_globals::register_builtin_globals;
use luaur_analysis::functions::to_string_error::to_string_type_error;
use luaur_analysis::functions::unfreeze::unfreeze;
use luaur_analysis::records::config_resolver::ConfigResolver;
use luaur_analysis::records::file_resolver::{FileResolver, FileResolverVtable};
use luaur_analysis::records::frontend::Frontend;
use luaur_analysis::records::frontend_options::FrontendOptions;
use luaur_analysis::records::module_info::ModuleInfo;
use luaur_analysis::records::source_code::SourceCode;
use luaur_analysis::records::type_check_limits::TypeCheckLimits;
use luaur_analysis::type_aliases::module_name_file_resolver::ModuleName;
use luaur_ast::records::ast_expr::AstExpr;
use luaur_config::records::config::Config;

use core::fmt;

/// One type-checker diagnostic with its source location (all 1-based).
///
/// Produced by [`check`] / [`check_with_definitions`] and carried inside
/// [`Error::TypeError`](crate::Error::TypeError). Unlike a flat error string,
/// the location fields let an editor / build tool point at the exact span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDiagnostic {
    /// 1-based start line.
    pub line: u32,
    /// 1-based start column.
    pub column: u32,
    /// 1-based end line.
    pub end_line: u32,
    /// 1-based end column.
    pub end_column: u32,
    /// The human-readable diagnostic message.
    pub message: String,
    /// True when the diagnostic comes from the host `declare` definitions rather
    /// than the checked script.
    pub in_definitions: bool,
}

impl fmt::Display for TypeDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.in_definitions {
            write!(f, "(host definitions) ")?;
        }
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

/// The fixed module name under which the checked source is registered, matching
/// the umbrella helper's `fileResolver.source["main"] = source`.
const MAIN_MODULE: &str = "main";

/// Minimal single-source in-memory [`FileResolver`] for a string check.
///
/// `#[repr(C)]` with `base` first so the vtable thunks can cast the
/// `*mut FileResolver` receiver back to `*mut CheckFileResolver` and reach
/// `source`. Holds exactly one module's source ("main").
#[repr(C)]
struct CheckFileResolver {
    base: FileResolver,
    source: String,
}

/// `readSource` thunk — returns the single source for the `"main"` module.
///
/// # Safety
/// `this` must point at the `base` subobject of a live `CheckFileResolver`.
unsafe fn check_file_resolver_read_source_thunk(
    this: *mut FileResolver,
    name: &ModuleName,
) -> Option<SourceCode> {
    let this = this as *const CheckFileResolver;
    if name != MAIN_MODULE {
        return None;
    }
    // SAFETY: per this fn's contract, `this` points at a live `CheckFileResolver`.
    let source = unsafe { (*this).source.clone() };
    Some(SourceCode {
        source,
        r#type: SourceCode::Module,
    })
}

/// `resolveModule` thunk — no require support for a string check, so always
/// `None`.
///
/// # Safety
/// `this` must point at the `base` subobject of a live `CheckFileResolver`.
unsafe fn check_file_resolver_resolve_module_thunk(
    _this: *mut FileResolver,
    _context: *const ModuleInfo,
    _expr: *mut AstExpr,
    _limits: &TypeCheckLimits,
) -> Option<ModuleInfo> {
    None
}

/// `getHumanReadableModuleName` thunk — returns the name verbatim.
///
/// # Safety
/// `this` must point at the `base` subobject of a live `CheckFileResolver`.
unsafe fn check_file_resolver_get_human_readable_module_name_thunk(
    _this: *const FileResolver,
    name: &ModuleName,
) -> String {
    name.clone()
}

/// `getEnvironmentForModule` thunk — no per-module environment, so `None`.
///
/// # Safety
/// `this` must point at the `base` subobject of a live `CheckFileResolver`.
unsafe fn check_file_resolver_get_environment_for_module_thunk(
    _this: *const FileResolver,
    _name: &ModuleName,
) -> Option<String> {
    None
}

impl CheckFileResolver {
    fn new(source: &str) -> Self {
        let vtable = FileResolverVtable {
            read_source: check_file_resolver_read_source_thunk,
            resolve_module: check_file_resolver_resolve_module_thunk,
            get_human_readable_module_name: check_file_resolver_get_human_readable_module_name_thunk,
            get_environment_for_module: check_file_resolver_get_environment_for_module_thunk,
        };
        CheckFileResolver {
            base: FileResolver {
                vtable,
                require_suggester: None,
            },
            source: source.to_string(),
        }
    }
}

/// Minimal [`ConfigResolver`] returning a default [`Config`].
///
/// `#[repr(C)]` with `base` first so the `getConfig` thunk can cast the
/// `*const ConfigResolver` receiver back to `*const CheckConfigResolver`.
#[repr(C)]
struct CheckConfigResolver {
    base: ConfigResolver,
    default_config: Config,
}

/// `getConfig` thunk — returns the single default config.
///
/// # Safety
/// `this` must point at the `base` subobject of a live `CheckConfigResolver`.
unsafe fn check_config_resolver_get_config_thunk(
    this: *const ConfigResolver,
    _name: *const ModuleName,
    _limits: *const TypeCheckLimits,
) -> *const Config {
    let this = this as *const CheckConfigResolver;
    // SAFETY: per this fn's contract, `this` points at a live `CheckConfigResolver`.
    unsafe { &(*this).default_config as *const Config }
}

impl CheckConfigResolver {
    fn new() -> Self {
        CheckConfigResolver {
            base: ConfigResolver {
                get_config: Some(check_config_resolver_get_config_thunk),
            },
            default_config: Config::default(),
        }
    }
}

/// The fixed package name under which host definitions are registered. Mirrors
/// `Fixture::loadDefinition`'s `"@test"`; `@`-prefixed names are the convention
/// for synthetic (non-file) modules.
const HOST_DEFINITIONS_PACKAGE: &str = "@host";

/// The fallible body, run under `catch_unwind` so a panic in the type checker
/// surfaces as a diagnostic rather than unwinding into the caller. Returns the
/// collected [`TypeDiagnostic`]s, or an empty `Vec` when the source type-checks
/// clean.
///
/// `definitions`, when present and non-empty, is Luau definition-file syntax
/// (`declare function …`, `declare class …`, `declare x: T`) describing the host
/// surface; it is registered into the global scope *after* the builtins (so it
/// may reference them) and *before* the script is checked.
fn run_check(source: &str, definitions: Option<&str>) -> Vec<TypeDiagnostic> {
    let mut diagnostics = Vec::new();

    let mut file_resolver = CheckFileResolver::new(source);
    let mut config_resolver = CheckConfigResolver::new();
    let options = FrontendOptions::default();

    let mut frontend = Frontend::frontend_file_resolver_config_resolver_frontend_options(
        &mut file_resolver.base,
        &mut config_resolver.base,
        &options,
    );
    unsafe {
        frontend.wire_self_pointers();
    }

    // Use the validated OLD solver path.
    frontend.set_luau_solver_mode(SolverMode::Old);

    // Add Luau builtins:
    //   Luau::unfreeze(frontend.globals.globalTypes);
    //   Luau::registerBuiltinGlobals(frontend, frontend.globals);
    //   Luau::freeze(frontend.globals.globalTypes);
    let frontend_ptr = &mut frontend as *mut Frontend;
    unsafe {
        unfreeze((*frontend_ptr).globals.global_types_mut());
        register_builtin_globals(&mut *frontend_ptr, &mut (*frontend_ptr).globals, false);
        freeze((*frontend_ptr).globals.global_types_mut());
    }

    // Register host type definitions, if any, into the same global scope the
    // builtins live in. A script then type-checks against the host-provided
    // surface (the Rust functions / userdata exposed to the runtime). Pattern
    // mirrors `Fixture::loadDefinition`: unfreeze -> loadDefinitionFile(globals,
    // globalScope, …) -> freeze.
    if let Some(defs) = definitions {
        if !defs.is_empty() {
            unsafe {
                unfreeze((*frontend_ptr).globals.global_types_mut());
                let target_scope = (*frontend_ptr).globals.global_scope();
                let result = (*frontend_ptr).load_definition_file(
                    &mut (*frontend_ptr).globals,
                    target_scope,
                    defs,
                    String::from(HOST_DEFINITIONS_PACKAGE),
                    /* captureComments */ false,
                    /* typeCheckForAutocomplete */ false,
                );
                freeze((*frontend_ptr).globals.global_types_mut());

                // Malformed host definitions are a usage error, surfaced with
                // `in_definitions: true` so they are distinguishable from script
                // diagnostics. A failed load did not persist anything, so
                // checking the script against a half-built surface is pointless —
                // return immediately.
                if !result.success {
                    for err in &result.parse_result.errors {
                        let begin = err.get_location().begin;
                        let end = err.get_location().end;
                        diagnostics.push(TypeDiagnostic {
                            line: begin.line + 1,
                            column: begin.column + 1,
                            end_line: end.line + 1,
                            end_column: end.column + 1,
                            message: err.get_message().to_string(),
                            in_definitions: true,
                        });
                    }
                    if let Some(module) = &result.module {
                        for err in &module.errors {
                            let begin = err.location.begin;
                            let end = err.location.end;
                            diagnostics.push(TypeDiagnostic {
                                line: begin.line + 1,
                                column: begin.column + 1,
                                end_line: end.line + 1,
                                end_column: end.column + 1,
                                message: to_string_type_error(err),
                                in_definitions: true,
                            });
                        }
                    }
                    if diagnostics.is_empty() {
                        diagnostics.push(TypeDiagnostic {
                            line: 1,
                            column: 1,
                            end_line: 1,
                            end_column: 1,
                            message: "failed to load".to_string(),
                            in_definitions: true,
                        });
                    }
                    return diagnostics;
                }
            }
        }
    }

    // Luau::CheckResult checkResult = frontend.check("main");
    let check_result =
        frontend.check_module_name_optional_frontend_options(&MAIN_MODULE.to_string(), None);

    for err in &check_result.errors {
        let begin = err.location.begin;
        let end = err.location.end;
        diagnostics.push(TypeDiagnostic {
            line: begin.line + 1,
            column: begin.column + 1,
            end_line: end.line + 1,
            end_column: end.column + 1,
            message: to_string_type_error(err),
            in_definitions: false,
        });
    }

    diagnostics
}

/// Type-check Luau `source`. Returns `Ok(())` if it type-checks clean, or `Err`
/// of the structured diagnostics on type errors.
///
/// ```
/// # #[cfg(feature = "typecheck")] {
/// luaur_rt::check("local x: number = 1").unwrap();
/// assert!(luaur_rt::check("local x: number = \"oops\"").is_err());
/// # }
/// ```
pub fn check(source: &str) -> core::result::Result<(), Vec<TypeDiagnostic>> {
    check_inner(source, None)
}

/// Type-check Luau `source` with extra host type `definitions` in scope.
///
/// `definitions` is Luau **definition-file syntax** describing the host-provided
/// globals — the Rust functions, values, and userdata you expose to the runtime
/// (e.g. via [`Lua::create_function`](crate::Lua::create_function) /
/// [`UserData`](crate::UserData)):
///
/// ```text
/// declare function add(a: number, b: number): number
/// declare config: { name: string, retries: number }
/// declare class Vec2
///     x: number
///     y: number
///     function magnitude(self): number
/// end
/// ```
///
/// Returns `Ok(())` when the source type-checks clean against the builtins plus
/// the host definitions, or `Err` of the structured diagnostics. Errors in the
/// definitions themselves are reported with `in_definitions == true`.
///
/// ```
/// # #[cfg(feature = "typecheck")] {
/// // The script references a host function the checker would otherwise reject:
/// luaur_rt::check("local n: number = add(1, 2)").unwrap_err();
/// luaur_rt::check_with_definitions(
///     "local n: number = add(1, 2)",
///     "declare function add(a: number, b: number): number",
/// )
/// .unwrap();
/// # }
/// ```
pub fn check_with_definitions(
    source: &str,
    definitions: &str,
) -> core::result::Result<(), Vec<TypeDiagnostic>> {
    check_inner(source, Some(definitions))
}

/// Shared body of [`check`] / [`check_with_definitions`]: run the checker under
/// `catch_unwind` (so a panic in the type checker becomes a diagnostic rather
/// than unwinding into the caller) and fold the diagnostics into a `Result`.
fn check_inner(
    source: &str,
    definitions: Option<&str>,
) -> core::result::Result<(), Vec<TypeDiagnostic>> {
    // A panic inside the checker becomes a single diagnostic.
    let owned = source.to_string();
    let owned_defs = definitions.map(|d| d.to_string());
    let diagnostics =
        match std::panic::catch_unwind(move || run_check(&owned, owned_defs.as_deref())) {
            Ok(diagnostics) => diagnostics,
            Err(payload) => vec![TypeDiagnostic {
                line: 1,
                column: 1,
                end_line: 1,
                end_column: 1,
                message: panic_message(&payload),
                in_definitions: false,
            }],
        };

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

/// Extract a `std::exception::what()`-equivalent message from a caught panic
/// payload.
fn panic_message(payload: &(dyn core::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown error".to_string()
    }
}
