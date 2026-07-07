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
use luaur_analysis::records::load_definition_file_result::LoadDefinitionFileResult;
use luaur_analysis::records::module_info::ModuleInfo;
use luaur_analysis::records::source_code::SourceCode;
use luaur_analysis::records::type_check_limits::TypeCheckLimits;
use luaur_analysis::records::type_error::TypeError;
use luaur_analysis::type_aliases::module_name_file_resolver::ModuleName;
use luaur_ast::records::ast_expr::AstExpr;
use luaur_ast::records::ast_expr_call::AstExprCall;
use luaur_ast::records::ast_expr_constant_string::AstExprConstantString;
use luaur_ast::records::ast_expr_global::AstExprGlobal;
use luaur_ast::records::ast_expr_index_expr::AstExprIndexExpr;
use luaur_ast::records::ast_expr_index_name::AstExprIndexName;
use luaur_ast::records::ast_node::AstNode;
use luaur_ast::rtti::ast_node_as;
use luaur_config::records::config::Config;

use core::fmt;
use std::collections::HashMap;
use std::ffi::CStr;

/// One type-checker diagnostic with its source location (all 1-based).
///
/// Produced by [`check`] / [`check_with_definitions`] and carried inside
/// [`Error::TypeError`](crate::Error::TypeError). Unlike a flat error string,
/// the location fields let an editor / build tool point at the exact span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDiagnostic {
    /// The module that produced this diagnostic, when the checker had one.
    ///
    /// The simple [`check`] / [`check_with_definitions`] helpers preserve their
    /// historical display shape and leave this empty for the synthetic `"main"`
    /// module. Module-aware checks fill it for imported modules.
    pub module: Option<String>,
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
        if let Some(module) = &self.module {
            write!(f, "{module}:")?;
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
            get_human_readable_module_name:
                check_file_resolver_get_human_readable_module_name_thunk,
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

/// In-memory [`FileResolver`] for module-aware checks.
///
/// Holds a complete module-name -> source map. The resolver supports the common
/// Luau require path shapes used by host-embedded scripts:
///
/// - `require("@alias")`
/// - `require("literal/module/name")`
/// - `require(game.Module)`
/// - `require(script.Parent.Module)`
/// - `require(game:GetService("Service").Module)`
#[repr(C)]
struct CheckModuleFileResolver {
    base: FileResolver,
    sources: HashMap<ModuleName, String>,
}

unsafe fn check_module_file_resolver_read_source_thunk(
    this: *mut FileResolver,
    name: &ModuleName,
) -> Option<SourceCode> {
    let this = this as *const CheckModuleFileResolver;
    let source = unsafe { (*this).sources.get(name)?.clone() };
    Some(SourceCode {
        source,
        r#type: SourceCode::Module,
    })
}

unsafe fn check_module_file_resolver_resolve_module_thunk(
    this: *mut FileResolver,
    context: *const ModuleInfo,
    expr: *mut AstExpr,
    _limits: &TypeCheckLimits,
) -> Option<ModuleInfo> {
    if expr.is_null() {
        return None;
    }

    let this = this as *const CheckModuleFileResolver;
    let resolver = unsafe { &*this };
    let context = if context.is_null() {
        None
    } else {
        Some(unsafe { &*context })
    };

    resolver.resolve_module(context, unsafe { &*expr })
}

unsafe fn check_module_file_resolver_get_human_readable_module_name_thunk(
    _this: *const FileResolver,
    name: &ModuleName,
) -> String {
    name.clone()
}

unsafe fn check_module_file_resolver_get_environment_for_module_thunk(
    _this: *const FileResolver,
    _name: &ModuleName,
) -> Option<String> {
    None
}

impl CheckModuleFileResolver {
    fn new(modules: &[(&str, &str)]) -> Self {
        let vtable = FileResolverVtable {
            read_source: check_module_file_resolver_read_source_thunk,
            resolve_module: check_module_file_resolver_resolve_module_thunk,
            get_human_readable_module_name:
                check_module_file_resolver_get_human_readable_module_name_thunk,
            get_environment_for_module: check_module_file_resolver_get_environment_for_module_thunk,
        };
        let sources = modules
            .iter()
            .map(|(name, source)| ((*name).to_string(), (*source).to_string()))
            .collect();

        CheckModuleFileResolver {
            base: FileResolver {
                vtable,
                require_suggester: None,
            },
            sources,
        }
    }

    fn resolve_module(&self, context: Option<&ModuleInfo>, expr: &AstExpr) -> Option<ModuleInfo> {
        let node = expr as *const AstExpr as *mut AstNode;

        unsafe {
            if let Some(string) = ast_node_as::<AstExprConstantString>(node).as_ref() {
                return self.module_info_if_present(constant_string_to_string(string));
            }

            if let Some(global) = ast_node_as::<AstExprGlobal>(node).as_ref() {
                let name = ast_name_to_string(global.name.value);

                if name == "script" {
                    return context.cloned();
                }

                return self.module_info_if_known_prefix(name);
            }

            if let Some(index) = ast_node_as::<AstExprIndexName>(node).as_ref() {
                let context = context?;
                let index_name = ast_name_to_string(index.index.value);

                if index_name == "Parent" {
                    let last_separator = context.name.rfind('/')?;
                    return Some(ModuleInfo {
                        name: context.name[..last_separator].to_string(),
                        optional: context.optional,
                    });
                }

                return self
                    .module_info_if_known_prefix(format!("{}/{}", context.name, index_name));
            }

            if let Some(index) = ast_node_as::<AstExprIndexExpr>(node).as_ref() {
                let context = context?;
                let index_expr = index.index as *mut AstNode;

                if let Some(index_string) =
                    ast_node_as::<AstExprConstantString>(index_expr).as_ref()
                {
                    return self.module_info_if_known_prefix(format!(
                        "{}/{}",
                        context.name,
                        constant_string_to_string(index_string)
                    ));
                }
            }

            if let Some(call) = ast_node_as::<AstExprCall>(node).as_ref() {
                let context = context?;

                if call.self_ && call.args.size >= 1 && context.name == "game" {
                    let arg = *call.args.data;
                    let arg_node = arg as *mut AstNode;
                    let func_node = call.func as *mut AstNode;

                    if let (Some(index_string), Some(func)) = (
                        ast_node_as::<AstExprConstantString>(arg_node).as_ref(),
                        ast_node_as::<AstExprIndexName>(func_node).as_ref(),
                    ) {
                        if ast_name_to_string(func.index.value) == "GetService" {
                            return self.module_info_if_known_prefix(format!(
                                "game/{}",
                                constant_string_to_string(index_string)
                            ));
                        }
                    }
                }
            }
        }

        None
    }

    fn module_info_if_present(&self, name: String) -> Option<ModuleInfo> {
        if self.sources.contains_key(&name) {
            Some(ModuleInfo {
                name,
                optional: false,
            })
        } else {
            None
        }
    }

    fn module_info_if_known_prefix(&self, name: String) -> Option<ModuleInfo> {
        let child_prefix = format!("{name}/");
        if self.sources.contains_key(&name)
            || self
                .sources
                .keys()
                .any(|module| module.starts_with(&child_prefix))
        {
            Some(ModuleInfo {
                name,
                optional: false,
            })
        } else {
            None
        }
    }
}

fn ast_name_to_string(name: *const core::ffi::c_char) -> String {
    if name.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(name).to_string_lossy().into_owned() }
    }
}

