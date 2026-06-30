use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_modf"]
pub unsafe fn math_modf(l: *mut lua_State) -> i32 {
    let mut ip: f64 = 0.0;
    let fp = (lua_l_checknumber(l, 1)).modf(&mut ip);
    lua_pushnumber(l, ip);
    lua_pushnumber(l, fp);
    2
}

trait F64Modf {
    fn modf(&self, ip: &mut f64) -> f64;
}

impl F64Modf for f64 {
    fn modf(&self, ip: &mut f64) -> f64 {
        *ip = self.trunc();
        self - *ip
    }
}
