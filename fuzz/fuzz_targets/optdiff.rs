// Differential oracle (the doc's "probably your strongest strategy"):
// optimization must PRESERVE observable behavior. The same deterministic program
// is compiled + run at -O0, -O1, and -O2; the observation (captured `print`
// output + final ok/err status) must be identical across all three. A divergence
// is a miscompilation / optimizer bug.
//
// NB: this is OUR OWN compiler's optimization levels — not a differential against
// a C++ reference (explicitly out of scope). Inconclusive runs (didn't compile,
// or hit the step limit — optimization changes step counts) are skipped, so the
// oracle only fires on a genuine behavioral divergence.

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

fn exercise_input(data: &[u8]) {
    let src = luaur_fuzz::generate_computational(data);

    let o0 = luaur_fuzz::run_observed(&src, 0);
    let o1 = luaur_fuzz::run_observed(&src, 1);
    let o2 = luaur_fuzz::run_observed(&src, 2);

    // Only compare when every level produced a conclusive observation.
    if let (Some(a), Some(b), Some(c)) = (o0, o1, o2) {
        assert!(
            a == b && b == c,
            "optimization changed observable behavior:\n  O0 = {a}\n  O1 = {b}\n  O2 = {c}\n--- program ---\n{src}"
        );
    }

    // --- Flag-matrix invariance --------------------------------------------
    // The debug / coverage / type-info compiler levels add metadata and
    // instrumentation; they must NOT change observable behavior. At a fixed opt
    // level, a baseline observation must equal the same program compiled with a
    // bumped debug level, a bumped coverage level, and a bumped type-info level.
    // Coverage instrumentation altering results is a real, embarrassing class of
    // bug this catches. (The first byte varies the fixed opt level so the matrix
    // is checked across O0/O1/O2 over the corpus, not just one level.)
    use luaur_fuzz::ObserveCfg;
    let opt = data.first().copied().unwrap_or(1) % 3;
    let base = ObserveCfg::opt(opt);
    let baseline = luaur_fuzz::run_observed_cfg(&src, base);
    let with_debug = luaur_fuzz::run_observed_cfg(
        &src,
        ObserveCfg {
            debug_level: 2,
            ..base
        },
    );
    let with_cov = luaur_fuzz::run_observed_cfg(
        &src,
        ObserveCfg {
            coverage_level: 2,
            ..base
        },
    );
    let with_ti = luaur_fuzz::run_observed_cfg(
        &src,
        ObserveCfg {
            type_info_level: 2,
            ..base
        },
    );

    if let (Some(b), Some(d), Some(c), Some(t)) = (baseline, with_debug, with_cov, with_ti) {
        assert!(
            b == d && b == c && b == t,
            "a behavior-preserving compiler flag changed observable behavior (opt={opt}):\n  \
             baseline    = {b}\n  debug=2     = {d}\n  coverage=2  = {c}\n  typeinfo=2  = {t}\n\
             --- program ---\n{src}"
        );
    }
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    {
        luaur_fuzz::install_afl_panic_hook();
        fuzz_nohook!(|data: &[u8]| {
            exercise_input(data);
        });
    }
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
