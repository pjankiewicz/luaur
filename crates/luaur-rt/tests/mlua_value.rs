// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// Dropped / trimmed (deferred or non-Luau-applicable luaur-rt features):
//   - LightUserData / Value::NULL                (no light-userdata support)
//   - Value::Vector / Value::Buffer / Value::Thread / Value::Other
//   - register_userdata_type / create_any_userdata typed read-back, __todebug
//   - test_value_exhaustive_match (mlua's full variant set)
//   - coroutine.create-based comparisons (threads deferred)

use luaur_rt::{Error, Lua, MultiValue, Result, UserData, Value};

#[test]
fn test_value_eq() -> Result<()> {
    let lua = Lua::new();
    let globals = lua.globals();

    // DEVIATION (see mlua_table::test_table_equals): Luau invokes `__eq` only
    // when *both* operands share the metamethod, so the `__eq` pair below uses
    // a single shared metatable.
    lua.load(
        r#"
        local mt = { __eq = function(a, b) return a[1] == b[1] end }
        table1 = setmetatable({1}, mt)
        table2 = setmetatable({1}, mt)
        string1 = "hello"
        string2 = "hello"
        num1 = 1
        num2 = 1.0
        num3 = "1"
        func1 = function() end
        func2 = func1
        func3 = function() end
    "#,
    )
    .exec()?;

    let table1: Value = globals.get("table1")?;
    let table2: Value = globals.get("table2")?;
    let string1: Value = globals.get("string1")?;
    let string2: Value = globals.get("string2")?;
    let num1: Value = globals.get("num1")?;
    let num2: Value = globals.get("num2")?;
    let num3: Value = globals.get("num3")?;
    let func1: Value = globals.get("func1")?;
    let func2: Value = globals.get("func2")?;
    let func3: Value = globals.get("func3")?;

    assert!(table1 != table2); // distinct objects (reference identity)
    assert!(table1.equals(&table2)?); // shared `__eq` => values match
    assert!(string1 == string2);
    assert!(string1.equals(&string2)?);
    assert!(num1 == num2); // 1 == 1.0
    assert!(num1.equals(&num2)?);
    assert!(num1 != num3); // number vs string
    assert!(func1 == func2);
    assert!(func1 != func3);
    assert!(!func1.equals(&func3)?);

    // Pointer identity behavior.
    assert!(!table1.to_pointer().is_null());
    assert!(table1.to_pointer() != table2.to_pointer());
    // Strings are interned, so equal string values share a pointer.
    assert!(string1.to_pointer() == string2.to_pointer() && !string1.to_pointer().is_null());
    assert!(func1.to_pointer() == func2.to_pointer());
    assert!(num1.to_pointer().is_null());

    Ok(())
}

#[test]
fn test_multi_value() {
    let mut multi_value = MultiValue::new();
    assert_eq!(multi_value.len(), 0);
    assert_eq!(multi_value.get(0), None);

    multi_value.push_front(Value::Number(2.));
    multi_value.push_front(Value::Number(1.));
    assert_eq!(multi_value.get(0), Some(&Value::Number(1.)));
    assert_eq!(multi_value.get(1), Some(&Value::Number(2.)));

    assert_eq!(multi_value.pop_front(), Some(Value::Number(1.)));
    assert_eq!(multi_value[0], Value::Number(2.));

    multi_value.clear();
    assert!(multi_value.is_empty());
}

#[test]
fn test_value_to_pointer() -> Result<()> {
    let lua = Lua::new();

    let globals = lua.globals();
    lua.load(
        r#"
        mytable = {}
        mystring = "hello"
        mynum = 1
        myfunc = function() end
    "#,
    )
    .exec()?;

    let table: Value = globals.get("mytable")?;
    let string: Value = globals.get("mystring")?;
    let num: Value = globals.get("mynum")?;
    let func: Value = globals.get("myfunc")?;
    let ud: Value = {
        struct U;
        impl UserData for U {}
        Value::UserData(lua.create_userdata(U)?)
    };

    assert!(!table.to_pointer().is_null());
    assert!(!string.to_pointer().is_null());
    assert!(num.to_pointer().is_null());
    assert!(!func.to_pointer().is_null());
    assert!(!ud.to_pointer().is_null());

    Ok(())
}

