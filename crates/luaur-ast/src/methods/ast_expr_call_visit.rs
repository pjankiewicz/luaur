use crate::records::ast_expr_call::AstExprCall;
use crate::records::ast_visitor::AstVisitor;
use crate::visit::AstVisitable;

impl AstVisitable for AstExprCall {
    fn visit(&self, visitor: &mut dyn AstVisitor) {
        if visitor.visit_expr_call(self as *const Self as *mut core::ffi::c_void) {
            unsafe {
                crate::visit::ast_expr_visit(self.func, visitor);

                for &arg in self.args.iter() {
                    crate::visit::ast_expr_visit(arg, visitor);
                }
            }
        }
    }
}

#[export_name = "luaur_ast_expr_call_visit"]
pub extern "C" fn ast_expr_call_visit(this: *const AstExprCall, visitor: *mut dyn AstVisitor) {
    unsafe {
        (*this).visit(&mut *visitor);
    }
}
