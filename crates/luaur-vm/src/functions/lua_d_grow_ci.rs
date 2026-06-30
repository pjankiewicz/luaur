use crate::enums::lua_status::lua_Status;
use crate::functions::lua_d_realloc_ci::lua_d_realloc_ci;
use crate::functions::lua_d_throw_ldo::lua_d_throw;
use crate::functions::lua_g_runerror_l::lua_g_runerror_l;
use crate::macros::luai_maxcalls::LUAI_MAXCALLS;
use crate::type_aliases::call_info::CallInfo;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_luaD_growCI"]
#[allow(non_snake_case)]
pub unsafe fn luaD_growCI(L: *mut lua_State) -> *mut CallInfo {
    // allow extra stack space to handle stack overflow in xpcall
    let hardlimit: i32 = LUAI_MAXCALLS + (LUAI_MAXCALLS >> 3);

    if (*L).size_ci >= hardlimit {
        // error while handling stack error
        lua_d_throw(L, lua_Status::LUA_ERRERR as i32);
    }

    let request: i32 = (*L).size_ci * 2;
    let new_size = if (*L).size_ci >= LUAI_MAXCALLS {
        hardlimit
    } else if request < LUAI_MAXCALLS {
        request
    } else {
        LUAI_MAXCALLS
    };

    lua_d_realloc_ci(L, new_size);

    if (*L).size_ci > LUAI_MAXCALLS {
        lua_g_runerror_l(L, core::ptr::null(), format_args!("stack overflow"));
    }

    (*L).ci = (*L).ci.add(1);
    (*L).ci
}

#[allow(non_snake_case)]
pub use luaD_growCI as lua_d_grow_ci;
