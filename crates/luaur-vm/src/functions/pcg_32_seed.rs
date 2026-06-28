use crate::functions::pcg_32_random::pcg_32_random;

pub fn pcg_32_seed(state: &mut u64, seed: u64) {
    *state = 0;
    pcg_32_random(state);
    // C++ does `*state += seed` on a uint64_t (well-defined unsigned wraparound);
    // a checked `+=` here would panic for seeds that wrap (e.g. math.randomseed(-1)
    // -> seed = 0xFFFF_FFFF_FFFF_FFFF). Match C++ with wrapping_add.
    *state = state.wrapping_add(seed);
    pcg_32_random(state);
}
