// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// Dropped (deferred / out-of-scope luaur-rt features):
//   - test_chunk_path  (loading a chunk from a filesystem Path)
//   - test_chunk_macro (the `mlua::chunk!` proc-macro)
//   - test_compiler / test_compiler_library_constants (mlua::Compiler)
//   - test_chunk_wrap  (Chunk::wrap detached constructor)
//   - chunk environment + ChunkMode (set_environment / mode): deferred.
// luaur-rt's `Lua::load` accepts `impl AsRef<str>` (text source), so the
// bytes-source arms of `test_chunk_impls` are expressed as `&str`/`String`.

use luaur_rt::{Lua, Result};

#[test]
fn test_chunk_methods() -> Result<()> {
    let lua = Lua::new();

    // Default name, then override via set_name.
    let default_chunk = lua.load("return 123");
    assert_eq!(default_chunk.name(), "chunk");

    let chunk2 = lua.load("return 123").set_name("@new_name");
    assert_eq!(chunk2.name(), "@new_name");
    assert_eq!(chunk2.eval::<i32>()?, 123);

    Ok(())
}

#[test]
fn test_chunk_eval_exec() -> Result<()> {
    let lua = Lua::new();

    // eval returns the value
    assert_eq!(lua.load("return 1 + 1").eval::<i32>()?, 2);
    // exec runs for side effects
    lua.load("result = 5 * 5").exec()?;
    assert_eq!(lua.globals().get::<i32>("result")?, 25);
    // into_function compiles to a reusable function
    let f = lua.load("return ...").into_function()?;
    assert_eq!(f.call::<i32>(42)?, 42);

    Ok(())
}

#[test]
fn test_chunk_impls() -> Result<()> {
    let lua = Lua::new();

    // StdString
    assert_eq!(lua.load(String::from("return 1")).eval::<i32>()?, 1);
    assert_eq!(lua.load(&String::from("return 2")).eval::<i32>()?, 2);

    // &str
    assert_eq!(lua.load("return 3").eval::<i32>()?, 3);

    Ok(())
}

#[test]
fn test_chunk_syntax_error() -> Result<()> {
    let lua = Lua::new();

    let err = lua.load("this is not + valid lua %").exec().unwrap_err();
    assert!(
        matches!(err, luaur_rt::Error::SyntaxError { .. }),
        "expected SyntaxError, got {err:?}"
    );

    Ok(())
}

#[test]
fn test_chunk_runtime_error() -> Result<()> {
    let lua = Lua::new();

    let err = lua.load(r#"error("boom")"#).exec().unwrap_err();
    assert!(err.to_string().contains("boom"), "got: {err}");

    Ok(())
}
