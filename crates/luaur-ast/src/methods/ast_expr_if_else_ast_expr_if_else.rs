use crate::records::ast_expr::AstExpr;
use crate::records::ast_expr_if_else::AstExprIfElse;
use crate::records::ast_node::AstNode;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;

impl AstExprIfElse {
    pub fn new(
        location: Location,
        condition: *mut AstExpr,
        has_then: bool,
        true_expr: *mut AstExpr,
        has_else: bool,
        false_expr: *mut AstExpr,
    ) -> Self {
        Self {
            base: AstExpr {
                base: AstNode {
                    class_index: <Self as AstNodeClass>::CLASS_INDEX,
                    location,
                },
            },
            condition,
            has_then,
            true_expr,
            has_else,
            false_expr,
        }
    }
}

#[export_name = "luaur_ast_expr_if_else_ast_expr_if_else"]
pub extern "C" fn ast_expr_if_else_ast_expr_if_else(
    location: &Location,
    condition: *mut AstExpr,
    has_then: bool,
    true_expr: *mut AstExpr,
    has_else: bool,
    false_expr: *mut AstExpr,
) -> AstExprIfElse {
    AstExprIfElse::new(
        *location, condition, has_then, true_expr, has_else, false_expr,
    )
}
