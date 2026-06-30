use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_encodepointer"]
pub unsafe fn lua_encodepointer(l: *mut lua_State, p: usize) -> usize {
    let g = (*l).global;
    let p = p as u64;
    let ptrenckey = (*g).ptrenckey;

    let result = (ptrenckey[0].wrapping_mul(p).wrapping_add(ptrenckey[2]))
        ^ (ptrenckey[1].wrapping_mul(p).wrapping_add(ptrenckey[3]));

    result as usize
}
