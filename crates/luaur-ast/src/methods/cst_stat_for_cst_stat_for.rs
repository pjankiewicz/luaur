use crate::records::cst_node::CstNode;
use crate::records::cst_stat_for::CstStatFor;
use crate::records::position::Position;
use crate::rtti::CstNodeClass;

impl CstStatFor {
    pub fn new(
        annotation_colon_position: Position,
        equals_position: Position,
        end_comma_position: Position,
        step_comma_position: Position,
    ) -> Self {
        Self {
            base: CstNode::new(Self::CLASS_INDEX),
            annotation_colon_position: annotation_colon_position,
            equals_position: equals_position,
            end_comma_position: end_comma_position,
            step_comma_position: step_comma_position,
        }
    }
}

#[export_name = "luaur_cst_stat_for_cst_stat_for"]
pub extern "C" fn cst_stat_for_cst_stat_for(
    annotation_colon_position: Position,
    equals_position: Position,
    end_comma_position: Position,
    step_comma_position: Position,
) -> CstStatFor {
    CstStatFor::new(
        annotation_colon_position,
        equals_position,
        end_comma_position,
        step_comma_position,
    )
}
