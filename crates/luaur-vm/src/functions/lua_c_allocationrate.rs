use crate::functions::lua_clock::lua_clock;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_luaC_allocationrate"]
pub unsafe fn luaC_allocationrate(l: *mut lua_State) -> i64 {
    let g = (*l).global;
    let duration_threshold: f64 = 1e-3; // avoid measuring intervals smaller than 1ms

    const GCS_ATOMIC: u8 = 3;

    if (*g).gcstate <= GCS_ATOMIC {
        let duration = lua_clock() - (*g).gcstats.endtimestamp;

        if duration < duration_threshold {
            return -1;
        }

        return (((*g).totalbytes as f64 - (*g).gcstats.endtotalsizebytes as f64) / duration)
            as i64;
    }

    // totalbytes is unstable during the sweep, use the rate measured at the end of mark phase
    let duration = (*g).gcstats.atomicstarttimestamp - (*g).gcstats.endtimestamp;

    if duration < duration_threshold {
        return -1;
    }

    return (((*g).gcstats.atomicstarttotalsizebytes as f64 - (*g).gcstats.endtotalsizebytes as f64)
        / duration) as i64;
}
