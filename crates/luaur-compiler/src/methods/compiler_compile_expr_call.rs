use crate::functions::get_builtin::get_builtin;
use crate::functions::get_builtin_info::get_builtin_info;
use crate::functions::sref_compiler::sref_ast_name;
use crate::records::builtin_info::BuiltinInfo;
use crate::records::compiler::Compiler;
use crate::records::reg_scope::RegScope;
use luaur_ast::records::ast_expr_call::AstExprCall;
use luaur_ast::records::ast_expr_index_name::AstExprIndexName;
use luaur_ast::records::ast_node::AstNode;
use luaur_bytecode::methods::bytecode_builder_get_string_hash::bytecode_builder_get_string_hash;
use luaur_common::enums::luau_builtin_function::LuauBuiltinFunction;
use luaur_common::enums::luau_bytecode_type::LuauBytecodeType;
use luaur_common::enums::luau_feedback_type::LuauFeedbackType;
use luaur_common::enums::luau_opcode::LuauOpcode;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

impl Compiler {
    pub fn compile_expr_call(
        &mut self,
        expr: *mut AstExprCall,
        target: u8,
        target_count: u8,
        target_top: bool,
        mult_ret: bool,
    ) {
        unsafe {
            let expr_ref = &*expr;
            LUAU_ASSERT!(target_count < 255);
            LUAU_ASSERT!(!target_top || (target as u32 + target_count as u32) == self.reg_top);

            self.set_debug_line_ast_node(expr as *mut _);

            if self.options.optimization_level >= 2 && !expr_ref.self_ {
                let func = self.get_function_expr(expr_ref.func);
                let fi = if !func.is_null() {
                    self.functions.find(&func)
                } else {
                    None
                };

                if fi.map_or(false, |f| f.can_inline)
                    && self.try_compile_inlined_call(
                        expr,
                        func,
                        target,
                        target_count,
                        mult_ret,
                        luaur_common::FInt::LuauCompileInlineThreshold.get(),
                        luaur_common::FInt::LuauCompileInlineThresholdMaxBoost.get(),
                        luaur_common::FInt::LuauCompileInlineDepth.get(),
                    )
                {
                    return;
                }
            }

            let mut rs = self.reg_scope_compiler();
            let reg_count =
                (1 + (expr_ref.self_ as usize) + expr_ref.args.size).max(target_count as usize);
            let regs = if target_top {
                self.alloc_reg(expr as *mut _, (reg_count - target_count as usize) as u32)
                    - target_count
            } else {
                self.alloc_reg(expr as *mut _, reg_count as u32)
            };

            let mut selfreg = 0u8;
            let mut bfid = -1;

            if self.options.optimization_level >= 1 && !expr_ref.self_ {
                // C++ `id && *id != LBF_NONE`: a builtins entry of LBF_NONE (0) means "not a
                // builtin" and must NOT enable FASTCALL. The inline builtin apply/restore
                // (and operator[] lookups) can leave a real LBF_NONE entry, so testing
                // `!= -1` wrongly set bfid = 0 and emitted FASTCALL with builtin 0.
                if let Some(id) = self.builtins.find(&expr) {
                    if *id
                        != luaur_common::enums::luau_builtin_function::LuauBuiltinFunction::LBF_NONE
                            as i32
                    {
                        bfid = *id;
                    }
                }
            }

            if bfid >= 0 && (*self.bytecode).needs_debug_remarks() {
                let builtin = get_builtin(expr_ref.func, &self.globals, &self.variables);
                let last_mult = expr_ref.args.size > 0
                    && self.is_expr_mult_ret(*expr_ref.args.data.add(expr_ref.args.size - 1));
                if !builtin.empty() {
                    // C++ `addDebugRemark("builtin %s.%s/%d%s", object, method, args.size, lastMult?"+":"")`
                    // (the object form) or `"builtin %s/%d%s"` when there is no object.
                    let argc = expr_ref.args.size as i32;
                    let suffix = if last_mult { "+" } else { "" };
                    let method = core::ffi::CStr::from_ptr(builtin.method.value)
                        .to_str()
                        .unwrap_or("");
                    if builtin.object.value.is_null() {
                        (*self.bytecode).add_debug_remark(format_args!(
                            "builtin {}/{}{}",
                            method, argc, suffix
                        ));
                    } else {
                        let object = core::ffi::CStr::from_ptr(builtin.object.value)
                            .to_str()
                            .unwrap_or("");
                        (*self.bytecode).add_debug_remark(format_args!(
                            "builtin {}.{}/{}{}",
                            object, method, argc, suffix
                        ));
                    }
                }
            }

            if bfid == LuauBuiltinFunction::LBF_SELECT_VARARG as i32 {
                // Optimization: compile select(_, ...) as FASTCALL1; only for single-return
                // expressions. Otherwise fall back to a regular call (bfid = -1).
                if !mult_ret && target_count == 1 {
                    return self.compile_expr_select_vararg(
                        expr,
                        target,
                        target_count,
                        target_top,
                        mult_ret,
                        regs,
                    );
                } else {
                    bfid = -1;
                }
            }

            if bfid == LuauBuiltinFunction::LBF_BIT32_EXTRACT as i32
                && expr_ref.args.size == 3
                && self.is_constant(*expr_ref.args.data.add(1))
                && self.is_constant(*expr_ref.args.data.add(2))
            {
                let fc = self.get_constant(*expr_ref.args.data.add(1));
                let wc = self.get_constant(*expr_ref.args.data.add(2));
                let fi = if fc.r#type == crate::enums::type_constant_folding::Type::Type_Number {
                    fc.data.value_number as i32
                } else {
                    -1
                };
                let wi = if wc.r#type == crate::enums::type_constant_folding::Type::Type_Number {
                    wc.data.value_number as i32
                } else {
                    -1
                };
                // Widen the add: `fi`/`wi` are folded user constants and `fi + wi`
                // overflows `int` for huge fields (UB in C++; panic w/ overflow-checks).
                if fi >= 0 && wi > 0 && fi as i64 + wi as i64 <= 32 {
                    let fwp = fi | ((wi - 1) << 5);
                    let cid = (*self.bytecode).add_constant_number(fwp as f64);
                    if cid >= 0 {
                        return self.compile_expr_fastcall_n(
                            expr,
                            target,
                            target_count,
                            target_top,
                            mult_ret,
                            regs,
                            LuauBuiltinFunction::LBF_BIT32_EXTRACTK as i32,
                            cid,
                        );
                    }
                }
            }

