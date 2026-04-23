mod enum_like;
mod fields;
mod struct_like;

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, GenericParam, Generics, Ident, Lifetime, Path};

use crate::attrs::ContainerAttrs;

pub(crate) struct ParseGenerics {
    pub(crate) lifetime: Lifetime,
    pub(crate) input_ident: Ident,
}

struct ImplHeader {
    impl_generics: TokenStream,
    ty_generics: TokenStream,
    where_clause: TokenStream,
}

pub(crate) fn derive_parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match expand_parse_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_parse_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let attrs = ContainerAttrs::from_input(input)?;

    let parse_generics = ParseGenerics {
        lifetime: unique_lifetime(&input.generics, "__typeward_parse"),
        input_ident: unique_type_param(&input.generics, "__TypewardInput"),
    };

    let all_parser_field_types =
        fields::collect_all_parser_field_types(&input.data, &attrs.crate_path)?;
    let impl_header = build_impl_header(
        input,
        &attrs.crate_path,
        &parse_generics,
        &all_parser_field_types,
    );

    let body = match &input.data {
        Data::Struct(data_struct) => {
            struct_like::expand_struct_body(data_struct, &attrs.crate_path, &parse_generics)
        }
        Data::Enum(data_enum) => {
            enum_like::expand_enum_body(&input.ident, data_enum, &attrs.crate_path, &parse_generics)
        }
        Data::Union(data_union) => {
            struct_like::expand_union_body(data_union, &attrs.crate_path, &parse_generics)
        }
    }?;

    let name = &input.ident;
    let impl_generics = &impl_header.impl_generics;
    let ty_generics = &impl_header.ty_generics;
    let where_clause = &impl_header.where_clause;
    let lifetime = &parse_generics.lifetime;
    let input_ident = &parse_generics.input_ident;
    let crate_path = &attrs.crate_path;

    Ok(quote! {
        impl #impl_generics #crate_path::parse::Parse<#lifetime, #input_ident> for #name #ty_generics #where_clause {
            fn parse_with_context(
                input: #input_ident,
                context: &mut #crate_path::parse::ParseOffsetContext,
            ) -> #crate_path::error::ParseResult<(Self, #input_ident)> {
                #body
            }
        }
    })
}

fn build_impl_header(
    input: &DeriveInput,
    crate_path: &Path,
    parse_generics: &ParseGenerics,
    field_types: &[syn::Type],
) -> ImplHeader {
    let mut impl_generics = input.generics.clone();

    // Add lifetime and input type parameters
    impl_generics
        .params
        .push(GenericParam::Lifetime(syn::LifetimeParam::new(
            parse_generics.lifetime.clone(),
        )));
    impl_generics
        .params
        .push(GenericParam::Type(syn::TypeParam::from(
            parse_generics.input_ident.clone(),
        )));

    // Build where clause predicates
    let lifetime = &parse_generics.lifetime;
    let input_ident = &parse_generics.input_ident;
    let where_clause = impl_generics.make_where_clause();
    where_clause.predicates.push(syn::parse_quote!(
        #input_ident: #crate_path::parse::ParseOffsetInput<#lifetime>
    ));

    // Add explicit parser bounds so parser field types can propagate any
    // additional input requirements (for example ParseOffsetInput). Skip
    // directly self-referential parser types to avoid recursive trait-solver
    // cycles for recursive derives.
    let self_ident = &input.ident;
    let generic_type_params = generic_type_param_names(&input.generics);
    let mut seen_parse_bounds = HashSet::new();
    for ty in field_types {
        if type_mentions_name_or_self(ty, self_ident) {
            continue;
        }
        if !type_mentions_any_name(ty, &generic_type_params) {
            continue;
        }

        let parse_bound_key = quote!(#ty).to_string();
        if !seen_parse_bounds.insert(parse_bound_key) {
            continue;
        }

        where_clause.predicates.push(syn::parse_quote!(
            #ty: #crate_path::parse::Parse<#lifetime, #input_ident>
        ));
    }

    let (impl_generics_tokens, _, where_clause_tokens) = impl_generics.split_for_impl();
    let (_, ty_generics_tokens, _) = input.generics.split_for_impl();

    ImplHeader {
        impl_generics: quote!(#impl_generics_tokens),
        ty_generics: quote!(#ty_generics_tokens),
        where_clause: quote!(#where_clause_tokens),
    }
}

fn generic_type_param_names(generics: &Generics) -> HashSet<String> {
    generics
        .type_params()
        .map(|type_param| type_param.ident.to_string())
        .collect()
}

fn type_mentions_any_name(ty: &syn::Type, names: &HashSet<String>) -> bool {
    if names.is_empty() {
        return false;
    }

    quote!(#ty)
        .to_string()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|segment| !segment.is_empty() && names.contains(segment))
}

fn type_mentions_name_or_self(ty: &syn::Type, name: &Ident) -> bool {
    let target = name.to_string();
    quote!(#ty)
        .to_string()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|segment| segment == target || segment == "Self")
}

fn unique_name(
    generics: &Generics,
    base: &str,
    used_names: impl Fn(&Generics) -> HashSet<String>,
) -> String {
    let used = used_names(generics);
    let mut suffix = 0usize;
    loop {
        let candidate = if suffix == 0 {
            base.to_string()
        } else {
            format!("{base}_{suffix}")
        };

        if !used.contains(&candidate) {
            return candidate;
        }

        suffix += 1;
    }
}

fn unique_lifetime(generics: &Generics, base: &str) -> Lifetime {
    let name = unique_name(generics, base, |g| {
        g.lifetimes()
            .map(|lt| lt.lifetime.ident.to_string())
            .collect()
    });
    Lifetime::new(&format!("'{name}"), Span::call_site())
}

fn unique_type_param(generics: &Generics, base: &str) -> Ident {
    let name = unique_name(generics, base, |g| {
        g.params
            .iter()
            .filter_map(|p| match p {
                GenericParam::Type(ty) => Some(ty.ident.to_string()),
                GenericParam::Const(c) => Some(c.ident.to_string()),
                GenericParam::Lifetime(_) => None,
            })
            .collect()
    });
    Ident::new(&name, Span::call_site())
}
