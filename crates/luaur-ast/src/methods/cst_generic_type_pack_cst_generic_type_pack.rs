use crate::records::cst_generic_type_pack::CstGenericTypePack;
use crate::records::cst_node::CstNode;
use crate::records::position::Position;
use crate::rtti::CstNodeClass;

impl CstGenericTypePack {
    pub fn new(ellipsis_position: Position, default_equals_position: Position) -> Self {
        Self {
            base: CstNode {
                class_index: <Self as CstNodeClass>::CLASS_INDEX,
            },
            ellipsis_position: ellipsis_position,
            default_equals_position: default_equals_position,
        }
    }
}

#[export_name = "luaur_cst_generic_type_pack_cst_generic_type_pack"]
pub extern "C" fn cst_generic_type_pack_cst_generic_type_pack(
    ellipsis_position: Position,
    default_equals_position: Position,
) -> CstGenericTypePack {
    CstGenericTypePack::new(ellipsis_position, default_equals_position)
}
