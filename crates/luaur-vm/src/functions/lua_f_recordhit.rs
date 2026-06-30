//! `luaF_recordhit` — record a call-target hit in the caller's feedback vector.
//! C++ source: `VM/src/lfunc.cpp:225`
//!
//! Returns `true` if the inline threshold has not yet been reached for this
//! slot (caller should continue speculation), `false` otherwise.

use crate::enums::feedback_vector_slot_kind::FeedbackVectorSlotKind;
use crate::records::closure::Closure;
use crate::records::lua_state::lua_State;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[export_name = "luaur_luaF_recordhit"]
#[allow(non_snake_case)]
pub unsafe fn luaF_recordhit(
    l: *mut lua_State,
    caller: *mut Closure,
    target: *mut Closure,
    slotid: u32,
) -> bool {
    if (*(*l).global).ecb.inlinefunction.is_none() {
        return false;
    }

    LUAU_ASSERT!((*caller).isC == 0);
    let callerp =
        (*(core::ptr::addr_of!((*caller).inner.l) as *const crate::records::closure::LClosure)).p;

    if (*target).isC != 0 {
        return false;
    }
    let targetp =
        (*(core::ptr::addr_of!((*target).inner.l) as *const crate::records::closure::LClosure)).p;

    LUAU_ASSERT!(slotid < (*callerp).feedbackvecsize);
    let slot = (*callerp).feedbackvec.add(slotid as usize);
    LUAU_ASSERT!((*slot).kind == FeedbackVectorSlotKind::CALL_TARGET);

    if (*slot).data.call_target.proto == 0 {
        (*slot).data.call_target.proto = (*targetp).funid;
    }

    if (*slot).data.call_target.proto != (*targetp).funid {
        return false;
    }

    (*slot).data.call_target.hits += 1;

    if (*slot).data.call_target.hits as i32 >= luaur_common::FInt::LuauInlineHitsThreshold.get() {
        if let Some(inline_fn) = (*(*l).global).ecb.inlinefunction {
            inline_fn(l, caller, target, (*slot).data.call_target.pc);
        }
        return false;
    }

    true
}