#[test]
fn test_value_to_string() -> Result<()> {
    let lua = Lua::new();

    assert_eq!(Value::Nil.to_string()?, "nil");
    assert_eq!(Value::Nil.type_name(), "nil");
    assert_eq!(Value::Boolean(true).to_string()?, "true");
    assert_eq!(Value::Boolean(true).type_name(), "boolean");
    assert_eq!(Value::Integer(1).to_string()?, "1");
    assert_eq!(Value::Number(34.59).to_string()?, "34.59");
    assert_eq!(Value::Number(34.59).type_name(), "number");

    let s = Value::String(lua.create_string("hello"));
    assert_eq!(s.to_string()?, "hello");
    assert_eq!(s.type_name(), "string");

    let table: Value = lua.load("return {}").eval()?;
    assert!(table.to_string()?.starts_with("table:"));
    let table: Value = lua
        .load("return setmetatable({}, {__tostring = function() return 'test table' end})")
        .eval()?;
    assert_eq!(table.to_string()?, "test table");
    assert_eq!(table.type_name(), "table");

    let func: Value = lua.load("return function() end").eval()?;
    assert!(func.to_string()?.starts_with("function:"));
    assert_eq!(func.type_name(), "function");

    let err = Value::Error(Box::new(Error::runtime("test error")));
    assert_eq!(err.to_string()?, "runtime error: test error");
    assert_eq!(err.type_name(), "error");

    Ok(())
}

#[test]
fn test_value_conversions() -> Result<()> {
    let lua = Lua::new();

    assert!(Value::Nil.is_nil());
    assert!(Value::Boolean(true).is_boolean());
    assert_eq!(Value::Boolean(false).as_boolean(), Some(false));
    assert!(Value::Integer(1).is_integer());
    assert_eq!(Value::Integer(1).as_integer(), Some(1));
    assert_eq!(Value::Integer(1).as_i32(), Some(1i32));
    assert_eq!(Value::Integer(1).as_u32(), Some(1u32));
    assert_eq!(Value::Integer(1).as_i64(), Some(1i64));
    assert_eq!(Value::Integer(1).as_u64(), Some(1u64));
    assert_eq!(Value::Integer(1).as_isize(), Some(1isize));
    assert_eq!(Value::Integer(1).as_usize(), Some(1usize));
    assert!(Value::Number(1.23).is_number());
    assert_eq!(Value::Number(1.23).as_number(), Some(1.23));
    assert_eq!(Value::Number(1.23).as_f32(), Some(1.23f32));
    assert_eq!(Value::Number(1.23).as_f64(), Some(1.23f64));
    assert!(Value::String(lua.create_string("hello")).is_string());
    assert_eq!(
        Value::String(lua.create_string("hello")).as_string().unwrap(),
        "hello"
    );
    assert_eq!(Value::String(lua.create_string("hello")).to_string()?, "hello");
    assert!(Value::Table(lua.create_table()).is_table());
    assert!(Value::Table(lua.create_table()).as_table().is_some());
    assert!(Value::Function(lua.create_function(|_, ()| Ok(())).unwrap()).is_function());
    assert!(
        Value::Function(lua.create_function(|_, ()| Ok(())).unwrap())
            .as_function()
            .is_some()
    );

    assert!(Value::Error(Box::new(Error::runtime("some error"))).is_error());
    assert_eq!(
        Value::Error(Box::new(Error::runtime("some error")))
            .as_error()
            .unwrap()
            .to_string(),
        "runtime error: some error"
    );

    Ok(())
}
