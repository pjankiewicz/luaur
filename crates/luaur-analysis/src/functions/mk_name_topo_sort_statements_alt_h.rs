use crate::records::identifier::Identifier;
use crate::records::internal_compiler_error::InternalCompilerError;
use luaur_ast::records::ast_stat_function::AstStatFunction;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

pub fn mk_name_ast_stat_function(function: &AstStatFunction) -> Identifier {
    // Import the overload that handles AstExpr (the type of function->name)
    use crate::functions::mk_name_topo_sort_statements_alt_g::mk_name_ast_expr;

    let name = unsafe { mk_name_ast_expr(&*function.name) };
    LUAU_ASSERT!(name.is_some());

    match name {
        Some(id) => id,
        None => {
            let err = InternalCompilerError::new(
                "Internal error: Function declaration has a bad name".to_string(),
                None,
                None,
            );
            panic!("{}", unsafe {
                core::ffi::CStr::from_ptr(err.what()).to_string_lossy()
            });
        }
    }
}
