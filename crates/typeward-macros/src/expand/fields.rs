use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, Fields, Ident, Path, Type};

use super::ParseGenerics;

#[derive(Clone)]
pub(crate) enum FieldShape {
    Unit,
    Named(Vec<Ident>),
    Unnamed(usize),
}

pub(crate) struct ParsedFields {
    pub(crate) parse_tokens: TokenStream,
    pub(crate) bindings: Vec<Ident>,
    pub(crate) shape: FieldShape,
    pub(crate) field_types: Vec<Type>,
}

pub(crate) fn describe_fields(fields: &Fields) -> (FieldShape, Vec<Type>) {
    let shape = match fields {
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
    };

    let types = collect_field_types(fields);
    (shape, types)
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

pub(crate) fn collect_all_field_types(data: &Data) -> Vec<Type> {
    match data {
        Data::Struct(data_struct) => collect_field_types(&data_struct.fields),
        Data::Union(data_union) => data_union
            .fields
            .named
            .iter()
            .map(|field| field.ty.clone())
            .collect(),
        Data::Enum(data_enum) => data_enum
            .variants
            .iter()
            .flat_map(|variant| collect_field_types(&variant.fields))
            .collect(),
    }
}

pub(crate) fn build_fields_parse_plan(
    fields: &Fields,
    crate_path: &Path,
    parse_generics: &ParseGenerics,
    binding_prefix: &str,
) -> ParsedFields {
    let (shape, types) = describe_fields(fields);
    let bindings: Vec<Ident> = (0..types.len())
        .map(|index| format_ident!("{binding_prefix}_{index}"))
        .collect();

    let lifetime = &parse_generics.lifetime;
    let input_ident = &parse_generics.input_ident;

    let parse_tokens = match types.len() {
        0 => {
            quote! {
                let __typeward_remaining = input;
            }
        }
        1 => {
            let binding = &bindings[0];
            let ty = &types[0];

            quote! {
                let (#binding, __typeward_remaining) = <#ty as #crate_path::parse::Parse<#lifetime, #input_ident>>::parse(input)?;
            }
        }
        _ => {
            let and_parser_type = parser_type_from_types(&types, crate_path);
            quote! {
                let (__typeward_and_value, __typeward_remaining) =
                    <#and_parser_type as #crate_path::parse::Parse<#lifetime, #input_ident>>::parse(input)?;
                let (#(#bindings),*) = #crate_path::unpack_and!(__typeward_and_value, #(#types),*);
            }
        }
    };

    ParsedFields {
        parse_tokens,
        bindings,
        shape,
        field_types: types,
    }
}

pub(crate) fn constructor(
    prefix: Option<&Ident>,
    shape: &FieldShape,
    bindings: &[Ident],
) -> TokenStream {
    let ident = match prefix {
        Some(variant) => quote!(Self::#variant),
        None => quote!(Self),
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

pub(crate) fn collect_field_types(fields: &Fields) -> Vec<Type> {
    match fields {
        Fields::Named(named) => named.named.iter().map(|field| field.ty.clone()).collect(),
        Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .map(|field| field.ty.clone())
            .collect(),
        Fields::Unit => Vec::new(),
    }
}
