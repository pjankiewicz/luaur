use crate::records::ast_array::AstArray;
use crate::records::ast_expr::AstExpr;
use crate::records::ast_expr_interp_string::AstExprInterpString;
use crate::records::ast_node::AstNode;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;

impl AstExprInterpString {
    pub fn new(
        location: Location,
        strings: AstArray<AstArray<core::ffi::c_char>>,
        expressions: AstArray<*mut AstExpr>,
    ) -> Self {
        Self {
            base: AstExpr {
                base: AstNode {
                    class_index: <Self as AstNodeClass>::CLASS_INDEX,
                    location,
                },
            },
            strings,
            expressions,
        }
    }
}

#[export_name = "luaur_ast_expr_interp_string_ast_expr_interp_string"]
pub extern "C" fn ast_expr_interp_string_ast_expr_interp_string(
    location: Location,
    strings: AstArray<AstArray<core::ffi::c_char>>,
    expressions: AstArray<*mut AstExpr>,
) -> AstExprInterpString {
    AstExprInterpString::new(location, strings, expressions)
}
