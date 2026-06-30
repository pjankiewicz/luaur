use crate::functions::currentline::currentline;
use crate::functions::getluaproto::get_lua_proto;
use crate::functions::lua_o_chunkid::lua_o_chunkid;
use crate::functions::lua_o_pushfstring::luaO_pushfstring;
use crate::functions::lua_pushstring::lua_pushstring;
use crate::macros::getstr::getstr;
use crate::macros::is_lua::isLua;
use crate::macros::lua_idsize::LUA_IDSIZE;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_char, CStr};

#[export_name = "luaur_pusherror"]
pub unsafe fn pusherror(L: *mut lua_State, msg: *const c_char) {
    let ci = (*L).ci;

    // isLua! macro expects a pointer to CallInfo, not a dereferenced struct.
    if isLua!(ci) {
        let proto = get_lua_proto(ci);
        let source = (*proto).source;

        let mut chunkbuf: [c_char; LUA_IDSIZE as usize] = [0; LUA_IDSIZE as usize];
        let chunkid = lua_o_chunkid(
            chunkbuf.as_mut_ptr(),
            chunkbuf.len(),
            getstr(source),
            (*source).len as usize,
        );

        let line = currentline(L, ci);

        let fmt = b"%s:%d: %s\0";
        let fmt_ptr = fmt.as_ptr() as *const c_char;

        // luaO_pushfstring expects a fmt C string + Rust fmt::Arguments.
        // We use CStr::from_ptr to safely convert C strings to Rust string-like objects for formatting.
        //
        // The `to_string_lossy()` `Cow`s must be bound to locals so they outlive
        // the `format_args!` that borrows them: a `fmt::Arguments` can never
        // outlive its captured temporaries, so storing it in a `let` and using
        // it in a *later* statement dangles (E0716 — the temporaries are dropped
        // at the end of the `let`). Inline `format_args!` into the call instead.
        let chunk = CStr::from_ptr(chunkid).to_string_lossy();
        let msg_str = CStr::from_ptr(msg).to_string_lossy();
        luaO_pushfstring(L, fmt_ptr, format_args!("{}:{}: {}", chunk, line, msg_str));
    } else {
        lua_pushstring(L, msg);
    }
}
