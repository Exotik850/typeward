use syn::{DeriveInput, Expr, Field, Path, Type, parse_quote};

pub(crate) struct ContainerAttrs {
    pub(crate) crate_path: Path,
}

pub(crate) struct FieldAttrs {
    pub(crate) from: Option<FromAttr>,
}

#[derive(Clone)]
pub(crate) struct FromAttr {
    pub(crate) parser_ty: Type,
    pub(crate) mapper: Expr,
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

impl FieldAttrs {
    pub(crate) fn from_field(field: &Field) -> syn::Result<Self> {
        let mut from: Option<FromAttr> = None;

        for attr in &field.attrs {
            if !attr.path().is_ident("parse") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("from") {
                    if from.is_some() {
                        return Err(meta.error("duplicate `from` argument"));
                    }

                    let content;
                    syn::parenthesized!(content in meta.input);

                    let parser_ty = content.parse::<Type>()?;
                    if content.is_empty() {
                        return Err(content.error("expected `from(Type, mapper)`"));
                    }

                    content.parse::<syn::Token![,]>()?;
                    let mapper = content.parse::<Expr>()?;

                    if !content.is_empty() {
                        return Err(content.error("unexpected tokens in `from` argument"));
                    }

                    from = Some(FromAttr { parser_ty, mapper });
                    Ok(())
                } else {
                    Err(meta
                        .error("unsupported parse field attribute; expected `from(Type, mapper)`"))
                }
            })?;
        }

        Ok(Self { from })
    }

    pub(crate) fn parser_ty_or(&self, fallback: &Type) -> Type {
        self.from
            .as_ref()
            .map_or_else(|| fallback.clone(), |from| from.parser_ty.clone())
    }
}
