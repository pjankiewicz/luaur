// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// Dropped (deferred / out-of-scope luaur-rt features):
//   - RegistryKey conversions (create_registry_value / registry_value)
//   - Thread / AnyUserData typed read-back (UserDataRef) conversions
//   - Either<L, R>, BorrowedStr, BorrowedBytes, BString, OsString, PathBuf
//   - `lua.convert` / `lua.unpack` helpers (we use FromLua/IntoLua directly)
//   - buffer-based conversions (luau buffer deferred)

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::CString;

use luaur_rt::{Error, FromLua, Function, IntoLua, Lua, Result, Table, Value};

#[test]
fn test_value_into_lua() -> Result<()> {
    let lua = Lua::new();

    // Direct conversion
    let v = Value::Boolean(true);
    let v2 = v.clone().into_lua(&lua)?;
    assert_eq!(v, v2);

    // Push into stack
    let table = lua.create_table();
    table.set("v", v.clone())?;
    assert_eq!(v, table.get::<Value>("v")?);

    Ok(())
}

#[test]
fn test_string_into_lua() -> Result<()> {
    let lua = Lua::new();

    // Direct conversion
    let s = lua.create_string("hello, world!");
    let s2 = s.clone().into_lua(&lua)?;
    assert_eq!(s, *s2.as_string().unwrap());

    // Push into stack
    let table = lua.create_table();
    table.set("s", s.clone())?;
    assert_eq!(s, table.get::<String>("s")?);

    Ok(())
}

#[test]
fn test_string_from_lua() -> Result<()> {
    let lua = Lua::new();

    // From stack
    let f = lua.create_function(|_, s: luaur_rt::LuaString| Ok(s))?;
    let s = f.call::<String>("hello, world!")?;
    assert_eq!(s, "hello, world!");

    // Should fallback to default conversion (number -> string)
    let s = f.call::<String>(42)?;
    assert_eq!(s, "42");

    Ok(())
}

#[test]
fn test_table_into_lua() -> Result<()> {
    let lua = Lua::new();

    // Direct conversion
    let t = lua.create_table();
    let t2 = t.clone().into_lua(&lua)?;
    assert_eq!(&t, t2.as_table().unwrap());

    // Push into stack
    let f = lua.create_function(|_, (t, s): (Table, String)| t.set("s", s))?;
    f.call::<()>((t.clone(), "hello"))?;
    assert_eq!("hello", t.get::<String>("s")?);

    Ok(())
}

#[test]
fn test_function_into_lua() -> Result<()> {
    let lua = Lua::new();

    // Direct conversion
    let f = lua.create_function(|_, ()| Ok::<_, Error>(()))?;
    let f2 = f.clone().into_lua(&lua)?;
    assert_eq!(&f, f2.as_function().unwrap());

    // Push into stack
    let table = lua.create_table();
    table.set("f", f.clone())?;
    assert_eq!(f, table.get::<Function>("f")?);

    Ok(())
}

#[test]
fn test_function_from_lua() -> Result<()> {
    let lua = Lua::new();

    assert!(lua.globals().get::<Function>("print").is_ok());
    match lua.globals().get::<Function>("math") {
        Err(err @ Error::FromLuaConversionError { .. }) => {
            assert_eq!(err.to_string(), "error converting Lua table to Function");
        }
        _ => panic!("expected `Error::FromLuaConversionError`"),
    }

    Ok(())
}

#[test]
fn test_bool_into_lua() -> Result<()> {
    let lua = Lua::new();

    // Direct conversion
    assert!(true.into_lua(&lua)?.is_boolean());

    // Push into stack
    let table = lua.create_table();
    table.set("b", true)?;
    assert_eq!(true, table.get::<bool>("b")?);

    Ok(())
}

#[test]
fn test_bool_from_lua() -> Result<()> {
    let lua = Lua::new();

    // `print` is a function (truthy)
    assert!(lua.globals().get::<bool>("print")?);
    // Numbers are truthy, nil is falsy (Lua truthiness)
    assert!(bool::from_lua(Value::Integer(123), &lua)?);
    assert!(!bool::from_lua(Value::Nil, &lua)?);

    Ok(())
}

#[test]
fn test_integer_from_lua() -> Result<()> {
    let lua = Lua::new();

    // From stack
    let f = lua.create_function(|_, i: i32| Ok(i))?;
    assert_eq!(f.call::<i32>(42)?, 42);

    // Out of range
    assert!(f.call::<i32>(i64::MAX).is_err());

    // Should fallback to default conversion (string -> integer)
    assert_eq!(f.call::<i32>("42")?, 42);

    Ok(())
}

#[test]
fn test_float_from_lua() -> Result<()> {
    let lua = Lua::new();

    // From stack
    let f = lua.create_function(|_, f: f32| Ok(f))?;
    assert_eq!(f.call::<f32>(42.0)?, 42.0);

    // Out of range (but never fails)
    let val = f.call::<f32>(f64::MAX)?;
    assert!(val.is_infinite());

    // Should fallback to default conversion (string -> float)
    assert_eq!(f.call::<f32>("42.0")?, 42.0);

    Ok(())
}

#[test]
fn test_conv_vec() -> Result<()> {
    let lua = Lua::new();

    let v = vec![1, 2, 3];
    lua.globals().set("v", v.clone())?;
    let v2: Vec<i32> = lua.globals().get("v")?;
    assert_eq!(v, v2);

    Ok(())
}

