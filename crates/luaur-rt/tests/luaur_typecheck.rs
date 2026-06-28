//! Tests for luaur-rt's static type-checking surface (the `typecheck` feature).
//!
//! These are luaur-SPECIFIC — there is no mlua equivalent (Lua has no static
//! types), so they deliberately live OUT of the `mlua_*.rs` ports to keep the
//! "verbatim mlua" suite clean.
#![cfg(feature = "typecheck")]

use luaur_rt::{check, check_with_definitions, Error, Lua, TypeDiagnostic};

// ---------------------------------------------------------------------------
// Free functions returning structured diagnostics.
// ---------------------------------------------------------------------------

#[test]
fn free_check_accepts_well_typed() {
    check("local x: number = 1\nreturn x").expect("well-typed source should check clean");
}

#[test]
fn free_check_rejects_mismatch_with_populated_line() {
    // The mismatch is on line 2 (line 1 is the strict mode pragma).
    let diagnostics: Vec<TypeDiagnostic> =
        check("--!strict\nlocal x: number = \"oops\"").expect_err("type mismatch should fail");
    assert!(!diagnostics.is_empty(), "expected at least one diagnostic");
    // Headline requirement: the structured diagnostic carries the right line.
    assert_eq!(
        diagnostics[0].line, 2,
        "diagnostic should point at line 2: {diagnostics:?}"
    );
    assert!(
        diagnostics[0].column >= 1,
        "column should be 1-based and populated: {diagnostics:?}"
    );
    assert!(
        !diagnostics[0].in_definitions,
        "a script error is not an in-definitions error"
    );
    let msg = &diagnostics[0].message;
    assert!(
        msg.contains("number") && msg.contains("string"),
        "message should mention the number/string mismatch: {msg}"
    );
}

#[test]
fn with_definitions_introduces_and_checks_host_fn() {
    // Without the declaration, `add` is unknown under --!strict.
    let bare = check("--!strict\nlocal n: number = add(1, 2)\nreturn n");
    assert!(bare.is_err(), "undeclared host fn should not type-check");

    // Declaring it makes the same script check clean.
    check_with_definitions(
        "--!strict\nlocal n: number = add(1, 2)\nreturn n",
        "declare function add(a: number, b: number): number",
    )
    .expect("declared host fn should type-check");

    // Misusing it (number result assigned to string) is still rejected.
    let misuse = check_with_definitions(
        "--!strict\nlocal s: string = add(1, 2)\nreturn s",
        "declare function add(a: number, b: number): number",
    )
    .expect_err("number result assigned to string should fail");
    let joined: String = misuse
        .iter()
        .map(|d| d.message.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        joined.contains("number") && joined.contains("string"),
        "diagnostic should mention number/string: {joined}"
    );
}

#[test]
fn malformed_definitions_flagged_in_definitions() {
    let diagnostics = check_with_definitions(
        "return 1",
        "declare function add(a: number, b: number: number", // missing ')'
    )
    .expect_err("malformed host definitions should fail");
    assert!(
        diagnostics.iter().any(|d| d.in_definitions),
        "malformed-definition diagnostic should carry in_definitions == true: {diagnostics:?}"
    );
}

// ---------------------------------------------------------------------------
// Runtime-object integration.
// ---------------------------------------------------------------------------

#[test]
fn lua_check_clean_and_mismatch() {
    let lua = Lua::new();
    lua.check("local x: number = 1\nreturn x")
        .expect("clean source should check");

    let err = lua
        .check("--!strict\nlocal x: number = \"oops\"")
        .expect_err("mismatch should fail");
    match err {
        Error::TypeError(v) => {
            assert!(!v.is_empty(), "expected a diagnostic");
            assert_eq!(v[0].line, 2, "diagnostic should point at line 2: {v:?}");
        }
        other => panic!("expected Error::TypeError, got {other:?}"),
    }
}

#[test]
fn add_definitions_persists_for_lua_check() {
    let lua = Lua::new();
    lua.add_definitions("declare function add(a: number, b: number): number")
        .expect("valid definitions should register");

    // The host fn is now visible to a later check.
    lua.check("--!strict\nlocal n: number = add(1, 2)\nreturn n")
        .expect("declared host fn should type-check after add_definitions");
}

#[test]
fn add_definitions_persists_for_chunk_check() {
    let lua = Lua::new();
    lua.add_definitions("declare function greet(name: string): string")
        .expect("valid definitions should register");

    // Chunk::check sees the accumulated definitions.
    let c = lua.load("--!strict\nlocal s: string = greet(\"world\")\nreturn s");
    c.check()
        .expect("chunk should type-check against host defs");
}

