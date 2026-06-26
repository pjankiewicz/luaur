// Runtime coverage for the error-message *formatting* paths — `pusherror`,
// `lua_g_runerror`, and the coroutine-resume error path — which prepend a
// `"<chunk>:<line>: "` position to a runtime error.
//
// These guard the exact class of bug fixed in `luaur-vm/src/functions/
// pusherror.rs` (issue #3): that builder constructs the position prefix with
// `format_args!` over `CStr::..to_string_lossy()` temporaries, a shape that is
// easy to get subtly wrong (E0716 — borrowed temporaries dropped too early) and
// that, being on the error path, is not hit by the happy-path suite. A latent
// break there can also hide behind a warm build cache, so exercising it at
// runtime — and asserting the produced message — is the durable safety net.

use luaur_rt::{Lua, Result};

/// Run `src` (named `name`) to completion and return the error message it
/// raises. Panics if it did *not* error.
fn err_message(lua: &Lua, src: &str, name: &str) -> String {
    let e = lua
        .load(src)
        .set_name(name)
        .exec()
        .expect_err("chunk was expected to raise a runtime error");
    e.to_string()
}

/// `error("...")` from a named chunk must yield `"<name>:<line>: <message>"` —
/// the canonical `pusherror` shape. The error sits on line 3 here.
#[test]
fn test_error_call_has_chunk_and_line_prefix() {
    let lua = Lua::new();
    let msg = err_message(&lua, "\n\nerror('boom')\n", "myChunk");
    assert!(msg.contains("boom"), "missing message text: {msg}");
    assert!(
        msg.contains("myChunk"),
        "missing chunk name in position prefix (pusherror chunkid): {msg}"
    );
    assert!(
        msg.contains(":3:"),
        "missing line-3 position from pusherror format: {msg}"
    );
}

/// A native runtime error (indexing `nil`) flows through `lua_g_runerror` and is
/// likewise position-prefixed. The fault is on line 2.
#[test]
fn test_runtime_type_error_has_position() {
    let lua = Lua::new();
    let msg = err_message(&lua, "local t = nil\nreturn t.x\n", "idxChunk");
    assert!(
        msg.to_lowercase().contains("index"),
        "expected an index error: {msg}"
    );
    assert!(msg.contains(":2:"), "missing line-2 position: {msg}");
}

/// The line number tracks the actual fault location, not a constant — a regress
/// in `pusherror`'s `{}:{}: {}` ordering/args would surface here.
#[test]
fn test_error_line_number_is_accurate() {
    let lua = Lua::new();
    // 5 leading newlines -> `error` is on line 6.
    let msg = err_message(&lua, "\n\n\n\n\nerror('deep')\n", "lineChunk");
    assert!(msg.contains("deep"), "missing message: {msg}");
    assert!(
        msg.contains(":6:"),
        "expected the fault on line 6 (pusherror line arg): {msg}"
    );
}

/// The coroutine-resume error path (`resume` -> `luaG_pusherror`) is a *separate*
/// caller of `pusherror`; make sure it, too, produces a positioned message.
#[test]
fn test_coroutine_resume_error_has_position() -> Result<()> {
    let lua = Lua::new();
    let func = lua
        .load("\nerror('co-boom')\n") // error on line 2
        .set_name("coChunk")
        .into_function()?;
    let thread = lua.create_thread(func)?;
    let err = thread
        .resume::<()>(())
        .expect_err("coroutine was expected to error");
    let msg = err.to_string();
    assert!(msg.contains("co-boom"), "missing message: {msg}");
    assert!(
        msg.contains(":2:"),
        "missing position from coroutine-resume error path: {msg}"
    );
    Ok(())
}
