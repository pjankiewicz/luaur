use crate::records::ast_generic_type::AstGenericType;
use crate::records::ast_generic_type_pack::AstGenericTypePack;
use crate::records::ast_stat_type_alias::AstStatTypeAlias;
use crate::records::ast_visitor::AstVisitor;
use crate::visit::AstVisitable;

impl AstVisitable for AstStatTypeAlias {
    fn visit(&self, visitor: &mut dyn AstVisitor) {
        if visitor.visit_stat_type_alias(self as *const Self as *mut core::ffi::c_void) {
            for &el in self.generics.iter() {
                unsafe {
                    crate::visit::ast_node_visit(
                        el as *mut crate::records::ast_node::AstNode,
                        visitor,
                    );
                }
            }

            for &el in self.generic_packs.iter() {
                unsafe {
                    crate::visit::ast_node_visit(
                        el as *mut crate::records::ast_node::AstNode,
                        visitor,
                    );
                }
            }

            unsafe {
                crate::visit::ast_type_visit(self.type_ptr, visitor);
            }
        }
    }
}

#[export_name = "luaur_ast_stat_type_alias_visit"]
pub extern "C" fn ast_stat_type_alias_visit(
    this: *const AstStatTypeAlias,
    visitor: *mut dyn AstVisitor,
) {
    unsafe {
        (*this).visit(&mut *visitor);
    }
}