#[test]
fn add_definitions_rejects_malformed() {
    let lua = Lua::new();
    let err = lua
        .add_definitions("declare function add(a: number, b: number: number") // missing ')'
        .expect_err("malformed definitions should be rejected");
    match err {
        Error::TypeError(v) => {
            assert!(
                v.iter().any(|d| d.in_definitions),
                "malformed-definition error should be in_definitions: {v:?}"
            );
        }
        other => panic!("expected Error::TypeError, got {other:?}"),
    }
}

#[test]
fn lua_check_with_definitions_one_off_does_not_persist() {
    let lua = Lua::new();
    // One-off defs make this check pass...
    lua.check_with_definitions(
        "--!strict\nlocal n: number = add(1, 2)\nreturn n",
        "declare function add(a: number, b: number): number",
    )
    .expect("one-off defs should be in scope for this check");

    // ...but they did not persist: a plain check no longer knows `add`.
    let err = lua.check("--!strict\nlocal n: number = add(1, 2)\nreturn n");
    assert!(
        err.is_err(),
        "one-off definitions must not persist on the Lua"
    );
}

// ---------------------------------------------------------------------------
// Headline flow: static check is advisory; the chunk still runs.
// ---------------------------------------------------------------------------

#[test]
fn check_then_run_static_check_is_advisory() {
    let lua = Lua::new();
    // Ill-typed under --!strict: a number assigned to a string-typed local.
    // But Luau is dynamically typed, so it still evaluates to a value.
    let ill_typed = "--!strict\nlocal s: string = 42\nreturn s";

    // (1) The static check rejects it.
    let c = lua.load(ill_typed);
    assert!(
        matches!(c.check(), Err(Error::TypeError(_))),
        "ill-typed chunk should fail Chunk::check"
    );

    // (2) Running it anyway still produces the runtime value (advisory check).
    let value: i64 = lua
        .load(ill_typed)
        .eval()
        .expect("dynamically-typed Luau should still run the ill-typed chunk");
    assert_eq!(value, 42, "the chunk should evaluate to 42 at runtime");
}

// ---------------------------------------------------------------------------
// Determinism: `check` must be a pure function of its input.
// ---------------------------------------------------------------------------

