use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Result, Token};

pub struct ModuleEntry {
    pub name: LitStr,
    pub source_or_path: LitStr,
}

impl Parse for ModuleEntry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![=>]>()?;
        let source_or_path = input.parse()?;
        Ok(Self {
            name,
            source_or_path,
        })
    }
}
