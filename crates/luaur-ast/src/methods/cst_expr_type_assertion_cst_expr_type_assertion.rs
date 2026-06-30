use crate::records::cst_expr_type_assertion::CstExprTypeAssertion;
use crate::records::cst_node::CstNode;
use crate::records::position::Position;
use crate::rtti::CstNodeClass;

impl CstExprTypeAssertion {
    pub fn new(op_position: Position) -> Self {
        Self {
            base: CstNode {
                class_index: <Self as CstNodeClass>::CLASS_INDEX,
            },
            op_position: op_position,
        }
    }
}

#[export_name = "luaur_cst_expr_type_assertion_cst_expr_type_assertion"]
pub extern "C" fn cst_expr_type_assertion_cst_expr_type_assertion(
    op_position: Position,
) -> CstExprTypeAssertion {
    CstExprTypeAssertion::new(op_position)
}