/// Regression test for nondeterministic type inference.
///
/// `filterMap` (and the refinement-map `merge` / `refineLValue` paths) used to
/// dedupe a refined union's options through a `std::collections::HashSet<TypeId>`,
/// whose iteration order is a per-instance random seed over raw `*const Type`
/// pointers. That made the resulting union's *option order* — and therefore the
/// "first incompatible option" reported by union-vs-type unification — vary from
/// one `check()` call to the next, even within a single process. The C++ original
/// uses `std::set<TypeId>` (pointer-ordered: at least stable within a run); the
/// faithful fix dedupes in insertion order, which is deterministic regardless of
/// addresses.
///
/// This program builds a multi-option union via the `or` truthy-filter
/// (`v or 0`, with `v: string | number | boolean`) and then forces a
/// union-vs-`number` unification (`... + 1`), which reports the first failing
/// option. Before the fix this yielded two distinct diagnostic strings across
/// runs; it must now be identical every time.
#[test]
fn check_is_deterministic_for_refined_unions() {
    let src = "local function f(v: string | number | boolean)\n  return (v or 0) + 1\nend\n";

    let baseline = format!("{:?}", check(src));
    // It must actually exercise the diagnostic path we care about.
    assert!(
        check(src).is_err(),
        "the union-vs-number mismatch should produce a diagnostic"
    );

    // The old HashSet-order bug flipped the reported option on ~1 in 2 calls, so
    // even a few dozen repetitions catch a regression with overwhelming
    // probability — P(64 buggy draws all matching the baseline by luck) is ~2^-63.
    // Kept modest on purpose: each `check()` rebuilds a Frontend and registers all
    // builtins, so a few hundred calls would trip CI's anti-OOM slow-timeout.
    for i in 0..64 {
        let again = format!("{:?}", check(src));
        assert_eq!(
            baseline, again,
            "check() must be deterministic, but result diverged on iteration {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// `check` must not abort on self-referential / forward-referenced aliases.
// ---------------------------------------------------------------------------

/// Regression test for a `getMutable` "must follow first" assertion abort.
///
/// `TypeChecker::check(AstStatTypeAlias)` called `getMutable::<TableType>(ty)` /
/// `getMutable::<MetatableType>(ty)` on the raw `resolve_type` result without
/// `follow`-ing it. For a forward-referenced alias chain (here `T = Pt`, `Pt`
/// referring back to `T`, plus a third alias `Pair = T`), that result is a
/// `BoundType` — and `getMutable` asserts its argument is not bound. With Luau
/// assertions armed (debug builds / the fuzz profile) this aborted with SIGTRAP;
/// in release the assert is compiled out, silently returning null. The sibling
/// `check(AstStatLocal)` already followed; the fix makes the alias path match.
///
/// Found by the `typeck_typed` fuzz target. Under `cfg(test)` (debug-assertions
/// on) a regression would abort the process — so simply running this to
/// completion is the assertion.
#[test]
fn check_does_not_abort_on_self_referential_alias() {
    // Forward-referenced + mutually recursive aliases whose resolution yields a
    // BoundType at the `getMutable` site.
    let src = "type T = Pt\ntype Pt = string | { f: T } & boolean\ntype Pair = T\n";
    // We don't care whether it's Ok or Err (it's a recursive-type error) — only
    // that the checker returns instead of aborting on the bound type.
    let _ = check(src);

    // A couple of related shapes that also reach the alias `getMutable` path.
    let _ = check("type A = A\n");
    let _ = check("type X = Y\ntype Y = { next: X }?\ntype Z = X\n");
}

/// Sibling of the above for the **TypePackId** path. `PromoteTypeLevels`'s
/// `visit_type_pack_id_free_type_pack` would call the asserting `getMutable`
/// (via `is::<FreeTypePack>`) on a pack the txn log has bound but not yet
/// committed, tripping `BoundTypePack::get_if(...).is_none()` at
/// `get_mutable_type_pack.rs:16` (SIGTRAP with assertions armed). The fix mirrors
/// the TypeId path: short-circuit on `is::<BoundTypePack>` first.
///
/// Found by the `typeck_typed` fuzz target; this source is the decoded reproducer
/// (recursive aliases unified through a function type yield the bound free pack).
/// C++ luau-analyze runs it cleanly (only ordinary type errors) — port-specific.
/// Running to completion under `cfg(test)` (assertions on) IS the assertion.
#[test]
fn check_does_not_abort_on_bound_free_type_pack_promotion() {
    let src = "type T = V | T\n\
               type U = {{{boolean}}}\n\
               type V = V | number\n\
               local h: boolean\n\
               c = -129\n\
               local h = 550\n\
               local h: T\n\
               type Pair = V\n\
               local x: {string & unknown & unknown & number & unknown & V & number & unknown} & T & T & T & {T} & T & T & T & T = ({x = h:h(h:h(h:e(true), 366.49), -function(c) do\n\
               e = e\n\
               e = e\n\
               end\n\
               do\n\
               e = e\n\
               end\n\
                end), e = 0 + 0})\n\
               type T<T> = number\n";
    let _ = check(src);
}

/// The remaining `PromoteTypeLevels` TypeId arms — `visit_type_id_table_type` and
/// `visit_type_id_function_type` — had the same gap as `visit_type_id_free_type`
/// (fixed earlier): `is::<TableType>` / `is::<FunctionType>` go through the
/// asserting `getMutable`, tripping `BoundType::get_if(...).is_none()` at
/// `get_mutable_type.rs:16` (SIGTRAP) on a type the txn log bound but didn't
/// commit. Fix: short-circuit on `is::<BoundType>` first, like the FreeType arm.
///
/// Found by the `spans` fuzz target; this is the decoded reproducer (an
/// intersection-typed function parameter promoted through a table/function type).
/// C++ luau-analyze runs it cleanly (only ordinary type errors) — port-specific.
#[test]
fn check_does_not_abort_on_bound_table_or_function_promotion() {
    let src = "type T = any\n\
               type U = never\n\
               type V = boolean?\n\
               local function b(z: BoxT, z: T & T): V\n\
               if z:z(z:g(z:c(), ((rawequal(true)))), z) then\n\
               local c = {f = {}, d = h, [true] = z:z(z:z(#{d = false, false} .. true, z:z(y(z:z(z:z((((((((c[#z:z(nil, nil)]))))))), select()), ((z:h(true, true)))), nil), ((((((((((((((x.x)))))))))))))))), ((((0)))))}\n\
               return 0\n\
               else\n\
               return 0\n\
               end\n\
               local a = 0\n\
               return 0\n\
               end\n\
               type T<T> = number\n";
    let _ = check(src);
}
