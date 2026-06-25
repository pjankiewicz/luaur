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
    let joined: String = misuse.iter().map(|d| d.message.clone()).collect::<Vec<_>>().join("\n");
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
    c.check().expect("chunk should type-check against host defs");
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