            let mut max_fastcall_args = 2;
            if bfid >= 0 && expr_ref.args.size == 3 {
                for i in 0..expr_ref.args.size {
                    if self.get_expr_local_reg(*expr_ref.args.data.add(i)) >= 0 {
                        max_fastcall_args = 3;
                        break;
                    }
                }
            }

            if bfid >= 0 && expr_ref.args.size >= 1 && expr_ref.args.size <= max_fastcall_args {
                if !self.is_expr_mult_ret(*expr_ref.args.data.add(expr_ref.args.size - 1)) {
                    return self.compile_expr_fastcall_n(
                        expr,
                        target,
                        target_count,
                        target_top,
                        mult_ret,
                        regs,
                        bfid,
                        -1,
                    );
                } else if self.options.optimization_level >= 2 {
                    let info = get_builtin_info(bfid);
                    if expr_ref.args.size as i32 == info.params
                        && (info.flags & BuiltinInfo::Flag_NoneSafe) != 0
                    {
                        return self.compile_expr_fastcall_n(
                            expr,
                            target,
                            target_count,
                            target_top,
                            mult_ret,
                            regs,
                            bfid,
                            -1,
                        );
                    }
                }
            }

            if expr_ref.self_ {
                let fi =
                    luaur_ast::rtti::ast_node_as::<AstExprIndexName>(expr_ref.func as *mut AstNode);
                LUAU_ASSERT!(!fi.is_null());
                let reg = self.get_expr_local_reg((*fi).expr);
                if reg >= 0 {
                    selfreg = reg as u8;
                } else {
                    selfreg = regs;
                    self.compile_expr_temp_top((*fi).expr, selfreg);
                }
            } else if bfid < 0 {
                self.compile_expr_temp_top(expr_ref.func, regs);
            }

            let mut mult_call = false;
            for i in 0..expr_ref.args.size {
                let arg = *expr_ref.args.data.add(i);
                if i + 1 == expr_ref.args.size {
                    mult_call = self.compile_expr_temp_mult_ret(
                        arg,
                        regs + 1 + (expr_ref.self_ as u8) + i as u8,
                    );
                } else {
                    self.compile_expr_temp_top(arg, regs + 1 + (expr_ref.self_ as u8) + i as u8);
                }
            }

            self.set_debug_line_end(expr_ref.func as *mut AstNode);

            if expr_ref.self_ {
                let fi =
                    luaur_ast::rtti::ast_node_as::<AstExprIndexName>(expr_ref.func as *mut AstNode);
                self.set_debug_line_location(&(*fi).index_location);
                let iname = sref_ast_name((*fi).index);
                let cid = (*self.bytecode).add_constant_string(iname);
                (*self.bytecode).emit_abc(
                    LuauOpcode::LOP_NAMECALL,
                    regs,
                    selfreg,
                    bytecode_builder_get_string_hash(iname) as u8,
                );
                (*self.bytecode).emit_aux(cid as u32);
                self.hint_temporary_expr_reg_type(
                    (*fi).expr,
                    selfreg as i32,
                    LuauBytecodeType(4),
                    2,
                );
            } else if bfid >= 0 {
                let fastcall_label = (*self.bytecode).emit_label();
                (*self.bytecode).emit_abc(LuauOpcode::LOP_FASTCALL, bfid as u8, 0, 0);
                self.compile_expr_temp(expr_ref.func, regs);
                let call_label = (*self.bytecode).emit_label();
                (*self.bytecode).patch_skip_c(fastcall_label, call_label);
            }

            // C++ `canInline = currentFunction->functionDepth != 0 && !multCall && !multRet`
            // (Compiler.cpp:1394): call feedback is only emitted for calls in *nested* functions,
            // not the depth-0 main chunk. The port had a stray null-check instead of the depth test.
            if luaur_common::FFlag::LuauEmitCallFeedback.get()
                && bfid < 0
                && (*self.current_function).function_depth != 0
                && !mult_call
                && !mult_ret
            {
                let fb_slot = (*self.bytecode).add_fb_slot(LuauFeedbackType::LFT_CALLTARGET);
                (*self.bytecode).emit_abc(
                    LuauOpcode::LOP_CALLFB,
                    regs,
                    if mult_call {
                        0
                    } else {
                        (expr_ref.self_ as u8) + expr_ref.args.size as u8 + 1
                    },
                    if mult_ret { 0 } else { target_count + 1 },
                );
                (*self.bytecode).emit_aux(fb_slot);
            } else {
                (*self.bytecode).emit_abc(
                    LuauOpcode::LOP_CALL,
                    regs,
                    if mult_call {
                        0
                    } else {
                        (expr_ref.self_ as u8) + expr_ref.args.size as u8 + 1
                    },
                    if mult_ret { 0 } else { target_count + 1 },
                );
            }

            if !target_top {
                for i in 0..target_count {
                    (*self.bytecode).emit_abc(LuauOpcode::LOP_MOVE, target + i, regs + i, 0);
                }
            }
        }
    }
}
