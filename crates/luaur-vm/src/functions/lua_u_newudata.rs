use crate::enums::lua_type::lua_Type;
use crate::functions::lua_m_newgco::luaM_newgco_;
use crate::functions::lua_m_toobig::lua_m_toobig;
use crate::macros::lua_c_init::luaC_init;
use crate::macros::sizeudata::sizeudata;
use crate::records::udata::Udata;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn lua_u_newudata(L: *mut lua_State, s: usize, tag: c_int) -> *mut Udata {
    if s > c_int::MAX as usize - core::mem::size_of::<Udata>() {
        lua_m_toobig(L);
    }

    let u = luaM_newgco_(L, sizeudata(s), (*L).activememcat) as *mut Udata;
    luaC_init!(L, u, lua_Type::LUA_TUSERDATA as c_int);
    (*u).len = s as c_int;
    (*u).metatable = core::ptr::null_mut();
    LUAU_ASSERT!(tag >= 0 && tag <= 255);
    (*u).tag = tag as u8;
    u
}

#[allow(non_snake_case)]
#[export_name = "luaur_luaU_newudata"]
pub unsafe extern "C" fn luaU_newudata(L: *mut lua_State, s: usize, tag: c_int) -> *mut Udata {
    lua_u_newudata(L, s, tag)
}
