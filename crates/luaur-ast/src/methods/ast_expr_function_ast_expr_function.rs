use crate::records::ast_array::AstArray;
use crate::records::ast_attr::AstAttr;
use crate::records::ast_expr::AstExpr;
use crate::records::ast_expr_function::AstExprFunction;
use crate::records::ast_generic_type::AstGenericType;
use crate::records::ast_generic_type_pack::AstGenericTypePack;
use crate::records::ast_local::AstLocal;
use crate::records::ast_name::AstName;
use crate::records::ast_node::AstNode;
use crate::records::ast_stat_block::AstStatBlock;
use crate::records::ast_type_pack::AstTypePack;
use crate::records::location::Location;
use crate::rtti::AstNodeClass;

impl AstExprFunction {
    pub fn new(
        location: Location,
        attributes: AstArray<*mut AstAttr>,
        generics: AstArray<*mut AstGenericType>,
        generic_packs: AstArray<*mut AstGenericTypePack>,
        self_: *mut AstLocal,
        args: AstArray<*mut AstLocal>,
        vararg: bool,
        vararg_location: Location,
        body: *mut AstStatBlock,
        function_depth: usize,
        debugname: AstName,
        return_annotation: *mut AstTypePack,
        vararg_annotation: *mut AstTypePack,
        arg_location: Option<Location>,
    ) -> Self {
        Self {
            base: AstExpr {
                base: AstNode {
                    class_index: <Self as AstNodeClass>::CLASS_INDEX,
                    location,
                },
            },
            attributes,
            generics,
            generic_packs,
            self_,
            args,
            return_annotation,
            vararg,
            vararg_location,
            vararg_annotation,
            body,
            function_depth,
            debugname,
            arg_location,
        }
    }
}

#[export_name = "luaur_ast_expr_function_ast_expr_function"]
pub unsafe extern "C" fn ast_expr_function_ast_expr_function(
    location: Location,
    attributes: AstArray<*mut AstAttr>,
    generics: AstArray<*mut AstGenericType>,
    generic_packs: AstArray<*mut AstGenericTypePack>,
    self_: *mut AstLocal,
    args: AstArray<*mut AstLocal>,
    vararg: bool,
    vararg_location: Location,
    body: *mut AstStatBlock,
    function_depth: usize,
    debugname: AstName,
    return_annotation: *mut AstTypePack,
    vararg_annotation: *mut AstTypePack,
    arg_location: *const Option<Location>,
) -> AstExprFunction {
    AstExprFunction::new(
        location,
        attributes,
        generics,
        generic_packs,
        self_,
        args,
        vararg,
        vararg_location,
        body,
        function_depth,
        debugname,
        return_annotation,
        vararg_annotation,
        if arg_location.is_null() {
            None
        } else {
            *arg_location
        },
    )
}
