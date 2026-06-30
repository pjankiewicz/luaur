use crate::records::ast_expr::AstExpr;
use crate::records::ast_local::AstLocal;
use crate::records::ast_stat_for_in::AstStatForIn;
use crate::records::ast_visitor::AstVisitor;
use crate::visit::AstVisitable;

impl AstVisitable for AstStatForIn {
    fn visit(&self, visitor: &mut dyn AstVisitor) {
        if visitor.visit_stat_for_in(self as *const Self as *mut core::ffi::c_void) {
            for i in 0..self.vars.size {
                unsafe {
                    let var = *self.vars.data.add(i);
                    if !var.is_null() && !(*var).annotation.is_null() {
                        crate::visit::ast_type_visit((*var).annotation, visitor);
                    }
                }
            }

            for i in 0..self.values.size {
                unsafe {
                    let expr = *self.values.data.add(i);
                    crate::visit::ast_expr_visit(expr, visitor);
                }
            }

            unsafe {
                crate::visit::ast_stat_visit(self.body as *mut _, visitor);
            }
        }
    }
}

#[export_name = "luaur_ast_stat_for_in_visit"]
pub extern "C" fn ast_stat_for_in_visit(this: *const AstStatForIn, visitor: *mut dyn AstVisitor) {
    unsafe {
        (*this).visit(&mut *visitor);
    }
}
