// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// Dropped (deferred luaur-rt features): test_function_environment (Function
// environment get/set), test_function_info (debug Function::info), and the
// Luau-only test_function_coverage / test_function_deep_clone (compiler
// coverage + deep clone), test_function_dump (non-luau bytecode dump), and the
// Function::wrap / wrap_raw family (no `wrap` constructor — luaur-rt builds
// callbacks via `Lua::create_function`).

use luaur_rt::{Error, Function, Lua, Result, Variadic};

#[test]
fn test_function_call() -> Result<()> {
    let lua = Lua::new();

    let concat = lua
        .load(r#"return function(arg1, arg2) return arg1 .. arg2 end"#)
        .eval::<Function>()?;
    assert_eq!(concat.call::<String>(("foo", "bar"))?, "foobar");

    Ok(())
}

#[test]
fn test_function_call_error() -> Result<()> {
    let lua = Lua::new();

    let concat_err = lua
        .load(r#"return function(arg1, arg2) error("concat error") end"#)
        .eval::<Function>()?;
    match concat_err.call::<String>(("foo", "bar")) {
        Err(Error::RuntimeError(msg)) if msg.contains("concat error") => {}
        other => panic!("unexpected result: {other:?}"),
    }

    Ok(())
}

#[test]
fn test_function_bind() -> Result<()> {
    let lua = Lua::new();

    let globals = lua.globals();
    lua.load(
        r#"
        function concat(...)
            local res = ""
            for _, s in pairs({...}) do
                res = res..s
            end
            return res
        end
    "#,
    )
    .exec()?;

    let mut concat = globals.get::<Function>("concat")?;
    concat = concat.bind("foo")?;
    concat = concat.bind("bar")?;
    concat = concat.bind(("baz", "baf"))?;
    assert_eq!(concat.call::<String>(())?, "foobarbazbaf");
    assert_eq!(concat.call::<String>(("hi", "wut"))?, "foobarbazbafhiwut");

    let mut concat2 = globals.get::<Function>("concat")?;
    concat2 = concat2.bind(())?;
    assert_eq!(concat2.call::<String>(())?, "");
    assert_eq!(concat2.call::<String>(("ab", "cd"))?, "abcd");

    Ok(())
}

#[test]
fn test_function_bind_error() -> Result<()> {
    let lua = Lua::new();

    // A function that ignores all of its arguments.
    let func = lua.load(r#"return function(...) end"#).eval::<Function>()?;
    // Calling with an enormous variadic should overflow the Lua stack.
    assert!(func.call::<()>(Variadic::from_iter(1..1000000)).is_err());

    Ok(())
}

#[test]
fn test_function_pointer() -> Result<()> {
    let lua = Lua::new();

    let func1 = lua.load("return function() end").into_function()?;
    let func2 = func1.call::<Function>(())?;

    assert_eq!(func1.to_pointer(), func1.clone().to_pointer());
    assert_ne!(func1.to_pointer(), func2.to_pointer());

    Ok(())
}

#[test]
fn test_create_function_basic() -> Result<()> {
    // Adapted to exercise the luaur-rt callback path (mlua's `Function::wrap`
    // counterpart): a Rust closure exposed to Lua, including error return.
    let lua = Lua::new();

    let f = lua.create_function(|_, (s, n): (String, usize)| Ok(s.repeat(n)))?;
    lua.globals().set("f", f)?;
    lua.load(r#"assert(f("hello", 2) == "hellohello")"#).exec()?;

    let ferr =
        lua.create_function(|_, ()| -> Result<()> { Err(Error::runtime("some error")) })?;
    lua.globals().set("ferr", ferr)?;
    lua.load(
        r#"
        local ok, err = pcall(ferr)
        assert(not ok and tostring(err):find("some error"))
    "#,
    )
    .exec()?;

    Ok(())
}
