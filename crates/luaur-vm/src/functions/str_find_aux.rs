use crate::functions::lmemfind::lmemfind;
use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::functions::lua_toboolean::lua_toboolean;
use crate::functions::nospecials::nospecials;
use crate::functions::posrelat::posrelat;
use crate::functions::prepstate::prepstate;
use crate::functions::push_captures::push_captures;
use crate::functions::r#match::match_item;
use crate::functions::reprepstate::reprepstate;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::match_state::MatchState;
use core::ffi::c_char;
use core::ffi::c_int;

#[export_name = "luaur_str_find_aux"]
pub unsafe fn str_find_aux(l: *mut lua_State, find: c_int) -> c_int {
    let mut ls: usize = 0;
    let s = lua_l_checklstring(l, 1, &mut ls);

    let mut lp: usize = 0;
    let p = lua_l_checklstring(l, 2, &mut lp);

    let mut init = posrelat(lua_l_optinteger(l, 3, 1), ls);
    if init < 1 {
        init = 1;
    } else if init > ls as i32 + 1 {
        lua_pushnil(l);
        return 1;
    }

    let do_plain = find != 0 && (lua_toboolean(l, 4) != 0 || nospecials(p, lp) != 0);

    if do_plain {
        let s2 = lmemfind(s.add((init - 1) as usize), ls + 1 - init as usize, p, lp);
        if !s2.is_null() {
            lua_pushinteger(l, (s2.offset_from(s) as c_int) + 1);
            lua_pushinteger(l, (s2.offset_from(s) as c_int) + lp as c_int);
            return 2;
        }
    } else {
        let mut ms: MatchState = core::mem::zeroed();

        let mut s1 = s.add((init - 1) as usize);
        let mut anchor = *p == b'^' as c_char;
        let mut p_ptr = p;
        let mut lp_len = lp;

        if anchor {
            p_ptr = p_ptr.add(1);
            lp_len -= 1;
        }

        prepstate(&mut ms, l, s, ls, p_ptr, lp_len);

        loop {
            reprepstate(&mut ms);
            let res = match_item(&mut ms, s1, p_ptr);
            if !res.is_null() {
                if find != 0 {
                    lua_pushinteger(l, (s1.offset_from(s) as c_int) + 1);
                    lua_pushinteger(l, res.offset_from(s) as c_int);
                    return push_captures(&mut ms, core::ptr::null(), core::ptr::null()) + 2;
                } else {
                    return push_captures(&mut ms, s1, res);
                }
            }

            let current = s1;
            s1 = s1.add(1);
            if !(current < ms.src_end) || anchor {
                break;
            }
        }
    }

    lua_pushnil(l);
    1
}
