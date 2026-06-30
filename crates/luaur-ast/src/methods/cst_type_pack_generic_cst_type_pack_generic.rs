use crate::records::cst_node::CstNode;
use crate::records::cst_type_pack_generic::CstTypePackGeneric;
use crate::records::position::Position;
use crate::rtti::CstNodeClass;

impl CstTypePackGeneric {
    pub fn new(ellipsis_position: Position) -> Self {
        Self {
            base: CstNode {
                class_index: <Self as CstNodeClass>::CLASS_INDEX,
            },
            ellipsis_position,
        }
    }
}

#[allow(non_snake_case)]
#[export_name = "luaur_cst_type_pack_generic_cst_type_pack_generic"]
pub extern "C" fn cst_type_pack_generic_cst_type_pack_generic(
    ellipsis_position: Position,
) -> CstTypePackGeneric {
    CstTypePackGeneric::new(ellipsis_position)
}
