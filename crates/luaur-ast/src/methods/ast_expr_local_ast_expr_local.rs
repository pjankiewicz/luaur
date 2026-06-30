use crate::records::ast_expr::AstExpr;
use crate::records::ast_expr_local::AstExprLocal;
use crate::records::ast_local::AstLocal;
use crate::records::ast_node::AstNode;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;

impl AstExprLocal {
    pub fn new(location: Location, local: *mut AstLocal, upvalue: bool) -> Self {
        Self {
            base: AstExpr {
                base: AstNode {
                    class_index: <Self as AstNodeClass>::CLASS_INDEX,
                    location,
                },
            },
            local,
            upvalue,
        }
    }
}

#[export_name = "luaur_ast_expr_local_ast_expr_local"]
pub extern "C" fn ast_expr_local_ast_expr_local(
    location: Location,
    local: *mut AstLocal,
    upvalue: bool,
) -> AstExprLocal {
    AstExprLocal::new(location, local, upvalue)
}
