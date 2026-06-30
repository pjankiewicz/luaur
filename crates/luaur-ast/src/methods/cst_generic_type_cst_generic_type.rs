use crate::records::cst_generic_type::CstGenericType;
use crate::records::cst_node::CstNode;
use crate::records::position::Position;
use crate::rtti::CstNodeClass;

impl CstGenericType {
    pub fn new(default_equals_position: Position) -> Self {
        Self {
            base: CstNode {
                class_index: <Self as CstNodeClass>::CLASS_INDEX,
            },
            default_equals_position,
        }
    }
}

#[export_name = "luaur_cst_generic_type_cst_generic_type"]
pub extern "C" fn cst_generic_type_cst_generic_type(
    default_equals_position: Position,
) -> CstGenericType {
    CstGenericType::new(default_equals_position)
}
