use crate::records::ast_expr::AstExpr;
use crate::records::ast_local::AstLocal;
use crate::records::ast_stat_local::AstStatLocal;
use crate::records::ast_visitor::AstVisitor;
use crate::visit::{ast_expr_visit, ast_type_visit, AstVisitable};

impl AstVisitable for AstStatLocal {
    fn visit(&self, visitor: &mut dyn AstVisitor) {
        if visitor.visit_stat_local(self as *const Self as *mut core::ffi::c_void) {
            for var_ptr in self.vars.iter() {
                let var = unsafe { &**var_ptr };
                if !var.annotation.is_null() {
                    unsafe {
                        ast_type_visit(var.annotation, visitor);
                    }
                }
            }

            for expr_ptr in self.values.iter() {
                unsafe {
                    ast_expr_visit(*expr_ptr, visitor);
                }
            }
        }
    }
}

#[export_name = "luaur_ast_stat_local_visit"]
pub extern "C" fn ast_stat_local_visit(this: *mut AstStatLocal, visitor: *mut dyn AstVisitor) {
    unsafe {
        (*this).visit(&mut *visitor);
    }
}
