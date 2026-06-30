use crate::records::ast_array::AstArray;
use crate::records::ast_node::AstNode;
use crate::records::ast_type::AstType;
use crate::records::ast_type_error::AstTypeError;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;

impl AstTypeError {
    pub fn new(
        location: Location,
        types: AstArray<*mut AstType>,
        is_missing: bool,
        message_index: u32,
    ) -> Self {
        Self {
            base: AstType {
                base: AstNode {
                    class_index: <Self as AstNodeClass>::CLASS_INDEX,
                    location,
                },
            },
            types,
            is_missing,
            message_index,
        }
    }
}

#[export_name = "luaur_ast_type_error_ast_type_error"]
pub extern "C" fn ast_type_error_ast_type_error(
    location: Location,
    types: AstArray<*mut AstType>,
    is_missing: bool,
    message_index: u32,
) -> AstTypeError {
    AstTypeError::new(location, types, is_missing, message_index)
}
