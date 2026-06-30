use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::type_aliases::lua_state::lua_State;
use luaur_vm::type_aliases::stk_id::StkId;

use luaur_vm::functions::lua_v_getimport::lua_v_getimport;

#[allow(non_snake_case)]
pub unsafe fn get_import(L: *mut lua_State, res: StkId, id: u32, pc: u32) {
    let cl = clvalue!((*(*L).ci).func as *const luaur_vm::type_aliases::t_value::TValue);
    (*(*L).ci).savedpc = (*(*(*cl).inner.l).p).code.add(pc as usize);

    lua_v_getimport(L, (*cl).env, (*(*(*cl).inner.l).p).k, res, id, false);
}

#[export_name = "luaur_getImport"]
pub unsafe extern "C" fn getImport(L: *mut lua_State, res: StkId, id: u32, pc: u32) {
    get_import(L, res, id, pc);
}
