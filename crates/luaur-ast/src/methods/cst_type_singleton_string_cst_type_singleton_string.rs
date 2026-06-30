use crate::records::ast_array::AstArray;
use crate::records::cst_node::CstNode;
use crate::records::cst_type_singleton_string::CstTypeSingletonString;
use crate::rtti::CstNodeClass;
use luaur_common::LUAU_ASSERT;

impl CstTypeSingletonString {
    pub fn new(
        source_string: AstArray<core::ffi::c_char>,
        quote_style: crate::enums::quote_style_cst::QuoteStyle,
        block_depth: u32,
    ) -> Self {
        LUAU_ASSERT!(quote_style != crate::enums::quote_style_cst::QuoteStyle::QuotedInterp);

        Self {
            base: CstNode {
                class_index: <Self as CstNodeClass>::CLASS_INDEX,
            },
            source_string,
            quote_style,
            block_depth,
        }
    }
}

#[export_name = "luaur_cst_type_singleton_string_cst_type_singleton_string"]
pub extern "C" fn cst_type_singleton_string_cst_type_singleton_string(
    source_string: AstArray<core::ffi::c_char>,
    quote_style: crate::enums::quote_style_cst::QuoteStyle,
    block_depth: u32,
) -> CstTypeSingletonString {
    CstTypeSingletonString::new(source_string, quote_style, block_depth)
}
