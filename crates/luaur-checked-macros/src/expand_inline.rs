use crate::inline_input::InlineInput;
use crate::report;
use proc_macro2::TokenStream;
use quote::quote;

pub fn expand(tokens: TokenStream) -> TokenStream {
    let input = match syn::parse2::<InlineInput>(tokens) {
        Ok(input) => input,
        Err(err) => return err.to_compile_error(),
    };

    let source = input.source.value();
    let result = if input.modules.is_empty() {
        if let Some(defs) = &input.defs {
            luaur_rt::check_with_definitions(&source, &defs.value())
        } else {
            luaur_rt::check(&source)
        }
    } else {
        check_inline_modules(&input, &source)
    };

    if let Err(diagnostics) = result {
        return report::diagnostics_error(input.source.span(), &diagnostics);
    }

    let source_lit = input.source;
    quote! { #source_lit }
}

fn check_inline_modules(
    input: &InlineInput,
    source: &str,
) -> Result<(), Vec<luaur_rt::TypeDiagnostic>> {
    let root_module = input
        .module
        .as_ref()
        .map(|module| module.value())
        .unwrap_or_else(|| "main".to_string());
    let mut modules = Vec::with_capacity(input.modules.len() + 1);
    modules.push((root_module.clone(), source.to_string()));

    for module in &input.modules {
        let name = module.name.value();
        if name == root_module {
            return Err(vec![luaur_rt::TypeDiagnostic {
                module: Some(root_module),
                line: 1,
                column: 1,
                end_line: 1,
                end_column: 1,
                message: "module map duplicates the root module".to_string(),
                in_definitions: false,
            }]);
        }
        modules.push((name, module.source_or_path.value()));
    }

    let borrowed: Vec<(&str, &str)> = modules
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();

    if let Some(defs) = &input.defs {
        luaur_rt::check_modules_with_definitions(&root_module, &borrowed, &defs.value())
    } else {
        luaur_rt::check_modules(&root_module, &borrowed)
    }
}
