use crate::records::ast_array::AstArray;
use crate::records::ast_expr::AstExpr;
use crate::records::ast_expr_table::{AstExprTable, Item};
use crate::records::ast_node::AstNode;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;

impl AstExprTable {
    pub fn new(location: Location, items: AstArray<Item>) -> Self {
        Self {
            base: AstExpr {
                base: AstNode {
                    class_index: <Self as AstNodeClass>::CLASS_INDEX,
                    location,
                },
            },
            items,
        }
    }
}

#[export_name = "luaur_ast_expr_table_ast_expr_table"]
pub extern "C" fn ast_expr_table_ast_expr_table(
    location: Location,
    items: AstArray<Item>,
) -> AstExprTable {
    AstExprTable::new(location, items)
}
