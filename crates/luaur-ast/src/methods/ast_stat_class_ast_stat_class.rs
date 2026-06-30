use crate::records::ast_array::AstArray;
use crate::records::ast_local::AstLocal;
use crate::records::ast_node::AstNode;
use crate::records::ast_stat::AstStat;
use crate::records::ast_stat_class::AstStatClass;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;
use crate::type_aliases::ast_class_member::AstClassMember;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

impl AstStatClass {
    pub fn new(
        location: Location,
        name: *mut AstLocal,
        members: AstArray<AstClassMember>,
        exported: bool,
    ) -> Self {
        LUAU_ASSERT!(luaur_common::FFlag::DebugLuauUserDefinedClasses.get());
        Self {
            base: AstStat {
                base: AstNode {
                    class_index: <Self as AstNodeClass>::CLASS_INDEX,
                    location,
                },
                has_semicolon: false,
            },
            name,
            members,
            exported,
        }
    }
}

#[export_name = "luaur_ast_stat_class_ast_stat_class"]
pub extern "C" fn ast_stat_class_ast_stat_class(
    location: Location,
    name: *mut AstLocal,
    members: AstArray<AstClassMember>,
    exported: bool,
) -> AstStatClass {
    AstStatClass::new(location, name, members, exported)
}
