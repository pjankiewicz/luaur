use crate::enums::value::Value;
use crate::functions::flatten_type_pack::flatten_type_pack_id;
use crate::functions::follow_type::follow_type_id;
use crate::functions::get_type_alt_j::get_type_id;
use crate::functions::parse_format_string::parse_format_string;
use crate::functions::should_suppress_errors_type_utils::should_suppress_errors;
use crate::functions::unwrap_group::unwrap_group;
use crate::records::cannot_check_dynamic_string_format_calls::CannotCheckDynamicStringFormatCalls;
use crate::records::count_mismatch::{CountMismatch, CountMismatchContext};
use crate::records::error_suppression::ErrorSuppression;
use crate::records::magic_function_type_check_context::MagicFunctionTypeCheckContext;
use crate::records::singleton_type::SingletonType;
use crate::records::string_singleton::StringSingleton;
use crate::records::type_mismatch::TypeMismatch;
use crate::type_aliases::type_error_data::TypeErrorData;
use crate::type_aliases::type_id::TypeId;
use alloc::string::ToString;
use luaur_ast::records::ast_expr_constant_string::AstExprConstantString;
use luaur_ast::records::ast_expr_index_name::AstExprIndexName;
use luaur_ast::records::ast_node::AstNode;
use luaur_ast::rtti::ast_node_as;
use luaur_common::FFlag;

pub fn magic_format_type_check(context: &MagicFunctionTypeCheckContext) -> bool {
    let typechecker = unsafe { &mut *context.typechecker.as_ptr() };
    let call_site = unsafe { &*context.call_site };

    let iter = unsafe { crate::functions::begin_type_pack::begin(context.arguments) };
    let end_iter = unsafe { crate::functions::end_type_pack::end(context.arguments) };

    if iter.operator_eq(&end_iter) {
        typechecker.report_error_type_error_data_location(
            TypeErrorData::CountMismatch(CountMismatch {
                expected: 1,
                maximum: None,
                actual: 0,
                context: CountMismatchContext::Arg,
                is_variadic: true,
                function: "string.format".to_string(),
            }),
            &call_site.base.base.location,
        );
        return true;
    }

    // we'll suppress any errors for `string.format` if the format string is error suppressing.
    if should_suppress_errors(&mut typechecker.normalizer as *mut _, unsafe {
        follow_type_id(*iter.operator_deref())
    }) == ErrorSuppression::from_value(Value::Suppress)
    {
        return true;
    }

    let mut fmt: *mut AstExprConstantString = core::ptr::null_mut();
    if !call_site.func.is_null() {
        let func_node = unsafe { &*call_site.func };
        if func_node.base.class_index == AstExprIndexName::ClassIndex {
            let index_expr =
                unsafe { ast_node_as::<AstExprIndexName>(call_site.func as *mut AstNode) };
            if !index_expr.is_null() && call_site.self_ {
                let unwrapped = unwrap_group(unsafe { &mut *index_expr }.expr);
                fmt = unsafe { ast_node_as::<AstExprConstantString>(unwrapped as *mut AstNode) };
            }
        }
    }

    if !call_site.self_ && call_site.args.size > 0 {
        fmt = unsafe { ast_node_as::<AstExprConstantString>(*call_site.args.data as *mut AstNode) };
    }

    let mut format_string: Option<&str> = None;
    if !fmt.is_null() {
        let fmt_ref = unsafe { &*fmt };
        let data = fmt_ref.value.data as *const u8;
        let size = fmt_ref.value.size as usize;
        format_string = Some(unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(data, size))
        });
    } else {
        let first_arg = unsafe { *iter.operator_deref() };
        let followed = unsafe { follow_type_id(first_arg) };
        let singleton = unsafe { get_type_id::<SingletonType>(followed) };
        if !singleton.is_null() {
            if let Some(string_singleton) =
                unsafe { (*singleton).variant.get_if::<StringSingleton>() }
            {
                format_string = Some(&string_singleton.value);
            }
        }
    }

    if FFlag::LuauSilenceDynamicFormatStringErrors.get() {
        if format_string.is_none() {
            return true;
        }
    } else if format_string.is_none() {
        typechecker.report_error_type_error_data_location(
            TypeErrorData::CannotCheckDynamicStringFormatCalls(
                CannotCheckDynamicStringFormatCalls::default(),
            ),
            &call_site.base.base.location,
        );
        return true;
    }

    // CLI-150726: The block below effectively constructs a type pack and then type checks it by going parameter-by-parameter.
    let format_str = format_string.unwrap();
    let expected = parse_format_string(
        context.builtin_types,
        format_str.as_ptr() as *const core::ffi::c_char,
        format_str.len(),
    );

    let (params, _tail) = flatten_type_pack_id(context.arguments);

    let param_offset = 1;
    // Compare the expressions passed with the types the function expects to determine whether this function was called with : or .
    let called_with_self = expected.len() == call_site.args.size as usize;
    // unify the prefix one argument at a time
    for i in 0..expected.len() {
        if i + param_offset >= params.len() {
            break;
        }
        // No argument expressions ⇒ nothing to attach a location to, and
        // `args.size - 1` would underflow.
        if call_site.args.size == 0 {
            break;
        }
        let actual_ty: TypeId = params[i + param_offset];
        let expected_ty: TypeId = expected[i];
        let arg_index = core::cmp::min(
            (call_site.args.size as usize) - 1,
            i + if called_with_self { 0 } else { param_offset },
        );
        let location = unsafe { (*(*call_site.args.data.add(arg_index))).base.location };
        // use subtyping instead here
        let scope_ptr = context.check_scope.as_ptr();
        let result = unsafe {
            (*typechecker.subtyping).is_subtype_type_id_type_id_not_null_scope(
                actual_ty,
                expected_ty,
                scope_ptr,
            )
        };

        if !result.is_subtype {
            match should_suppress_errors(&mut typechecker.normalizer as *mut _, actual_ty).value {
                Value::Suppress => {}
                Value::NormalizationFailed => {}
                Value::DoNotSuppress => {
                    let mut reasonings = typechecker
                        .explain_reasonings_type_id_type_id_location_subtyping_result(
                            actual_ty,
                            expected_ty,
                            location,
                            &result,
                        );

                    if !reasonings.suppressed {
                        let reason = reasonings.to_string();
                        typechecker.report_error_type_error_data_location(
                            TypeErrorData::TypeMismatch(TypeMismatch::from_wanted_given_reason(
                                expected_ty,
                                actual_ty,
                                reason,
                            )),
                            &location,
                        );
                    }
                }
            }
        }
    }

    true
}
