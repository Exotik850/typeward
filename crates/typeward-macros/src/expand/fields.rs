use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, Expr, Field, Fields, Ident, Path, Type};

use super::ParseGenerics;
use crate::attrs::FieldAttrs;

#[derive(Clone)]
pub(crate) enum FieldShape {
    Unit,
    Named(Vec<Ident>),
    Unnamed(usize),
}

#[derive(Clone)]
pub(crate) struct FieldPlan {
    pub(crate) parser_ty: Type,
    pub(crate) mapper: Option<Expr>,
}

pub(crate) struct ParsedFields {
    pub(crate) parse_tokens: TokenStream,
    pub(crate) bindings: Vec<Ident>,
    pub(crate) shape: FieldShape,
    pub(crate) field_plans: Vec<FieldPlan>,
}

pub(crate) fn describe_fields(fields: &Fields) -> FieldShape {
    match fields {
        Fields::Named(named) => FieldShape::Named(
            named
                .named
                .iter()
                .map(|field| {
                    field
                        .ident
                        .clone()
                        .expect("named field should have an ident")
                })
                .collect(),
        ),
        Fields::Unnamed(unnamed) => FieldShape::Unnamed(unnamed.unnamed.len()),
        Fields::Unit => FieldShape::Unit,
    }
}

pub(crate) fn parser_type_from_types(types: &[Type], crate_path: &Path) -> TokenStream {
    match types.len() {
        0 => quote!(()),
        1 => {
            let ty = &types[0];
            quote!(#ty)
        }
        _ => quote!(#crate_path::and!(#(#types),*)),
    }
}

pub(crate) fn parser_type_from_field_plans(
    field_plans: &[FieldPlan],
    crate_path: &Path,
) -> TokenStream {
    let types: Vec<_> = field_plans
        .iter()
        .map(|plan| plan.parser_ty.clone())
        .collect();
    parser_type_from_types(&types, crate_path)
}

pub(crate) fn collect_all_parser_field_types(
    data: &Data,
    crate_path: &Path,
) -> syn::Result<Vec<Type>> {
    match data {
        Data::Struct(data_struct) => collect_field_parser_types(&data_struct.fields, crate_path),
        Data::Union(data_union) => {
            let union_fields = Fields::Named(data_union.fields.clone());
            collect_field_parser_types(&union_fields, crate_path)
        }
        Data::Enum(data_enum) => data_enum
            .variants
            .iter()
            .map(|variant| collect_field_parser_types(&variant.fields, crate_path))
            .collect::<syn::Result<Vec<_>>>()
            .map(|nested| nested.into_iter().flatten().collect()),
    }
}

pub(crate) fn build_fields_parse_plan(
    fields: &Fields,
    crate_path: &Path,
    parse_generics: &ParseGenerics,
    binding_prefix: &str,
) -> syn::Result<ParsedFields> {
    let shape = describe_fields(fields);
    let field_plans = collect_field_plans(fields, crate_path)?;

    let bindings: Vec<Ident> = (0..field_plans.len())
        .map(|index| format_ident!("{binding_prefix}_{index}"))
        .collect();

    let parse_tokens = if field_plans.is_empty() {
        quote! {
            let __typeward_remaining = input;
        }
    } else {
        let lifetime = &parse_generics.lifetime;
        let input_ident = &parse_generics.input_ident;
        let parser_type = parser_type_from_field_plans(&field_plans, crate_path);
        let parsed_value = format_ident!("{binding_prefix}_parsed_value");
        let parsed_binding_prefix = format!("{binding_prefix}_parsed_field");
        let parsed_value_expr = quote!(#parsed_value);
        let binding_setup_tokens = build_binding_transform_tokens(
            &bindings,
            &field_plans,
            &parsed_value_expr,
            crate_path,
            &parsed_binding_prefix,
        );

        quote! {
            let (#parsed_value, __typeward_remaining) =
                <#parser_type as #crate_path::parse::Parse<#lifetime, #input_ident>>::parse_with_context(input, context)?;
            #binding_setup_tokens
        }
    };

    Ok(ParsedFields {
        parse_tokens,
        bindings,
        shape,
        field_plans,
    })
}

pub(crate) fn build_binding_transform_tokens(
    bindings: &[Ident],
    field_plans: &[FieldPlan],
    parsed_value: &TokenStream,
    crate_path: &Path,
    parsed_binding_prefix: &str,
) -> TokenStream {
    debug_assert_eq!(bindings.len(), field_plans.len());

    match bindings.len() {
        0 => quote! {},
        1 => {
            let parsed_binding = format_ident!("{parsed_binding_prefix}_0");
            let binding = &bindings[0];
            let parser_ty = &field_plans[0].parser_ty;
            let mapper = &field_plans[0].mapper;
            let map_stmt = map_parsed_value(binding, &parsed_binding, parser_ty, mapper);

            quote! {
                let #parsed_binding = #parsed_value;
                #map_stmt
            }
        }
        _ => {
            let parsed_bindings: Vec<Ident> = (0..bindings.len())
                .map(|index| format_ident!("{parsed_binding_prefix}_{index}"))
                .collect();

            let mapping_stmts = bindings.iter().zip(&parsed_bindings).zip(field_plans).map(
                |((binding, parsed_binding), plan)| {
                    map_parsed_value(binding, parsed_binding, &plan.parser_ty, &plan.mapper)
                },
            );

            let parser_tys: Vec<_> = field_plans.iter().map(|plan| &plan.parser_ty).collect();

            quote! {
                let (#(#parsed_bindings),*) =
                    #crate_path::unpack_and!(#parsed_value, (#(#parser_tys),*));
                #(#mapping_stmts)*
            }
        }
    }
}

fn map_parsed_value(
    binding: &Ident,
    parsed_binding: &Ident,
    parser_ty: &Type,
    mapper: &Option<Expr>,
) -> TokenStream {
    if let Some(mapper) = mapper {
        quote! {
            let #binding = {
                fn __typeward_apply_mapper<In, Out>(value: In, mapper: impl FnOnce(In) -> Out) -> Out {
                    mapper(value)
                }

                __typeward_apply_mapper::<#parser_ty, _>(#parsed_binding, #mapper)
            };
        }
    } else {
        quote! {
            let #binding = #parsed_binding;
        }
    }
}

pub(crate) fn constructor(
    prefix: Option<&Ident>,
    shape: &FieldShape,
    bindings: &[Ident],
) -> TokenStream {
    let ident = if let Some(variant) = prefix {
        quote!(Self::#variant)
    } else {
        quote!(Self)
    };

    match shape {
        FieldShape::Unit => quote!(#ident),
        FieldShape::Named(fields) => {
            quote!(#ident { #(#fields: #bindings),* })
        }
        FieldShape::Unnamed(arity) => {
            debug_assert_eq!(*arity, bindings.len());
            quote!(#ident(#(#bindings),*))
        }
    }
}

pub(crate) fn struct_constructor(shape: &FieldShape, bindings: &[Ident]) -> TokenStream {
    constructor(None, shape, bindings)
}

pub(crate) fn union_constructor(field_names: &[Ident], bindings: &[Ident]) -> TokenStream {
    debug_assert_eq!(field_names.len(), bindings.len());

    let first_field = &field_names[0];
    let first_binding = &bindings[0];
    let ignored_bindings: Vec<_> = bindings.iter().skip(1).collect();

    let ignore_rest = if ignored_bindings.is_empty() {
        quote! {}
    } else {
        quote! {
            let _ = (#(&#ignored_bindings),*);
        }
    };

    quote!({
        #ignore_rest
        Self {
            #first_field: #first_binding,
        }
    })
}

fn collect_field_parser_types(fields: &Fields, crate_path: &Path) -> syn::Result<Vec<Type>> {
    collect_field_plans(fields, crate_path)
        .map(|plans| plans.into_iter().map(|plan| plan.parser_ty).collect())
}

fn collect_field_plans(fields: &Fields, crate_path: &Path) -> syn::Result<Vec<FieldPlan>> {
    match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|field| field_plan(field, crate_path))
            .collect(),
        Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .map(|field| field_plan(field, crate_path))
            .collect(),
        Fields::Unit => Ok(Vec::new()),
    }
}

fn field_plan(field: &Field, crate_path: &Path) -> syn::Result<FieldPlan> {
    let attrs = FieldAttrs::from_field(field, crate_path)?;
    Ok(FieldPlan {
        parser_ty: attrs.parser_ty_or(&field.ty),
        mapper: attrs.mapper(),
    })
}
