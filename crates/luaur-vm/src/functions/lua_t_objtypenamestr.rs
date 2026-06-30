use crate::functions::lua_h_getstr::lua_h_getstr;
use crate::macros::lightuserdatatag::lightuserdatatag;
use crate::macros::lua_lutag_limit::LUA_LUTAG_LIMIT;
use crate::macros::tsvalue::tsvalue;
use crate::macros::ttislightuserdata::ttislightuserdata;
use crate::macros::ttisstring::ttisstring;
use crate::macros::ttisuserdata::ttisuserdata;
use crate::macros::ttype::ttype;
use crate::macros::utag_proxy::UTAG_PROXY;
use crate::macros::uvalue::uvalue;
use crate::records::lua_table::LuaTable;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::t_string::TString;
use crate::type_aliases::t_value::TValue;

#[allow(non_snake_case)]
pub unsafe fn lua_t_objtypenamestr(L: *mut lua_State, o: *const TValue) -> *const TString {
    // Userdata created by the environment can have a custom type name set in the individual metatable
    // If there is no custom name, 'userdata' is returned
    if ttisuserdata!(o)
        && (*uvalue!(o)).tag as i32 != UTAG_PROXY
        && !(*uvalue!(o)).metatable.is_null()
    {
        // TM_TYPE is index 19 in the tag method names array (TMS enum)
        let func: unsafe fn(*mut LuaTable, *mut TString) -> *const TValue =
            core::mem::transmute(lua_h_getstr as *const core::ffi::c_void);

        let type_ = func(
            (*uvalue!(o)).metatable,
            (*(*L).global).tmname[19] as *mut TString,
        );

        if ttisstring!(type_) {
            return tsvalue!(type_);
        }

        return (*(*L).global).ttname[ttype!(o) as usize];
    }

    // Tagged lightuserdata can be named using lua_setlightuserdataname
    if ttislightuserdata!(o) {
        let tag = lightuserdatatag!(o);

        if (tag as u32) < LUA_LUTAG_LIMIT as u32 {
            let name = (*(*L).global).lightuserdataname[tag as usize];
            if !name.is_null() {
                return name;
            }
        }
    }

    // For all types except userdata and table, a global metatable can be set with a global name override
    let mt = (*(*L).global).mt[ttype!(o) as usize];
    if !mt.is_null() {
        // TM_TYPE is index 19
        let func: unsafe fn(*mut LuaTable, *mut TString) -> *const TValue =
            core::mem::transmute(lua_h_getstr as *const core::ffi::c_void);

        let type_ = func(mt, (*(*L).global).tmname[19] as *mut TString);

        if ttisstring!(type_) {
            return tsvalue!(type_);
        }
    }

    (*(*L).global).ttname[ttype!(o) as usize]
}

#[export_name = "luaur_luaT_objtypenamestr"]
pub unsafe extern "C" fn lua_t_objtypenamestr_export(
    L: *mut lua_State,
    o: *const TValue,
) -> *const core::ffi::c_void {
    lua_t_objtypenamestr(L, o).cast()
}
