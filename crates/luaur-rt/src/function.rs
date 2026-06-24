//! The [`Function`] handle. Mirrors `mlua::Function`.

use std::rc::Rc;

use crate::error::Result;
use crate::ffi::*;
use crate::multi::MultiValue;
use crate::state::{Lua, LuaRef};
use crate::traits::{FromLuaMulti, IntoLuaMulti};

/// A handle to a callable Lua value (a Lua closure or a Rust function).
///
/// Mirrors `mlua::Function`.
#[derive(Clone)]
pub struct Function {
    pub(crate) reference: Rc<LuaRef>,
}

impl Function {
    pub(crate) fn from_ref(reference: LuaRef) -> Function {
        Function {
            reference: Rc::new(reference),
        }
    }

    pub(crate) unsafe fn push_to_stack(&self) {
        self.reference.push();
    }

    /// The owning [`Lua`].
    pub fn lua(&self) -> Lua {
        self.reference.lua()
    }

    /// Call the function with `args`, converting the results to `R`.
    ///
    /// Mirrors `mlua::Function::call`. Runs under `lua_pcall`, so a Lua runtime
    /// error (or a Rust callback returning `Err`) becomes `Err(Error)` rather
    /// than unwinding.
    pub fn call<R: FromLuaMulti>(&self, args: impl IntoLuaMulti) -> Result<R> {
        let lua = self.lua();
        let state = lua.state();
        let args: MultiValue = args.into_lua_multi(&lua)?;

        unsafe {
            let base = lua_gettop(state);
            let nargs = args.len() as c_int;
            // Guard against pushing more values than the Lua stack can hold:
            // an unprotected overflow would abort the VM. We need room for the
            // function + all arguments (+1 slack for the call machinery).
            if lua_checkstack(state, nargs.saturating_add(2)) == 0 {
                return Err(crate::error::Error::RuntimeError(
                    "stack overflow: too many arguments to function call".to_string(),
                ));
            }
            // Push the function, then the arguments.
            self.reference.push();
            for v in args.iter() {
                lua.push_value(v)?;
            }
            // LUA_MULTRET == -1: keep every result.
            let status = lua_pcall(state, nargs, -1, 0);
            if status != 0 {
                return Err(lua.pop_error(status));
            }
            // Collect every value pushed above `base` as the results.
            let top = lua_gettop(state);
            let nresults = top - base;
            let mut results = MultiValue::with_capacity(nresults.max(0) as usize);
            for i in 0..nresults {
                let idx = base + 1 + i;
                results.push_back(lua.value_from_stack(idx)?);
            }
            lua_settop(state, base);
            R::from_lua_multi(results, &lua)
        }
    }

    /// Return a new function that, when called, prepends `args` to its own
    /// arguments and forwards to `self`.
    ///
    /// Mirrors `mlua::Function::bind`. Implemented as a Rust closure that
    /// captures the bound prefix and the target function.
    pub fn bind(&self, args: impl IntoLuaMulti) -> Result<Function> {
        let lua = self.lua();
        let bound: MultiValue = args.into_lua_multi(&lua)?;
        let target = self.clone();
        let bound_vec: Vec<crate::value::Value> = bound.into_vec();
        lua.create_function(move |_, extra: MultiValue| {
            let mut all = MultiValue::with_capacity(bound_vec.len() + extra.len());
            for v in &bound_vec {
                all.push_back(v.clone());
            }
            for v in extra {
                all.push_back(v);
            }
            target.call::<MultiValue>(all)
        })
    }

    /// A raw pointer identifying this function (for identity comparison).
    /// Mirrors `mlua::Function::to_pointer`.
    pub fn to_pointer(&self) -> *const std::ffi::c_void {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let p = lua_topointer(state, -1);
            lua_pop(state, 1);
            p
        }
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Function")
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        // Pointer identity (matches mlua): same underlying function object.
        self.to_pointer() == other.to_pointer()
    }
}
