#![cfg(feature = "checked-macros")]

#[test]
fn luau_macro_checks_inline_source() {
    let source = luaur::luau!(
        r#"
--!strict
local total: number = 40 + 2
return total
"#
    );

    assert!(source.contains("total"));
}

#[test]
fn luau_macro_checks_inline_modules() {
    let source = luaur::luau! {
        source = r#"
--!strict
local Config = require("@config")
local total: number = Config.base + 2
return total
"#,
        module = "game/Main",
        modules = {
            "@config" => r#"
--!strict
return { base = 40 }
"#,
        },
    };

    assert!(source.contains("@config"));
}

#[test]
fn luau_file_macro_checks_root_file() {
    let source = luaur::luau_file!("tests/scripts/standalone.luau");

    assert!(source.contains("standalone"));
}

#[test]
fn luau_file_macro_checks_module_map() {
    let source = luaur::luau_file! {
        root = "tests/scripts/main.luau",
        module = "game/Main",
        defs = "tests/scripts/host.d.luau",
        modules = {
            "game/Math" => "tests/scripts/math.luau",
            "@config" => "tests/scripts/config.luau",
        },
    };

    assert!(source.contains("game.Math"));
}
