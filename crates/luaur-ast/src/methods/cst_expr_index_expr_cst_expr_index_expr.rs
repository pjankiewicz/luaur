use crate::records::cst_expr_index_expr::CstExprIndexExpr;
use crate::records::cst_node::CstNode;
use crate::records::position::Position;
use crate::rtti::CstNodeClass;

impl CstExprIndexExpr {
    pub fn new(open_bracket_position: Position, close_bracket_position: Position) -> Self {
        Self {
            base: CstNode {
                class_index: <Self as CstNodeClass>::CLASS_INDEX,
            },
            open_bracket_position: open_bracket_position,
            close_bracket_position: close_bracket_position,
        }
    }
}

#[export_name = "luaur_cst_expr_index_expr_cst_expr_index_expr"]
pub extern "C" fn cst_expr_index_expr_cst_expr_index_expr(
    open_bracket_position: Position,
    close_bracket_position: Position,
) -> CstExprIndexExpr {
    CstExprIndexExpr::new(open_bracket_position, close_bracket_position)
}
