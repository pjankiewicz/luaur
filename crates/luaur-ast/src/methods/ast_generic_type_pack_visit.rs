use crate::records::ast_generic_type_pack::AstGenericTypePack;
use crate::records::ast_visitor::AstVisitor;
use crate::visit::AstVisitable;

impl AstVisitable for AstGenericTypePack {
    fn visit(&self, visitor: &mut dyn AstVisitor) {
        if visitor.visit_generic_type_pack(self as *const Self as *mut core::ffi::c_void) {
            if !self.default_value.is_null() {
                unsafe {
                    crate::visit::ast_type_pack_visit(self.default_value, visitor);
                }
            }
        }
    }
}

#[export_name = "luaur_ast_generic_type_pack_visit"]
pub extern "C" fn ast_generic_type_pack_visit(
    this: *const AstGenericTypePack,
    visitor: *mut dyn AstVisitor,
) {
    unsafe {
        (*this).visit(&mut *visitor);
    }
}
