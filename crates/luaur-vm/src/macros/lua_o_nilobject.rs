use crate::type_aliases::t_value::TValue;
use crate::type_aliases::value::Value;

/// The shared immutable nil sentinel. Reference: `VM/src/lobject.cpp:16`
/// `const TValue luaO_nilobject_ = {{NULL}, {0}, LUA_TNIL};`
///
/// `TValue` holds raw pointers so it is not `Sync`; the wrapper asserts what
/// the C++ global guarantees — the object is immutable shared data.
#[repr(transparent)]
pub struct NilSentinel(pub TValue);
unsafe impl Sync for NilSentinel {}

#[export_name = "luaur_luaO_nilobject_"]
#[allow(non_upper_case_globals)]
pub static luaO_nilobject_: NilSentinel = NilSentinel(TValue {
    value: Value {
        p: core::ptr::null_mut(),
    },
    extra: [0],
    tt: 0, // LUA_TNIL
});

/// C++ `#define luaO_nilobject (&luaO_nilobject_)`.
#[allow(non_upper_case_globals)]
pub const luaO_nilobject: *const TValue = &luaO_nilobject_.0 as *const TValue;
