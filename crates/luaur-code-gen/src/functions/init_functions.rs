use crate::records::native_context::NativeContext;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::luau_fast_function::luau_fast_function;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;
use luaur_vm::type_aliases::tms::TMS;

extern "C" {
    #[link_name = "luaur_luaV_lessthan"]
    pub fn luaV_lessthan(L: *mut lua_State, l: *const TValue, r: *const TValue)
        -> core::ffi::c_int;
    #[link_name = "luaur_luaV_lessequal"]
    pub fn luaV_lessequal(
        L: *mut lua_State,
        l: *const TValue,
        r: *const TValue,
    ) -> core::ffi::c_int;
    #[link_name = "luaur_luaV_equalval"]
    pub fn luaV_equalval(
        L: *mut lua_State,
        t1: *const TValue,
        t2: *const TValue,
    ) -> core::ffi::c_int;

    #[link_name = "luaur_luaV_doarithimpl_TM_ADD"]
    pub fn luaV_doarithimpl_TM_ADD(
        L: *mut lua_State,
        ra: StkId,
        rb: *const TValue,
        rc: *const TValue,
    );
    #[link_name = "luaur_luaV_doarithimpl_TM_SUB"]
    pub fn luaV_doarithimpl_TM_SUB(
        L: *mut lua_State,
        ra: StkId,
        rb: *const TValue,
        rc: *const TValue,
    );
    #[link_name = "luaur_luaV_doarithimpl_TM_MUL"]
    pub fn luaV_doarithimpl_TM_MUL(
        L: *mut lua_State,
        ra: StkId,
        rb: *const TValue,
        rc: *const TValue,
    );
    #[link_name = "luaur_luaV_doarithimpl_TM_DIV"]
    pub fn luaV_doarithimpl_TM_DIV(
        L: *mut lua_State,
        ra: StkId,
        rb: *const TValue,
        rc: *const TValue,
    );
    #[link_name = "luaur_luaV_doarithimpl_TM_IDIV"]
    pub fn luaV_doarithimpl_TM_IDIV(
        L: *mut lua_State,
        ra: StkId,
        rb: *const TValue,
        rc: *const TValue,
    );
    #[link_name = "luaur_luaV_doarithimpl_TM_MOD"]
    pub fn luaV_doarithimpl_TM_MOD(
        L: *mut lua_State,
        ra: StkId,
        rb: *const TValue,
        rc: *const TValue,
    );
    #[link_name = "luaur_luaV_doarithimpl_TM_POW"]
    pub fn luaV_doarithimpl_TM_POW(
        L: *mut lua_State,
        ra: StkId,
        rb: *const TValue,
        rc: *const TValue,
    );
    #[link_name = "luaur_luaV_doarithimpl_TM_UNM"]
    pub fn luaV_doarithimpl_TM_UNM(
        L: *mut lua_State,
        ra: StkId,
        rb: *const TValue,
        rc: *const TValue,
    );

    #[link_name = "luaur_luaV_dolen"]
    pub fn luaV_dolen(L: *mut lua_State, ra: StkId, rb: *const TValue);
    #[link_name = "luaur_luaV_gettable"]
    pub fn luaV_gettable(L: *mut lua_State, t: *const TValue, key: *mut TValue, val: StkId);
    #[link_name = "luaur_luaV_settable"]
    pub fn luaV_settable(L: *mut lua_State, t: *const TValue, key: *mut TValue, val: StkId);
    #[link_name = "luaur_luaV_concat"]
    pub fn luaV_concat(L: *mut lua_State, total: core::ffi::c_int, last: core::ffi::c_int);

    #[link_name = "luaur_luaH_getn"]
    pub fn luaH_getn(t: *mut core::ffi::c_void) -> core::ffi::c_int;
    #[link_name = "luaur_luaH_new"]
    pub fn luaH_new(
        L: *mut lua_State,
        narray: core::ffi::c_int,
        lnhash: core::ffi::c_int,
    ) -> *mut core::ffi::c_void;
    #[link_name = "luaur_luaH_clone"]
    pub fn luaH_clone(L: *mut lua_State, tt: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    #[link_name = "luaur_luaH_resizearray"]
    pub fn luaH_resizearray(L: *mut lua_State, t: *mut core::ffi::c_void, nasize: core::ffi::c_int);
    #[link_name = "luaur_luaH_setnum"]
    pub fn luaH_setnum(
        L: *mut lua_State,
        t: *mut core::ffi::c_void,
        key: core::ffi::c_int,
    ) -> *mut TValue;

    #[link_name = "luaur_luaC_barriertable"]
    pub fn luaC_barriertable(
        L: *mut lua_State,
        t: *mut core::ffi::c_void,
        v: *mut core::ffi::c_void,
    );
    #[link_name = "luaur_luaC_barrierf"]
    pub fn luaC_barrierf(L: *mut lua_State, o: *mut core::ffi::c_void, v: *mut core::ffi::c_void);
    #[link_name = "luaur_luaC_barrierback"]
    pub fn luaC_barrierback(
        L: *mut lua_State,
        o: *mut core::ffi::c_void,
        gclist: *mut *mut core::ffi::c_void,
    );
    #[link_name = "luaur_luaC_step"]
    pub fn luaC_step(L: *mut lua_State, assist: bool) -> usize;

    #[link_name = "luaur_luaF_close"]
    pub fn luaF_close(L: *mut lua_State, level: StkId);
    #[link_name = "luaur_luaF_findupval"]
    pub fn luaF_findupval(L: *mut lua_State, level: StkId) -> *mut core::ffi::c_void;
    #[link_name = "luaur_luaF_newLclosure"]
    pub fn luaF_newLclosure(
        L: *mut lua_State,
        nelems: core::ffi::c_int,
        e: *mut core::ffi::c_void,
        p: *mut core::ffi::c_void,
    ) -> *mut core::ffi::c_void;

    #[link_name = "luaur_luaT_gettm"]
    pub fn luaT_gettm(
        events: *mut core::ffi::c_void,
        event: TMS,
        ename: *mut core::ffi::c_void,
    ) -> *const TValue;
    #[link_name = "luaur_luaT_objtypenamestr"]
    pub fn luaT_objtypenamestr(L: *mut lua_State, o: *const TValue) -> *const core::ffi::c_void;

    pub fn exp(x: f64) -> f64;
    pub fn pow(x: f64, y: f64) -> f64;
    pub fn fmod(x: f64, y: f64) -> f64;
    pub fn log(x: f64) -> f64;
    pub fn log2(x: f64) -> f64;
    pub fn log10(x: f64) -> f64;
    pub fn ldexp(x: f64, exp: core::ffi::c_int) -> f64;
    pub fn round(x: f64) -> f64;
    pub fn frexp(x: f64, exp: *mut core::ffi::c_int) -> f64;
    pub fn modf(x: f64, iptr: *mut f64) -> f64;

    pub fn asin(x: f64) -> f64;
    pub fn sin(x: f64) -> f64;
    pub fn sinh(x: f64) -> f64;
    pub fn acos(x: f64) -> f64;
    pub fn cos(x: f64) -> f64;
    pub fn cosh(x: f64) -> f64;
    pub fn atan(x: f64) -> f64;
    pub fn atan2(y: f64, x: f64) -> f64;
    pub fn tan(x: f64) -> f64;
    pub fn tanh(x: f64) -> f64;

    #[link_name = "luaur_forgLoopTableIter"]
    pub fn forgLoopTableIter(
        L: *mut lua_State,
        h: *mut core::ffi::c_void,
        index: core::ffi::c_int,
        ra: *mut TValue,
    ) -> bool;
    #[link_name = "luaur_forgLoopNodeIter"]
    pub fn forgLoopNodeIter(
        L: *mut lua_State,
        h: *mut core::ffi::c_void,
        index: core::ffi::c_int,
        ra: *mut TValue,
    ) -> bool;
    #[link_name = "luaur_forgLoopNonTableFallback"]
    pub fn forgLoopNonTableFallback(
        L: *mut lua_State,
        insnA: core::ffi::c_int,
        aux: core::ffi::c_int,
    ) -> core::ffi::c_int;
    #[link_name = "luaur_forgLoopNonTableFallback_DEPRECATED"]
    pub fn forgLoopNonTableFallback_DEPRECATED(
        L: *mut lua_State,
        insnA: core::ffi::c_int,
        aux: core::ffi::c_int,
    ) -> bool;
    #[link_name = "luaur_forgPrepXnextFallback"]
    pub fn forgPrepXnextFallback(L: *mut lua_State, ra: *mut TValue, pc: core::ffi::c_int);
    #[link_name = "luaur_callProlog"]
    pub fn callProlog(
        L: *mut lua_State,
        ra: *mut TValue,
        argtop: StkId,
        nresults: core::ffi::c_int,
    ) -> *mut core::ffi::c_void;
    #[link_name = "luaur_callEpilogC"]
    pub fn callEpilogC(L: *mut lua_State, nresults: core::ffi::c_int, n: core::ffi::c_int);
    #[link_name = "luaur_newUserdata"]
    pub fn newUserdata(
        L: *mut lua_State,
        s: usize,
        tag: core::ffi::c_int,
    ) -> *mut core::ffi::c_void;
    #[link_name = "luaur_getImport"]
    pub fn getImport(L: *mut lua_State, res: StkId, id: core::ffi::c_uint, pc: core::ffi::c_uint);

    #[link_name = "luaur_callFallback"]
    pub fn callFallback(
        L: *mut lua_State,
        ra: StkId,
        argtop: StkId,
        nresults: core::ffi::c_int,
    ) -> *mut core::ffi::c_void;

    #[link_name = "luaur_executeGETGLOBAL"]
    pub fn executeGETGLOBAL(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;
    #[link_name = "luaur_executeSETGLOBAL"]
    pub fn executeSETGLOBAL(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;
    #[link_name = "luaur_executeGETTABLEKS"]
    pub fn executeGETTABLEKS(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;
    #[link_name = "luaur_executeSETTABLEKS"]
    pub fn executeSETTABLEKS(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;
    #[link_name = "luaur_executeNAMECALL"]
    pub fn executeNAMECALL(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;
    #[link_name = "luaur_executeFORGPREP"]
    pub fn executeFORGPREP(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;
    #[link_name = "luaur_executeGETVARARGSMultRet"]
    pub fn executeGETVARARGSMultRet(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        rai: core::ffi::c_int,
    );
    #[link_name = "luaur_executeGETVARARGSConst"]
    pub fn executeGETVARARGSConst(
        L: *mut lua_State,
        base: StkId,
        rai: core::ffi::c_int,
        b: core::ffi::c_int,
    );
    #[link_name = "luaur_executeDUPCLOSURE"]
    pub fn executeDUPCLOSURE(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;
    #[link_name = "luaur_executePREPVARARGS"]
    pub fn executePREPVARARGS(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;
    #[link_name = "luaur_executeSETLIST"]
    pub fn executeSETLIST(
        L: *mut lua_State,
        pc: *const u32,
        base: StkId,
        k: *mut TValue,
    ) -> *const u32;

    #[link_name = "luaur_luauF_table"]
    pub static luauF_table: [luau_fast_function; 256];
}

pub fn init_functions(context: &mut NativeContext) {
    unsafe {
        core::ptr::copy_nonoverlapping(luauF_table.as_ptr(), context.luauF_table.as_mut_ptr(), 256);

        context.luaV_lessthan = Some(luaV_lessthan);
        context.luaV_lessequal = Some(luaV_lessequal);
        context.luaV_equalval = Some(luaV_equalval);

        context.luaV_doarithadd = Some(luaV_doarithimpl_TM_ADD);
        context.luaV_doarithsub = Some(luaV_doarithimpl_TM_SUB);
        context.luaV_doarithmul = Some(luaV_doarithimpl_TM_MUL);
        context.luaV_doarithdiv = Some(luaV_doarithimpl_TM_DIV);
        context.luaV_doarithidiv = Some(luaV_doarithimpl_TM_IDIV);
        context.luaV_doarithmod = Some(luaV_doarithimpl_TM_MOD);
        context.luaV_doarithpow = Some(luaV_doarithimpl_TM_POW);
        context.luaV_doarithunm = Some(luaV_doarithimpl_TM_UNM);

        context.luaV_dolen = Some(luaV_dolen);
        context.luaV_gettable = Some(luaV_gettable);
        context.luaV_settable = Some(luaV_settable);
        context.luaV_concat = Some(luaV_concat);

        context.luaH_getn = Some(luaH_getn);
        context.luaH_new = Some(luaH_new);
        context.luaH_clone = Some(luaH_clone);
        context.luaH_resizearray = Some(luaH_resizearray);
        context.luaH_setnum = Some(luaH_setnum);

        context.luaC_barriertable = Some(luaC_barriertable);
        context.luaC_barrierf = Some(luaC_barrierf);
        context.luaC_barrierback = Some(luaC_barrierback);
        context.luaC_step = Some(luaC_step);

        context.luaF_close = Some(luaF_close);
        context.luaF_findupval = Some(luaF_findupval);
        context.luaF_newLclosure = Some(luaF_newLclosure);

        context.luaT_gettm = Some(luaT_gettm);
        context.luaT_objtypenamestr = Some(luaT_objtypenamestr);

        context.libm_exp = Some(exp);
        context.libm_pow = Some(pow);
        context.libm_fmod = Some(fmod);
        context.libm_log = Some(log);
        context.libm_log2 = Some(log2);
        context.libm_log10 = Some(log10);
        context.libm_ldexp = Some(ldexp);
        context.libm_round = Some(round);
        context.libm_frexp = Some(frexp);
        context.libm_modf = Some(modf);

        context.libm_asin = Some(asin);
        context.libm_sin = Some(sin);
        context.libm_sinh = Some(sinh);
        context.libm_acos = Some(acos);
        context.libm_cos = Some(cos);
        context.libm_cosh = Some(cosh);
        context.libm_atan = Some(atan);
        context.libm_atan2 = Some(atan2);
        context.libm_tan = Some(tan);
        context.libm_tanh = Some(tanh);

        context.forgLoopTableIter = Some(forgLoopTableIter);
        context.forgLoopNodeIter = Some(forgLoopNodeIter);
        context.forgLoopNonTableFallback = Some(forgLoopNonTableFallback);
        context.forgLoopNonTableFallback_DEPRECATED = Some(forgLoopNonTableFallback_DEPRECATED);
        context.forgPrepXnextFallback = Some(forgPrepXnextFallback);
        context.callProlog = Some(callProlog);
        context.callEpilogC = Some(callEpilogC);
        context.newUserdata = Some(newUserdata);
        context.getImport = Some(getImport);

        context.callFallback = Some(callFallback);

        context.executeGETGLOBAL = Some(executeGETGLOBAL);
        context.executeSETGLOBAL = Some(executeSETGLOBAL);
        context.executeGETTABLEKS = Some(executeGETTABLEKS);
        context.executeSETTABLEKS = Some(executeSETTABLEKS);

        context.executeNAMECALL = Some(executeNAMECALL);
        context.executeFORGPREP = Some(executeFORGPREP);
        context.executeGETVARARGSMultRet = Some(executeGETVARARGSMultRet);
        context.executeGETVARARGSConst = Some(executeGETVARARGSConst);
        context.executeDUPCLOSURE = Some(executeDUPCLOSURE);
        context.executePREPVARARGS = Some(executePREPVARARGS);
        context.executeSETLIST = Some(executeSETLIST);
    }
}
