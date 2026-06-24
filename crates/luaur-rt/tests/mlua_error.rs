// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// Dropped (deferred luaur-rt features):
//   - test_error_context / test_error_chain: depend on mlua's `ErrorContext`
//     (`.context`/`.with_context`), the structured `Error::CallbackError`/
//     `WithContext` chain, and `Error::chain()`/`parent()`. luaur-rt carries a
//     flat error model (no nested callback/context chain), so these are out of
//     scope. `Error::external` + `downcast_ref` (the portable core) is kept.
//   - test_error_anyhow: requires the `anyhow` feature.

use std::io;

use luaur_rt::{Error, Lua, Result};

#[test]
fn test_external_error() {
    // `Error::external` should preserve a luaur-rt `Error`
    let runtime_err = Error::runtime("test error");
    let converted = Error::external(runtime_err);
    assert!(matches!(converted, Error::RuntimeError(ref msg) if msg == "test error"));

    // Other errors should become `ExternalError`
    let converted = Error::external(io::Error::other("other error"));
    assert!(matches!(converted, Error::ExternalError(_)));
    assert!(converted.downcast_ref::<io::Error>().is_some());
}

#[test]
fn test_rust_error_surfaces_to_lua() -> Result<()> {
    // A Rust callback returning `Err` becomes a catchable Lua error whose
    // message is preserved (the portable slice of mlua's error semantics).
    let lua = Lua::new();

    let func = lua.create_function(|_, ()| -> Result<()> {
        Err(Error::runtime("runtime error"))
    })?;
    lua.globals().set("func", func)?;

    let msg = lua
        .load("local _, err = pcall(func); return tostring(err)")
        .eval::<String>()?;
    assert!(msg.contains("runtime error"), "got: {msg}");

    Ok(())
}

#[test]
fn test_external_io_error_message_preserved() -> Result<()> {
    let lua = Lua::new();

    let func = lua.create_function(|_, ()| -> Result<()> {
        Err(Error::external(io::Error::other("disk on fire")))
    })?;
    lua.globals().set("func", func)?;

    let msg = lua
        .load("local _, err = pcall(func); return tostring(err)")
        .eval::<String>()?;
    assert!(msg.contains("disk on fire"), "got: {msg}");

    Ok(())
}

#[test]
fn test_error_display() {
    // Display formats are part of the observable contract.
    assert_eq!(
        Error::runtime("boom").to_string(),
        "runtime error: boom"
    );
    let conv = Error::FromLuaConversionError {
        from: "nil",
        to: "String".to_string(),
        message: None,
    };
    assert_eq!(conv.to_string(), "error converting Lua nil to String");
}
