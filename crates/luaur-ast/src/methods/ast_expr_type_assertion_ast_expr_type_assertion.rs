use crate::records::ast_expr::AstExpr;
use crate::records::ast_expr_type_assertion::AstExprTypeAssertion;
use crate::records::ast_node::AstNode;
use crate::records::ast_type::AstType;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;

impl AstExprTypeAssertion {
    pub fn new(location: Location, expr: *mut AstExpr, annotation: *mut AstType) -> Self {
        Self {
            base: AstExpr {
                base: AstNode {
                    class_index: <Self as AstNodeClass>::CLASS_INDEX,
                    location,
                },
            },
            expr,
            annotation,
        }
    }
}

#[export_name = "luaur_ast_expr_type_assertion_ast_expr_type_assertion"]
pub extern "C" fn ast_expr_type_assertion_ast_expr_type_assertion(
    location: Location,
    expr: *mut AstExpr,
    annotation: *mut AstType,
) -> AstExprTypeAssertion {
    AstExprTypeAssertion::new(location, expr, annotation)
}
