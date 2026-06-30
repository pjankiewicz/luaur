use crate::functions::finish_gc_cycle_metrics::finish_gc_cycle_metrics;
use crate::functions::gcstep::gcstep;
use crate::functions::getheaptrigger::getheaptrigger;
use crate::functions::lua_clock::lua_clock;
use crate::functions::record_gc_state_step::record_gc_state_step;
use crate::functions::start_gc_cycle_metrics::start_gc_cycle_metrics;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[inline]
unsafe fn gc_interrupt(l: *mut lua_State, state: c_int) {
    let g = &*(*l).global;
    if let Some(interrupt) = g.cb.interrupt {
        interrupt(l, state);
    }
}

#[allow(non_snake_case)]
pub unsafe fn luaC_step(l: *mut lua_State, assist: bool) -> usize {
    let g = (*l).global;

    let lim = ((*g).gcstepsize as usize * (*g).gcstepmul as usize) / 100;
    LUAU_ASSERT!((*g).totalbytes >= (*g).GCthreshold);
    let debt = (*g).totalbytes - (*g).GCthreshold;

    gc_interrupt(l, 0);

    if (*g).gcstate == 0 {
        (*g).gcstats.starttimestamp = lua_clock();
    }

    #[cfg(feature = "luai_gcmetrics")]
    {
        if (*g).gcstate == 0 {
            start_gc_cycle_metrics(g);
        }
        let _lasttimestamp = lua_clock();
    }

    let lastgcstate = (*g).gcstate as i32;

    let work = gcstep(l, lim);

    #[cfg(feature = "luai_gcmetrics")]
    {
        record_gc_state_step(g, lastgcstate, lua_clock() - _lasttimestamp, assist, work);
    }

    let actualstepsize = (work * 100) / (*g).gcstepmul as usize;

    if (*g).gcstate == 0 {
        let heapgoal = ((*g).totalbytes / 100) * (*g).gcgoal as usize;
        let heaptrigger = getheaptrigger(g, heapgoal);

        (*g).GCthreshold = heaptrigger;

        (*g).gcstats.heapgoalsizebytes = heapgoal;
        (*g).gcstats.endtimestamp = lua_clock();
        (*g).gcstats.endtotalsizebytes = (*g).totalbytes;

        #[cfg(feature = "luai_gcmetrics")]
        {
            finish_gc_cycle_metrics(g);
        }
    } else {
        (*g).GCthreshold = (*g).totalbytes + actualstepsize;

        if (*g).GCthreshold >= debt {
            (*g).GCthreshold -= debt;
        }
    }

    gc_interrupt(l, lastgcstate);

    actualstepsize
}

#[export_name = "luaur_luaC_step"]
pub unsafe extern "C" fn lua_c_step_export(l: *mut lua_State, assist: bool) -> usize {
    luaC_step(l, assist)
}
