use crate::records::ast_type_pack_variadic::AstTypePackVariadic;
use crate::records::ast_visitor::AstVisitor;
use crate::visit::AstVisitable;

impl AstVisitable for AstTypePackVariadic {
    fn visit(&self, visitor: &mut dyn AstVisitor) {
        if visitor.visit_type_pack_variadic(self as *const Self as *mut core::ffi::c_void) {
            unsafe {
                crate::visit::ast_type_visit(self.variadic_type, visitor);
            }
        }
    }
}

#[export_name = "luaur_ast_type_pack_variadic_visit"]
pub extern "C" fn ast_type_pack_variadic_visit(
    this: *const AstTypePackVariadic,
    visitor: *mut dyn AstVisitor,
) {
    if this.is_null() {
        return;
    }
    unsafe {
        (*this).visit(&mut *visitor);
    }
}
