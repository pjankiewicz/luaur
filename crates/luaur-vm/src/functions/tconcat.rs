use crate::enums::lua_type::lua_Type;
use crate::functions::addfield::addfield;
use crate::functions::lua_l_addlstring::lua_l_addlstring;
use crate::functions::lua_l_buffinit::lua_l_buffinit;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_l_optlstring::lua_l_optlstring;
use crate::functions::lua_l_pushresult::lua_l_pushresult;
use crate::functions::lua_objlen::lua_objlen;
use crate::macros::hvalue::hvalue;
use crate::records::lua_l_strbuf::LuaLStrbuf;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_tconcat"]
pub unsafe fn tconcat(L: *mut lua_State) -> core::ffi::c_int {
    let mut lsep: usize = 0;
    let sep = lua_l_optlstring(L, 2, core::ptr::null(), &mut lsep);
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    let i = lua_l_optinteger(L, 3, 1);
    let last = lua_objlen(L, 1);
    let last = lua_l_optinteger(L, 4, last);

    let t = hvalue!((*L).base);

    let mut b: LuaLStrbuf = LuaLStrbuf {
        p: core::ptr::null_mut(),
        end: core::ptr::null_mut(),
        L: core::ptr::null_mut(),
        storage: core::ptr::null_mut(),
        buffer: [0; 512],
    };
    lua_l_buffinit(L, &mut b);
    let mut current_i = i;
    while current_i < last {
        addfield(L, &mut b, current_i, t);
        if lsep != 0 {
            lua_l_addlstring(&mut b, sep, lsep);
        }
        current_i += 1;
    }
    if current_i == last {
        addfield(L, &mut b, current_i, t);
    }
    lua_l_pushresult(&mut b);
    1
}