#[test]
fn test_conv_hashmap() -> Result<()> {
    let lua = Lua::new();

    let mut map = HashMap::new();
    map.insert("hello".to_string(), "world".to_string());
    lua.globals().set("map", map.clone())?;
    let map2: HashMap<String, String> = lua.globals().get("map")?;
    assert_eq!(map, map2);

    Ok(())
}

#[test]
fn test_conv_hashset() -> Result<()> {
    let lua = Lua::new();

    let mut set = HashSet::new();
    set.insert("hello".to_string());
    set.insert("world".to_string());
    lua.globals().set("set", set.clone())?;
    let set2: HashSet<String> = lua.globals().get("set")?;
    assert_eq!(set, set2);

    let set3 = lua.load(r#"return {"a", "b", "c"}"#).eval::<HashSet<String>>()?;
    assert_eq!(
        set3,
        HashSet::from(["a".to_string(), "b".to_string(), "c".to_string()])
    );

    Ok(())
}

#[test]
fn test_conv_btreemap() -> Result<()> {
    let lua = Lua::new();

    let mut map = BTreeMap::new();
    map.insert("hello".to_string(), "world".to_string());
    lua.globals().set("map", map.clone())?;
    let map2: BTreeMap<String, String> = lua.globals().get("map")?;
    assert_eq!(map, map2);

    Ok(())
}

#[test]
fn test_conv_btreeset() -> Result<()> {
    let lua = Lua::new();

    let mut set = BTreeSet::new();
    set.insert("hello".to_string());
    set.insert("world".to_string());
    lua.globals().set("set", set.clone())?;
    let set2: BTreeSet<String> = lua.globals().get("set")?;
    assert_eq!(set, set2);

    let set3 = lua.load(r#"return {"a", "b", "c"}"#).eval::<BTreeSet<String>>()?;
    assert_eq!(
        set3,
        BTreeSet::from(["a".to_string(), "b".to_string(), "c".to_string()])
    );

    Ok(())
}

#[test]
fn test_conv_cstring() -> Result<()> {
    let lua = Lua::new();

    let s = CString::new(b"hello".to_vec()).unwrap();
    lua.globals().set("s", s.clone())?;
    let s2: CString = lua.globals().get("s")?;
    assert_eq!(s, s2);

    Ok(())
}

#[test]
fn test_conv_boxed_str() -> Result<()> {
    let lua = Lua::new();

    let s = String::from("hello").into_boxed_str();
    lua.globals().set("s", s.clone())?;
    let s2: Box<str> = lua.globals().get("s")?;
    assert_eq!(s, s2);

    Ok(())
}

#[test]
fn test_conv_boxed_slice() -> Result<()> {
    let lua = Lua::new();

    let v = vec![1, 2, 3].into_boxed_slice();
    lua.globals().set("v", v.clone())?;
    let v2: Box<[i32]> = lua.globals().get("v")?;
    assert_eq!(v, v2);

    Ok(())
}

#[test]
fn test_conv_array() -> Result<()> {
    let lua = Lua::new();

    let v = [1, 2, 3];
    lua.globals().set("v", v)?;
    let v2: [i32; 3] = lua.globals().get("v")?;
    assert_eq!(v, v2);

    let v2 = lua.globals().get::<[i32; 4]>("v");
    assert!(matches!(v2, Err(Error::FromLuaConversionError { .. })));

    Ok(())
}

#[test]
fn test_option_into_from_lua() -> Result<()> {
    let lua = Lua::new();

    // Direct conversion
    let v = Some(42);
    let v2 = v.into_lua(&lua)?;
    assert_eq!(v, v2.as_i32());

    // Push into stack / get from stack
    let f = lua.create_function(|_, v: Option<i32>| Ok(v))?;
    assert_eq!(f.call::<Option<i32>>(Some(42))?, Some(42));
    assert_eq!(f.call::<Option<i32>>(Option::<i32>::None)?, None);
    assert_eq!(f.call::<Option<i32>>(())?, None);

    Ok(())
}

#[test]
fn test_char_into_lua() -> Result<()> {
    let lua = Lua::new();

    let v = '\u{1f980}'; // crab
    let v2 = v.into_lua(&lua)?;
    assert_eq!(*v2.as_string().unwrap(), v.to_string());

    Ok(())
}

#[test]
fn test_char_from_lua() -> Result<()> {
    let lua = Lua::new();

    assert_eq!(char::from_lua(lua.create_string("A").into_lua(&lua)?, &lua)?, 'A');
    assert_eq!(char::from_lua(Value::Integer(65), &lua)?, 'A');
    assert_eq!(char::from_lua(Value::Integer(128175), &lua)?, '\u{1f4af}');
    assert!(char::from_lua(Value::Integer(5456324), &lua)
        .is_err_and(|e| e.to_string().contains("out of range")));
    assert!(
        char::from_lua(lua.create_string("hello").into_lua(&lua)?, &lua)
            .is_err_and(|e| e.to_string().contains("exactly one char"))
    );
    assert!(char::from_lua(Value::Table(lua.create_table()), &lua)
        .is_err_and(|e| e.to_string().contains("expected string or integer")));

    Ok(())
}
