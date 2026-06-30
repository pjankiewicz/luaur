use crate::enums::lua_type::lua_Type;
use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_l_error_l::lua_l_error_l;
use crate::functions::lua_objlen::lua_objlen;
use crate::functions::lua_rawseti::lua_rawseti;
use crate::functions::moveelements::moveelements;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_tinsert"]
pub unsafe fn tinsert(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    let n = lua_objlen(L, 1);
    let top = lua_gettop(L);
    let pos: core::ffi::c_int;

    match top {
        2 => {
            // called with only 2 arguments
            pos = n + 1; // insert new element at the end
        }
        3 => {
            // 2nd argument is the position
            pos = lua_l_checkinteger(L, 2);

            // move up elements if necessary
            if 1 <= pos && pos <= n {
                moveelements(L, 1, 1, pos, n, pos + 1);
            }
        }
        _ => {
            lua_l_error_l(
                L,
                c"wrong number of arguments to 'insert'".as_ptr(),
                core::format_args!("wrong number of arguments to 'insert'"),
            );
            return 0;
        }
    }

    lua_rawseti(L, 1, pos); // t[pos] = v
    0
}
