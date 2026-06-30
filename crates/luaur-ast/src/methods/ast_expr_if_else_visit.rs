use crate::records::ast_expr_if_else::AstExprIfElse;
use crate::records::ast_visitor::AstVisitor;
use crate::visit::AstVisitable;

impl AstVisitable for AstExprIfElse {
    fn visit(&self, visitor: &mut dyn AstVisitor) {
        if visitor.visit_expr_if_else(self as *const Self as *mut core::ffi::c_void) {
            unsafe {
                crate::visit::ast_expr_visit(self.condition, visitor);
                crate::visit::ast_expr_visit(self.true_expr, visitor);
                crate::visit::ast_expr_visit(self.false_expr, visitor);
            }
        }
    }
}

#[export_name = "luaur_ast_expr_if_else_visit"]
#[allow(non_snake_case)]
pub unsafe extern "C" fn ast_expr_if_else_visit(
    this: *mut AstExprIfElse,
    visitor: *mut dyn AstVisitor,
) {
    (*this).visit(&mut *visitor);
}
