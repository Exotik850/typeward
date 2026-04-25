use syn::{DeriveInput, Expr, Field, Path, Type, parse_quote};

pub(crate) struct ContainerAttrs {
    pub(crate) crate_path: Path,
    pub(crate) recursive: bool,
}

pub(crate) struct FieldAttrs {
    pub(crate) parser_ty: Option<Type>,
    pub(crate) mappers: Vec<FieldMapper>,
}

#[derive(Clone)]
pub(crate) enum FieldMapper {
    Infallible(Expr),
    Fallible(Expr),
}

impl ContainerAttrs {
    pub(crate) fn from_input(input: &DeriveInput) -> syn::Result<Self> {
        let mut crate_path: Option<Path> = None;
        let mut recursive = false;

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
                } else if meta.path.is_ident("recursive") {
                    if recursive {
                        return Err(meta.error("duplicate `recursive` argument"));
                    }

                    if !meta.input.is_empty() {
                        return Err(meta.error("`recursive` does not accept arguments"));
                    }

                    recursive = true;
                    Ok(())
                } else {
                    Err(meta.error(
                        "unsupported parse attribute; expected `crate = path` or `recursive`",
                    ))
                }
            })?;
        }

        Ok(Self {
            crate_path: crate_path.unwrap_or_else(|| parse_quote!(::typeward)),
            recursive,
        })
    }
}

impl FieldAttrs {
    pub(crate) fn from_field(field: &Field, crate_path: &Path) -> syn::Result<Self> {
        let mut parser_ty: Option<Type> = None;
        let mut mappers: Vec<FieldMapper> = Vec::new();

        for attr in &field.attrs {
            if !attr.path().is_ident("parse") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("from") {
                    if parser_ty.is_some() {
                        return Err(meta.error("duplicate parser source; `from` or `ws` may only be used once"));
                    }

                    let content;
                    syn::parenthesized!(content in meta.input);

                    let from_parser_ty = content.parse::<Type>()?;

                    if !content.is_empty() {
                        content.parse::<syn::Token![,]>()?;
                        if content.is_empty() {
                            return Err(content.error("expected mapper expression after comma"));
                        }

                        let mapper = content.parse::<Expr>()?;
                        mappers.push(FieldMapper::Infallible(mapper));
                    }

                    if !content.is_empty() {
                        return Err(content.error("unexpected tokens in `from` argument"));
                    }

                    parser_ty = Some(from_parser_ty);
                    Ok(())
                } else if meta.path.is_ident("ws") {
                    if parser_ty.is_some() {
                        return Err(meta.error("duplicate parser source; `from` or `ws` may only be used once"));
                    }

                    if !meta.input.is_empty() {
                        return Err(meta.error("`ws` does not accept arguments"));
                    }

                    let field_ty = &field.ty;
                    parser_ty = Some(parse_quote!(#crate_path::combinators::ws::Ws<#field_ty>));
                    mappers.push(FieldMapper::Infallible(
                        parse_quote!(|__typeward_ws| __typeward_ws.into_inner()),
                    ));
                    Ok(())
                } else if meta.path.is_ident("map") {
                    let content;
                    syn::parenthesized!(content in meta.input);

                    let mapper = content.parse::<Expr>()?;

                    if !content.is_empty() {
                        return Err(content.error("unexpected tokens in `map` argument"));
                    }

                    mappers.push(FieldMapper::Infallible(mapper));
                    Ok(())
                } else if meta.path.is_ident("try_map") {
                    let content;
                    syn::parenthesized!(content in meta.input);

                    let mapper = content.parse::<Expr>()?;

                    if !content.is_empty() {
                        return Err(content.error("unexpected tokens in `try_map` argument"));
                    }

                    mappers.push(FieldMapper::Fallible(mapper));
                    Ok(())
                } else {
                    Err(meta.error(
                        "unsupported parse field attribute; expected `ws`, `from(Type[, mapper])`, `map(expr)`, or `try_map(expr)`",
                    ))
                }
            })?;
        }

        Ok(Self { parser_ty, mappers })
    }

    pub(crate) fn parser_ty_or(&self, fallback: &Type) -> Type {
        self.parser_ty
            .as_ref()
            .cloned()
            .unwrap_or_else(|| fallback.clone())
    }

    pub(crate) fn mappers(&self) -> Vec<FieldMapper> {
        self.mappers.clone()
    }
}
