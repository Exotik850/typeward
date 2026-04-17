use syn::{DeriveInput, Path, parse_quote};

pub(crate) struct ContainerAttrs {
    pub(crate) crate_path: Path,
}

impl ContainerAttrs {
    pub(crate) fn from_input(input: &DeriveInput) -> syn::Result<Self> {
        let mut crate_path: Option<Path> = None;

        for attr in &input.attrs {
            if !attr.path().is_ident("parse") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate") {
                    if crate_path.is_some() {
                        return Err(meta.error("duplicate `crate` argument"));
                    }

                    let value = meta.value()?;
                    crate_path = Some(value.parse::<Path>()?);
                    Ok(())
                } else {
                    Err(meta.error("unsupported parse attribute; expected `crate = path`"))
                }
            })?;
        }

        Ok(Self {
            crate_path: crate_path.unwrap_or_else(|| parse_quote!(::typeward)),
        })
    }
}
