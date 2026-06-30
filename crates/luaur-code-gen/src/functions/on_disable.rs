//! Node: `cxx:Function:Luau.CodeGen:CodeGen/src/CodeGen.cpp:100:on_disable`
//!
//! Disable native code for a proto: point its entry back at bytecode, clear the
//! exec target, and walk every thread's Lua call stack clearing the
//! `LUA_CALLINFO_NATIVE` flag on any frame still pointing at this proto.

use core::ffi::c_void;
use luaur_vm::enums::lua_type::lua_Type;
use luaur_vm::functions::lua_m_visitgco::lua_m_visitgco;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::gco_2_th::gco2th;
use luaur_vm::macros::is_lua::isLua;
use luaur_vm::macros::lua_callinfo_native::LUA_CALLINFO_NATIVE;
use luaur_vm::records::gc_object::GCObject;
use luaur_vm::records::lua_page::lua_Page;
use luaur_vm::records::proto::Proto;
use luaur_vm::type_aliases::lua_state::lua_State;

unsafe fn on_disable_visitor(
    context: *mut c_void,
    _page: *mut lua_Page,
    gco: *mut GCObject,
) -> bool {
    let proto = context as *mut Proto;

    if (*gco).gch.tt as i32 != lua_Type::LUA_TTHREAD as i32 {
        return false;
    }

    let th = gco2th!(gco);

    let mut ci = (*th).ci;
    while ci > (*th).base_ci {
        if isLua!(ci) {
            let f = &*clvalue!((*ci).func);
            let p = f.inner.l.p;

            if p == proto {
                (*ci).flags &= !(LUA_CALLINFO_NATIVE as u32);
            }
        }
        ci = ci.sub(1);
    }

    false
}

pub unsafe fn on_disable(L: *mut lua_State, proto: *mut Proto) {
    // do nothing if proto already uses bytecode
    if (*proto).codeentry == (*proto).code as *const _ {
        return;
    }

    // ensure that VM does not call native code for this proto
    (*proto).codeentry = (*proto).code as *const _;

    // prevent native code from entering proto with breakpoints
    (*proto).exectarget = 0;

    lua_m_visitgco(L, proto as *mut c_void, on_disable_visitor as *mut c_void);
}

#[export_name = "luaur_on_disable"]
pub unsafe extern "C" fn on_disable_export(L: *mut lua_State, proto: *mut Proto) {
    on_disable(L, proto);
}
