use syn::{DeriveInput, Expr, Field, Path, Type, parse_quote};

pub(crate) struct ContainerAttrs {
    pub(crate) crate_path: Path,
}

pub(crate) struct FieldAttrs {
    pub(crate) from: Option<FromAttr>,
    pub(crate) ws: bool,
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
        let mut ws = false;

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
                } else if meta.path.is_ident("ws") {
                    if ws {
                        return Err(meta.error("duplicate `ws` argument"));
                    }

                    if !meta.input.is_empty() {
                        return Err(meta.error("`ws` does not accept arguments"));
                    }

                    ws = true;
                    Ok(())
                } else {
                    Err(meta.error(
                        "unsupported parse field attribute; expected `ws` or `from(Type, mapper)`",
                    ))
                }
            })?;
        }

        Ok(Self { from, ws })
    }

    pub(crate) fn parser_ty_or(&self, fallback: &Type, crate_path: &Path) -> Type {
        self.from.as_ref().map_or_else(
            || {
                if self.ws {
                    parse_quote!(#crate_path::combinators::ws::Ws<#fallback>)
                } else {
                    fallback.clone()
                }
            },
            |from| from.parser_ty.clone(),
        )
    }

    pub(crate) fn mapper(&self) -> Option<Expr> {
        self.from.as_ref().map_or_else(
            || {
                self.ws
                    .then(|| parse_quote!(|__typeward_ws| __typeward_ws.into_inner()))
            },
            |from| Some(from.mapper.clone()),
        )
    }
}