fn constant_string_to_string(expr: &AstExprConstantString) -> String {
    expr.value
        .as_slice()
        .iter()
        .map(|&c| c as u8 as char)
        .collect()
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
                            module: None,
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
                                module: None,
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
                            module: None,
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
            module: None,
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

fn push_definition_diagnostics(
    diagnostics: &mut Vec<TypeDiagnostic>,
    result: &LoadDefinitionFileResult,
) {
    for err in &result.parse_result.errors {
        let begin = err.get_location().begin;
        let end = err.get_location().end;
        diagnostics.push(TypeDiagnostic {
            module: None,
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
                module: None,
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
            module: None,
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 1,
            message: "failed to load".to_string(),
            in_definitions: true,
        });
    }
}

fn push_type_error_diagnostic(
    diagnostics: &mut Vec<TypeDiagnostic>,
    err: &TypeError,
    fallback_module: Option<&str>,
) {
    let begin = err.location.begin;
    let end = err.location.end;
    let module = if err.module_name.is_empty() {
        fallback_module.map(str::to_string)
    } else {
        Some(err.module_name.clone())
    };
    diagnostics.push(TypeDiagnostic {
        module,
        line: begin.line + 1,
        column: begin.column + 1,
        end_line: end.line + 1,
        end_column: end.column + 1,
        message: to_string_type_error(err),
        in_definitions: false,
    });
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

/// Type-check a graph of Luau modules.
///
/// `root_module` is the module name to check first. `modules` must contain a
/// `(module_name, source)` pair for `root_module`; any modules reachable through
/// supported `require(...)` paths are checked and their exported type surfaces
/// are used for the requiring script.
///
/// Supported require paths are the common embedded-script forms:
///
/// ```text
/// require("@alias")
/// require("literal/module/name")
/// require(game.Module)
/// require(script.Parent.Module)
/// require(game:GetService("Service").Module)
/// ```
pub fn check_modules(
    root_module: &str,
    modules: &[(&str, &str)],
) -> core::result::Result<(), Vec<TypeDiagnostic>> {
    check_modules_inner(root_module, modules, None)
}

/// Type-check a graph of Luau modules with extra host type `definitions` in
/// scope. See [`check_modules`] and [`check_with_definitions`].
pub fn check_modules_with_definitions(
    root_module: &str,
    modules: &[(&str, &str)],
    definitions: &str,
) -> core::result::Result<(), Vec<TypeDiagnostic>> {
    check_modules_inner(root_module, modules, Some(definitions))
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
                module: None,
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

fn check_modules_inner(
    root_module: &str,
    modules: &[(&str, &str)],
    definitions: Option<&str>,
) -> core::result::Result<(), Vec<TypeDiagnostic>> {
    let owned_root = root_module.to_string();
    let owned_modules: Vec<(String, String)> = modules
        .iter()
        .map(|(name, source)| ((*name).to_string(), (*source).to_string()))
        .collect();
    let owned_defs = definitions.map(|d| d.to_string());

    let diagnostics = match std::panic::catch_unwind(move || {
        let borrowed: Vec<(&str, &str)> = owned_modules
            .iter()
            .map(|(name, source)| (name.as_str(), source.as_str()))
            .collect();
        run_check_modules(&owned_root, &borrowed, owned_defs.as_deref())
    }) {
        Ok(diagnostics) => diagnostics,
        Err(payload) => vec![TypeDiagnostic {
            module: Some(root_module.to_string()),
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

fn run_check_modules(
    root_module: &str,
    modules: &[(&str, &str)],
    definitions: Option<&str>,
) -> Vec<TypeDiagnostic> {
    let mut diagnostics = Vec::new();

    if !modules.iter().any(|(name, _)| *name == root_module) {
        diagnostics.push(TypeDiagnostic {
            module: Some(root_module.to_string()),
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 1,
            message: format!("root module '{root_module}' is not present in module sources"),
            in_definitions: false,
        });
        return diagnostics;
    }

    let mut file_resolver = CheckModuleFileResolver::new(modules);
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
    frontend.set_luau_solver_mode(SolverMode::Old);

    let frontend_ptr = &mut frontend as *mut Frontend;
    unsafe {
        unfreeze((*frontend_ptr).globals.global_types_mut());
        register_builtin_globals(&mut *frontend_ptr, &mut (*frontend_ptr).globals, false);
        freeze((*frontend_ptr).globals.global_types_mut());
    }

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

                if !result.success {
                    push_definition_diagnostics(&mut diagnostics, &result);
                    return diagnostics;
                }
            }
        }
    }

    let check_result =
        frontend.check_module_name_optional_frontend_options(&root_module.to_string(), None);

    for err in &check_result.errors {
        push_type_error_diagnostic(&mut diagnostics, err, Some(root_module));
    }

    diagnostics
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

/// A reusable type checker: registers the Luau builtins **once** and checks many
/// sources against the shared global environment.
///
/// A one-shot [`check`] rebuilds the whole frontend and re-parses + re-checks the
/// entire `@luau` builtin definition file on every call — that setup dominates the
/// cost. `Checker` pays it once at construction, so each [`Checker::check`] only
/// parses and checks the input itself (orders of magnitude faster across many
/// snippets — fuzzers, language servers, batch linters).
///
/// Host definitions are not supported here: they mutate the global environment and
/// so can't be cached this way — use [`check_with_definitions`] for those.
pub struct Checker {
    // Boxed so their addresses are stable for the checker's whole lifetime:
    // `Frontend` stores raw `*mut` pointers into the resolvers (the `repr(C)`
    // vtable thunks cast back via the address) and `wire_self_pointers` makes the
    // frontend self-referential. Moving the `Checker` moves only the `Box`
    // pointers; the pointees stay put on the heap, so every stored pointer stays
    // valid. `config_resolver` is kept alive solely because the frontend points
    // into it.
    file_resolver: Box<CheckFileResolver>,
    #[allow(dead_code)]
    config_resolver: Box<CheckConfigResolver>,
    frontend: Box<Frontend>,
}

impl Checker {
    /// Build a checker with the Luau builtins registered (the expensive,
    /// once-only step).
    pub fn new() -> Self {
        let mut file_resolver = Box::new(CheckFileResolver::new(""));
        let mut config_resolver = Box::new(CheckConfigResolver::new());
        let options = FrontendOptions::default();
        let mut frontend = Box::new(
            Frontend::frontend_file_resolver_config_resolver_frontend_options(
                &mut file_resolver.base,
                &mut config_resolver.base,
                &options,
            ),
        );
        // Wire AFTER boxing so the self-pointers target the stable heap address.
        unsafe {
            frontend.wire_self_pointers();
        }
        frontend.set_luau_solver_mode(SolverMode::Old);

        let frontend_ptr: *mut Frontend = &mut *frontend;
        // SAFETY: `frontend_ptr` is the live boxed frontend. Register the Luau
        // builtins into the global scope once (the cost we're amortizing).
        unsafe {
            unfreeze((*frontend_ptr).globals.global_types_mut());
            register_builtin_globals(&mut *frontend_ptr, &mut (*frontend_ptr).globals, false);
            freeze((*frontend_ptr).globals.global_types_mut());
        }

        Checker {
            file_resolver,
            config_resolver,
            frontend,
        }
    }

    /// Type-check `source` against the cached global environment. `Ok(())` when it
    /// checks clean, otherwise the diagnostics. Never panics on malformed input
    /// (the checker returns errors).
    pub fn check(&mut self, source: &str) -> core::result::Result<(), Vec<TypeDiagnostic>> {
        // Point the single "main" module at the new source and force a re-check.
        self.file_resolver.source = source.to_string();
        self.frontend.mark_dirty(&MAIN_MODULE.to_string(), None);

        let check_result = self
            .frontend
            .check_module_name_optional_frontend_options(&MAIN_MODULE.to_string(), None);

        let mut diagnostics = Vec::new();
        for err in &check_result.errors {
            let begin = err.location.begin;
            let end = err.location.end;
            diagnostics.push(TypeDiagnostic {
                module: None,
                line: begin.line + 1,
                column: begin.column + 1,
                end_line: end.line + 1,
                end_column: end.column + 1,
                message: to_string_type_error(err),
                in_definitions: false,
            });
        }
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}
