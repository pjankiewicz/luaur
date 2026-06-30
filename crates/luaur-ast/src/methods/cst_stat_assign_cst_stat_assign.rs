use crate::records::ast_array::AstArray;
use crate::records::cst_node::CstNode;
use crate::records::cst_stat_assign::CstStatAssign;
use crate::records::position::Position;
use crate::rtti::CstNodeClass;

impl CstStatAssign {
    pub fn new(
        vars_comma_positions: AstArray<Position>,
        equals_position: Position,
        values_comma_positions: AstArray<Position>,
    ) -> Self {
        Self {
            base: CstNode {
                class_index: <Self as CstNodeClass>::CLASS_INDEX,
            },
            vars_comma_positions: vars_comma_positions,
            equals_position: equals_position,
            values_comma_positions: values_comma_positions,
        }
    }
}

#[export_name = "luaur_cst_stat_assign_cst_stat_assign"]
pub extern "C" fn cst_stat_assign_cst_stat_assign(
    vars_comma_positions: AstArray<Position>,
    equals_position: Position,
    values_comma_positions: AstArray<Position>,
) -> CstStatAssign {
    CstStatAssign::new(
        vars_comma_positions,
        equals_position,
        values_comma_positions,
    )
}
