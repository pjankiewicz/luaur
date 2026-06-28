use crate::enums::lua_type::lua_Type;
use crate::macros::luai_num_2_unsigned::luai_num2unsigned;
use crate::macros::nvalue::nvalue;
use crate::macros::setnvalue::setnvalue;
use crate::macros::ttisnumber::ttisnumber;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;

#[allow(non_snake_case)]
pub unsafe fn luau_f_replace(
    _l: *mut lua_State,
    res: StkId,
    arg0: *mut TValue,
    nresults: core::ffi::c_int,
    args: StkId,
    nparams: core::ffi::c_int,
) -> core::ffi::c_int {
    if nparams >= 3
        && nresults <= 1
        && ttisnumber!(arg0)
        && ttisnumber!(args)
        && ttisnumber!(args.offset(1))
    {
        let a1 = nvalue!(arg0);
        let a2 = nvalue!(args);
        let a3 = nvalue!(args.offset(1));

        let mut n: u32 = 0;
        luai_num2unsigned(&mut n, a1);

        let mut v: u32 = 0;
        luai_num2unsigned(&mut v, a2);

        let f: i32 = a3 as i32;

        if nparams == 3 {
            if (f as u32) < 32 {
                let m: u32 = 1;
                let r: u32 = (n & !(m << (f as u32))) | ((v & m) << (f as u32));
                setnvalue!(res, r as f64);
                return 1;
            }
        } else if ttisnumber!(args.offset(2)) {
            let a4 = nvalue!(args.offset(2));
            let w: i32 = a4 as i32;

            if f >= 0 && w > 0 && f as i64 + w as i64 <= 32 {
                let m: u32 = !(0xFFFF_FFFE_u32 << (w - 1));
                let r: u32 = (n & !(m << (f as u32))) | ((v & m) << (f as u32));
                setnvalue!(res, r as f64);
                return 1;
            }
        }
    }

    -1
}
