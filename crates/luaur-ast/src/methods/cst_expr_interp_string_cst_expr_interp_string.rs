use crate::records::ast_array::AstArray;
use crate::records::cst_expr_interp_string::CstExprInterpString;
use crate::records::cst_node::CstNode;
use crate::records::position::Position;
use crate::rtti::CstNodeClass;

impl CstExprInterpString {
    pub fn new(
        source_strings: AstArray<AstArray<i8>>,
        string_positions: AstArray<Position>,
    ) -> Self {
        Self {
            base: CstNode {
                class_index: <Self as CstNodeClass>::CLASS_INDEX,
            },
            source_strings,
            string_positions,
        }
    }
}

#[export_name = "luaur_cst_expr_interp_string_cst_expr_interp_string"]
pub extern "C" fn cst_expr_interp_string_cst_expr_interp_string(
    source_strings: AstArray<AstArray<i8>>,
    string_positions: AstArray<Position>,
) -> CstExprInterpString {
    CstExprInterpString::new(source_strings, string_positions)
}
