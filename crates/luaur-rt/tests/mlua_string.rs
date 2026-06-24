// Adapted from mlua (https://github.com/mlua-rs/mlua), MIT License,
// © 2019 Aleksandr Orlenko / mlua authors. See tests/ATTRIBUTION.md.
//
// Dropped (deferred luaur-rt features): test_string_wrap (LuaString::wrap, a
// detached-string constructor) and test_external_string (lua55-only
// create_external_string).

use std::borrow::Cow;
use std::collections::HashSet;

use luaur_rt::{Lua, LuaString, Result};

#[test]
fn test_string_compare() {
    let lua = Lua::new();

    fn with_str<F: FnOnce(LuaString)>(lua: &Lua, s: &str, f: F) {
        f(lua.create_string(s));
    }

    // Tests that all comparisons we want to have are usable
    with_str(&lua, "teststring", |t| assert_eq!(t, "teststring")); // &str
    with_str(&lua, "teststring", |t| assert_eq!(t, b"teststring")); // &[u8; N]
    with_str(&lua, "teststring", |t| assert_eq!(t, b"teststring".to_vec())); // Vec<u8>
    with_str(&lua, "teststring", |t| assert_eq!(t, "teststring".to_string())); // String
    with_str(&lua, "teststring", |t| assert_eq!(t, t)); // LuaString
    with_str(&lua, "teststring", |t| {
        assert_eq!(t, Cow::from(b"teststring".as_ref())) // Cow (borrowed)
    });
    with_str(&lua, "bla", |t| assert_eq!(t, Cow::from(b"bla".to_vec()))); // Cow (owned)

    // Test ordering
    with_str(&lua, "a", |a| {
        assert!(!(a < a));
        assert!(!(a > a));
    });
    with_str(&lua, "a", |a| assert!(a < "b"));
    with_str(&lua, "a", |a| assert!(a < b"b"));
    with_str(&lua, "a", |a| with_str(&lua, "b", |b| assert!(a < b)));

    // Long strings (not interned by Lua)
    let long_str = "abc".repeat(100);
    with_str(&lua, &long_str, |s1| {
        with_str(&lua, &long_str, |s2| assert_eq!(s1, s2))
    });
}

#[test]
fn test_string_views() -> Result<()> {
    let lua = Lua::new();

    lua.load(
        r#"
        ok = "null bytes are valid utf-8, wh\0 knew?"
        err = "but \255 isn't :("
        empty = ""
    "#,
    )
    .exec()?;

    let globals = lua.globals();
    let ok: LuaString = globals.get("ok")?;
    let err: LuaString = globals.get("err")?;
    let empty: LuaString = globals.get("empty")?;

    assert_eq!(ok.to_str()?, "null bytes are valid utf-8, wh\0 knew?");
    assert_eq!(ok.to_string_lossy(), "null bytes are valid utf-8, wh\0 knew?");
    assert_eq!(ok.as_bytes(), b"null bytes are valid utf-8, wh\0 knew?".to_vec());

    assert!(err.to_str().is_err());
    assert_eq!(err.as_bytes(), b"but \xff isn't :(".to_vec());

    assert_eq!(empty.to_str()?, "");
    assert_eq!(empty.as_bytes_with_nul(), vec![0]);
    assert_eq!(empty.as_bytes(), Vec::<u8>::new());

    Ok(())
}

#[test]
fn test_string_from_bytes() -> Result<()> {
    let lua = Lua::new();

    let rs = lua.create_string([0, 1, 2, 3, 0, 1, 2, 3]);
    assert_eq!(rs.as_bytes(), vec![0, 1, 2, 3, 0, 1, 2, 3]);

    Ok(())
}

#[test]
fn test_string_hash() -> Result<()> {
    let lua = Lua::new();

    let set: HashSet<LuaString> = lua.load(r#"return {"hello", "world", "abc", 321}"#).eval()?;
    assert_eq!(set.len(), 4);
    assert!(set.contains(&lua.create_string("hello")));
    assert!(set.contains(&lua.create_string("world")));
    assert!(set.contains(&lua.create_string("abc")));
    assert!(set.contains(&lua.create_string("321")));
    assert!(!set.contains(&lua.create_string("Hello")));

    Ok(())
}

#[test]
fn test_string_fmt_debug() -> Result<()> {
    let lua = Lua::new();

    // Valid utf8
    let s = lua.create_string("hello");
    assert_eq!(format!("{s:?}"), r#""hello""#);
    assert_eq!(format!("{:?}", s.to_str()?), r#""hello""#);
    assert_eq!(format!("{:?}", s.as_bytes()), "[104, 101, 108, 108, 111]");

    // Invalid utf8
    let s = lua.create_string(b"hello\0world\r\n\t\xf0\x90\x80");
    assert_eq!(format!("{s:?}"), r#"b"hello\0world\r\n\t\xf0\x90\x80""#);

    Ok(())
}

#[test]
fn test_string_pointer() -> Result<()> {
    let lua = Lua::new();

    let str1 = lua.create_string("hello");
    let str2 = lua.create_string("hello");

    // Lua uses string interning, so these should be the same
    assert_eq!(str1.to_pointer(), str2.to_pointer());

    Ok(())
}

#[test]
fn test_string_display() -> Result<()> {
    let lua = Lua::new();

    let s = lua.create_string("hello");
    assert_eq!(format!("{}", s.display()), "hello");

    // With invalid utf8
    let s = lua.create_string(b"hello\0world\xFF");
    assert_eq!(format!("{}", s.display()), "hello\0world\u{fffd}");

    Ok(())
}

#[test]
fn test_bytes_into_iter() -> Result<()> {
    let lua = Lua::new();

    let s = lua.create_string("hello");
    let bytes = s.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        assert_eq!(b, s.as_bytes()[i]);
    }

    Ok(())
}
