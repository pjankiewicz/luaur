use crate::records::compile_error::CompileError;
use crate::records::compiler::Compiler;
use crate::records::reg_scope::RegScope;
use luaur_ast::records::ast_expr_constant_integer::AstExprConstantInteger;
use luaur_ast::records::ast_expr_unary::AstExprUnary;
use luaur_ast::rtti;

impl Compiler {
    pub fn compile_expr_unary(&mut self, expr: *mut AstExprUnary, target: u8) {
        unsafe {
            let expr_ref = &*expr;
            let mut rs = self.reg_scope_compiler();

            if luaur_common::FFlag::LuauIntegerType2.get()
                && expr_ref.op == luaur_ast::records::ast_expr_unary::AstExprUnaryOp::Minus
            {
                let cint = rtti::ast_node_as::<AstExprConstantInteger>(expr_ref.expr as *mut _);
                if !cint.is_null() {
                    // Two's-complement negation `~v + 1 == -v`. The `+ 1` MUST
                    // wrap (C semantics): for v == 0 it wraps `u64::MAX -> 0`, and
                    // for v == i64::MIN it wraps back to i64::MIN — both correct.
                    // A checked `+` panics on those under the fuzz build's
                    // overflow-checks (found by the compile fuzzer on `-<int 0>`).
                    let cid = (*self.bytecode)
                        .add_constant_integer((!((*cint).value as u64)).wrapping_add(1) as i64);
                    if cid < 0 {
                        CompileError::raise(
                            &expr_ref.base.base.location,
                            format_args!("Exceeded constant limit; simplify the code to compile"),
                        );
                    }
                    self.emit_load_k(target, cid);
                    return;
                }
            }

            let re = self.compile_expr_auto(expr_ref.expr, &mut rs);
            (*self.bytecode).emit_abc(self.get_unary_op(expr_ref.op), target, re, 0);
        }
    }
}
