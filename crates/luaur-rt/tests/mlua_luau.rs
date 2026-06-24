// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// Only the Luau-relevant, non-deferred subset of mlua's `tests/luau.rs` is
// ported. Dropped (deferred / out-of-scope luaur-rt features):
//   - Vector / vector library tests, Compiler tests, Buffer tests
//   - sandbox / sandbox_safeenv / sandbox_nolibs / sandbox_threads
//   - interrupts, fflags (Lua::set_fflag), memory categories, heap dumps
//   - integer64 type, typeof(error-value) (luaur-rt carries `Value::Error` as a
//     string, so `typeof` would report "string"; the error-tagged value path is
//     deferred).

use luaur_rt::{Error, Lua, Result, Table, Value};

#[test]
fn test_version() -> Result<()> {
    let lua = Lua::new();
    assert!(lua.globals().get::<String>("_VERSION")?.starts_with("Luau"));
    Ok(())
}

#[test]
fn test_load_from_rust() -> Result<()> {
    // DEVIATION: mlua's `test_loadstring` exercises the Lua-level `loadstring`
    // builtin, which luaur's base library does not register. The luaur-rt
    // analog is `Lua::load(...).into_function()`, which compiles a string into a
    // callable function — exercised here with the same observable result.
    let lua = Lua::new();

    let f = lua.load("return 123").into_function()?;
    assert_eq!(f.call::<i32>(())?, 123);

    Ok(())
}

#[test]
fn test_readonly_table() -> Result<()> {
    let lua = Lua::new();

    let t = lua.create_sequence_from([1])?;
    assert!(!t.is_readonly());
    t.set_readonly(true);
    assert!(t.is_readonly());

    fn check_readonly_error<T: std::fmt::Debug>(res: Result<T>) {
        match res {
            Err(Error::RuntimeError(e)) if e.contains("attempt to modify a readonly table") => {}
            r => panic!("expected readonly RuntimeError, got {r:?}"),
        }
    }

    check_readonly_error(t.set("key", "value"));
    check_readonly_error(t.raw_set("key", "value"));
    check_readonly_error(t.raw_insert(1, "value"));
    check_readonly_error(t.raw_remove(1));
    check_readonly_error(t.push("value"));
    check_readonly_error(t.pop::<Value>());
    check_readonly_error(t.raw_push("value"));
    check_readonly_error(t.raw_pop::<Value>());

    // Special case: cannot change the metatable of a readonly table.
    check_readonly_error(t.set_metatable(None));

    // Flipping back to writable restores mutation.
    t.set_readonly(false);
    t.set("key", "value")?;
    assert_eq!(t.get::<String>("key")?, "value");

    Ok(())
}

#[test]
fn test_readonly_table_reads_still_work() -> Result<()> {
    let lua = Lua::new();

    let t = lua.create_sequence_from([10, 20, 30])?;
    t.set_readonly(true);
    // Reads must remain available on a readonly table.
    assert_eq!(t.get::<i64>(1)?, 10);
    assert_eq!(t.raw_get::<i64>(2)?, 20);
    assert_eq!(t.raw_len(), 3);
    assert_eq!(t.sequence_values::<i64>().collect::<Result<Vec<_>>>()?, vec![10, 20, 30]);

    Ok(())
}

#[test]
fn test_metatable_via_lua() -> Result<()> {
    // A small Luau metatable round-trip (set_metatable + __index function),
    // mirroring the spirit of mlua's vector-metatable test without vectors.
    let lua = Lua::new();

    let base = lua
        .load(
            r#"
            return {
                __index = {
                    greet = function() return "hi" end,
                }
            }
        "#,
        )
        .eval::<Table>()?;

    let t = lua.create_table();
    t.set_metatable(Some(base))?;
    let greeting: String = lua
        .load("local t = ...; return t.greet()")
        .into_function()?
        .call(t)?;
    assert_eq!(greeting, "hi");

    Ok(())
}
