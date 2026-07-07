use crate::module_entry::ModuleEntry;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, Error, Ident, LitStr, Result, Token};

pub struct InlineInput {
    pub source: LitStr,
    pub module: Option<LitStr>,
    pub defs: Option<LitStr>,
    pub modules: Vec<ModuleEntry>,
}

impl Parse for InlineInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(LitStr) {
            return parse_literal_form(input);
        }

        let mut source = None;
        let mut module = None;
        let mut defs = None;
        let mut modules = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "source" => assign_once(&mut source, key, input.parse()?)?,
                "module" => assign_once(&mut module, key, input.parse()?)?,
                "defs" => assign_once(&mut defs, key, input.parse()?)?,
                "modules" => {
                    if !modules.is_empty() {
                        return Err(Error::new(key.span(), "duplicate `modules` field"));
                    }
                    let content;
                    braced!(content in input);
                    modules = Punctuated::<ModuleEntry, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect();
                }
                _ => {
                    return Err(Error::new(
                        key.span(),
                        "expected one of `source`, `module`, `defs`, or `modules`",
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else if !input.is_empty() {
                return Err(input.error("expected `,`"));
            }
        }

        Ok(Self {
            source: source.ok_or_else(|| input.error("missing required `source` field"))?,
            module,
            defs,
            modules,
        })
    }
}

fn parse_literal_form(input: ParseStream<'_>) -> Result<InlineInput> {
    let source = input.parse()?;
    let mut defs = None;

    if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
        let key: Ident = input.parse()?;
        if key != "defs" {
            return Err(Error::new(key.span(), "expected `defs`"));
        }
        input.parse::<Token![=]>()?;
        defs = Some(input.parse()?);
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }

    if !input.is_empty() {
        return Err(input.error("unexpected tokens after inline Luau source"));
    }

    Ok(InlineInput {
        source,
        module: None,
        defs,
        modules: Vec::new(),
    })
}

fn assign_once(slot: &mut Option<LitStr>, key: Ident, value: LitStr) -> Result<()> {
    if slot.is_none() {
        *slot = Some(value);
        Ok(())
    } else {
        Err(Error::new(key.span(), format!("duplicate `{key}` field")))
    }
}
