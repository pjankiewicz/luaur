use crate::module_entry::ModuleEntry;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, Error, Ident, LitStr, Result, Token};

pub struct FileInput {
    pub root: LitStr,
    pub module: Option<LitStr>,
    pub defs: Option<LitStr>,
    pub modules: Vec<ModuleEntry>,
}

impl Parse for FileInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(LitStr) {
            let root = input.parse()?;
            if !input.is_empty() {
                return Err(input.error("unexpected tokens after Luau file path"));
            }
            return Ok(Self {
                root,
                module: None,
                defs: None,
                modules: Vec::new(),
            });
        }

        let mut root = None;
        let mut module = None;
        let mut defs = None;
        let mut modules = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match key.to_string().as_str() {
                "root" => assign_once(&mut root, key, input.parse()?)?,
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
                        "expected one of `root`, `module`, `defs`, or `modules`",
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
            root: root.ok_or_else(|| input.error("missing required `root` field"))?,
            module,
            defs,
            modules,
        })
    }
}

fn assign_once(slot: &mut Option<LitStr>, key: Ident, value: LitStr) -> Result<()> {
    if slot.is_none() {
        *slot = Some(value);
        Ok(())
    } else {
        Err(Error::new(key.span(), format!("duplicate `{key}` field")))
    }
}
