use crate::functions::follow_type::follow_type_id;
use crate::functions::get_type_alt_j::get_type_id;
use crate::records::metatable_type::MetatableType;
use crate::records::table_type::TableType;
use crate::type_aliases::type_id::TypeId;
use alloc::string::{String, ToString};

/// C++ `getName(TypeId)` returns a `std::optional<Name>` (an owned string), not a
/// borrow. The previous port returned `&'static str` via `Box::leak`, leaking a
/// string on every call (the checker resolves names while stringifying errors) —
/// caught by the fuzz suite's LeakSanitizer. Return an owned `String`; the sole
/// caller only compares it.
pub fn get_name(type_id: TypeId) -> Option<String> {
    let mut ty = unsafe { follow_type_id(type_id) };

    let mtv_ptr = unsafe { get_type_id::<MetatableType>(ty) };
    if !mtv_ptr.is_null() {
        let mtv = unsafe { &*mtv_ptr };
        if let Some(name) = mtv.syntheticName() {
            return Some(name.to_string());
        }
        ty = unsafe { follow_type_id(mtv.table()) };
    }

    let ttv_ptr = unsafe { get_type_id::<TableType>(ty) };
    if !ttv_ptr.is_null() {
        let ttv = unsafe { &*ttv_ptr };
        if let Some(name) = &ttv.name {
            return Some(name.clone());
        }
        if let Some(name) = &ttv.synthetic_name {
            return Some(name.clone());
        }
    }

    None
}
