use crate::records::ast_generic_type_pack::AstGenericTypePack;
use crate::records::ast_name::AstName;
use crate::records::ast_node::AstNode;
use crate::records::ast_type_pack::AstTypePack;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;

impl AstGenericTypePack {
    pub fn new(location: Location, name: AstName, default_value: *mut AstTypePack) -> Self {
        Self {
            base: AstNode {
                class_index: <Self as AstNodeClass>::CLASS_INDEX,
                location,
            },
            name,
            default_value,
        }
    }
}

#[export_name = "luaur_ast_generic_type_pack_ast_generic_type_pack"]
pub extern "C" fn ast_generic_type_pack_ast_generic_type_pack(
    location: Location,
    name: AstName,
    default_value: *mut AstTypePack,
) -> AstGenericTypePack {
    AstGenericTypePack::new(location, name, default_value)
}
