// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// luaur-rt's userdata v1 supports *constructing* userdata and *using it from
// Lua* (methods, mutable methods, plain functions, and meta-methods). Rust-side
// typed read-back (`borrow`/`borrow_mut`/`take`/`is`/`type_id`),
// `UserDataFields`, `UserDataRef`, `MetaMethod` enum, `ObjectLike`,
// user-values, and `destroy`/once-methods are deferred. The mlua tests that
// rely on those are dropped; the ones below exercise the supported from-Lua
// surface, keeping mlua's behavioral assertions.

use std::cell::Cell;
use std::rc::Rc;

use luaur_rt::{Function, Lua, Result, UserData, UserDataMethods, Variadic};

#[test]
fn test_methods() -> Result<()> {
    struct MyUserData(i64);

    impl UserData for MyUserData {
        fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
            methods.add_method("get_value", |_, data, ()| Ok(data.0));
            methods.add_method_mut("set_value", |_, data, args: i64| {
                data.0 = args;
                Ok(())
            });
        }
    }

    let lua = Lua::new();
    let globals = lua.globals();
    globals.set("userdata", lua.create_userdata(MyUserData(42))?)?;
    lua.load(
        r#"
        function get_it()
            return userdata:get_value()
        end

        function set_it(i)
            return userdata:set_value(i)
        end
    "#,
    )
    .exec()?;
    let get = globals.get::<Function>("get_it")?;
    let set = globals.get::<Function>("set_it")?;
    assert_eq!(get.call::<i64>(())?, 42);
    set.call::<()>(100)?;
    assert_eq!(get.call::<i64>(())?, 100);

    Ok(())
}

#[test]
fn test_method_variadic() -> Result<()> {
    struct MyUserData(i64);

    impl UserData for MyUserData {
        fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
            methods.add_method("get", |_, data, ()| Ok(data.0));
            methods.add_method_mut("add", |_, data, vals: Variadic<i64>| {
                data.0 += vals.into_iter().sum::<i64>();
                Ok(())
            });
        }
    }

    let lua = Lua::new();
    let globals = lua.globals();
    globals.set("userdata", lua.create_userdata(MyUserData(0))?)?;
    lua.load("userdata:add(1, 5, -10)").exec()?;
    let total: i64 = lua.load("return userdata:get()").eval()?;
    assert_eq!(total, -4);

    Ok(())
}

#[test]
fn test_metamethods() -> Result<()> {
    // Arithmetic/comparison meta-methods used from Lua. luaur-rt meta-methods
    // receive `&self` and the other operand; we return a number, keeping mlua's
    // intent of "the `__add`/`__sub` metamethods fire and compute the result".
    //
    // DEVIATION: luaur-rt reserves `__index` on a userdata's metatable for its
    // method table, so a *custom* `__index` function (mlua's `MetaMethod::Index`
    // on a userdata that also has methods) is not exercised here.
    struct MyUserData(i64);

    impl UserData for MyUserData {
        fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
            methods.add_method("get", |_, data, ()| Ok(data.0));
            methods.add_meta_method("__add", |_, data, other: i64| Ok(data.0 + other));
            methods.add_meta_method("__sub", |_, data, other: i64| Ok(data.0 - other));
        }
    }

    let lua = Lua::new();
    let globals = lua.globals();
    globals.set("userdata1", lua.create_userdata(MyUserData(7))?)?;

    assert_eq!(lua.load("return userdata1 + 3").eval::<i64>()?, 10);
    assert_eq!(lua.load("return userdata1 - 2").eval::<i64>()?, 5);
    assert_eq!(lua.load("return userdata1:get()").eval::<i64>()?, 7);

    Ok(())
}

#[test]
fn test_functions() -> Result<()> {
    // `add_function` registers a plain function in the userdata namespace
    // (no `self`), callable as `ud.func(...)`.
    struct MyUserData(i64);

    impl UserData for MyUserData {
        fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
            methods.add_method("get_value", |_, data, ()| Ok(data.0));
            methods.add_function("get_constant", |_, ()| Ok(7));
        }
    }

    let lua = Lua::new();
    let globals = lua.globals();
    globals.set("userdata", lua.create_userdata(MyUserData(42))?)?;
    lua.load(
        r#"
        function get_it()
            return userdata:get_value()
        end
        function get_constant()
            return userdata.get_constant()
        end
    "#,
    )
    .exec()?;
    assert_eq!(globals.get::<Function>("get_it")?.call::<i64>(())?, 42);
    assert_eq!(globals.get::<Function>("get_constant")?.call::<i64>(())?, 7);

    Ok(())
}

#[test]
fn test_gc_userdata_access_after_collect() -> Result<()> {
    // DEVIATION: mlua's `test_gc_userdata` resurrects a userdata from a table's
    // `__gc` and asserts the resurrected handle is unusable. luaur's base
    // library does not expose `collectgarbage` (only `gcinfo`), and `__gc` on
    // plain tables is not part of the supported surface, so the resurrection
    // scenario cannot be expressed. We instead assert the supported invariant:
    // a userdata accessed via a *live* Rust handle keeps working across an
    // explicit `gc_collect()`.
    struct MyUserdata {
        id: u8,
    }

    impl UserData for MyUserdata {
        fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
            methods.add_method("access", |_, this, ()| {
                assert_eq!(this.id, 123);
                Ok(this.id)
            });
        }
    }

    let lua = Lua::new();
    let ud = lua.create_userdata(MyUserdata { id: 123 })?;
    lua.globals().set("userdata", ud.clone())?;

    // A GC cycle must not collect a userdata still reachable from a handle/global.
    lua.gc_collect()?;
    let id: u8 = lua.load("return userdata:access()").eval()?;
    assert_eq!(id, 123);

    Ok(())
}

#[test]
fn test_userdata_drop_runs_destructor() -> Result<()> {
    // The wrapped value's `Drop` must run when the userdata is collected.
    // (Uses the Rust `gc_collect` API since luaur lacks `collectgarbage`.)
    struct Tracked(Rc<Cell<bool>>);
    impl UserData for Tracked {}
    impl Drop for Tracked {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let dropped = Rc::new(Cell::new(false));
    let lua = Lua::new();
    lua.globals().set("ud", lua.create_userdata(Tracked(dropped.clone()))?)?;
    assert!(!dropped.get());

    // Make the userdata unreachable, then collect.
    lua.load("ud = nil").exec()?;
    lua.gc_collect()?;
    lua.gc_collect()?;
    assert!(dropped.get(), "userdata destructor should have run");

    Ok(())
}
