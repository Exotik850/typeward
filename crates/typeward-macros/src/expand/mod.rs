mod enum_like;
mod fields;
mod struct_like;

use std::collections::HashSet;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, GenericParam, Generics, Ident, Lifetime, Path, visit::Visit};

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
    ensure_recursive_parse_is_opted_in(&input.ident, &all_parser_field_types, attrs.recursive)?;

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
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input,
                "Parse cannot be derived for unions",
            ));
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
                // TODO: Make the recursion gaurd only wrap when opted in
                // Or make a better solution for detecting and preventing infinite recursion that doesn't require opt-in
                #crate_path::parse::with_parse_recursion_guard(context, input, stringify!(#name), |context| {
                    #body
                })
            }
        }
    })
}

fn ensure_recursive_parse_is_opted_in(
    type_name: &Ident,
    field_types: &[syn::Type],
    recursive_opt_in: bool,
) -> syn::Result<()> {
    if recursive_opt_in {
        return Ok(());
    }

    let recursive_field_types: Vec<_> = field_types
        .iter()
        .filter(|ty| type_mentions_name_or_self(ty, type_name))
        .collect();

    if recursive_field_types.is_empty() {
        return Ok(());
    }

    let error_message = format!(
        "recursive parser type detected while deriving `Parse` for `{type_name}`; \
recursive derives are disabled by default because they can create non-terminating parses and stack overflows. \
Add `#[parse(recursive)]` to `{type_name}` to opt in"
    );

    let mut error = syn::Error::new_spanned(recursive_field_types[0], &error_message);
    for ty in recursive_field_types.into_iter().skip(1) {
        error.combine(syn::Error::new_spanned(ty, &error_message));
    }

    Err(error)
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

    type_mentions_ident(ty, |ident| names.contains(&ident.to_string()))
}

fn type_mentions_name_or_self(ty: &syn::Type, name: &Ident) -> bool {
    type_mentions_ident(ty, |ident| ident == name || ident == "Self")
}

fn type_mentions_ident(ty: &syn::Type, predicate: impl Fn(&Ident) -> bool) -> bool {
    struct TypeIdentVisitor<F> {
        predicate: F,
        found: bool,
    }

    impl<'ast, F> Visit<'ast> for TypeIdentVisitor<F>
    where
        F: Fn(&Ident) -> bool,
    {
        fn visit_ident(&mut self, ident: &'ast Ident) {
            if !self.found && (self.predicate)(ident) {
                self.found = true;
                return;
            }

            syn::visit::visit_ident(self, ident);
        }

        fn visit_type(&mut self, ty: &'ast syn::Type) {
            if self.found {
                return;
            }

            syn::visit::visit_type(self, ty);
        }
    }

    let mut visitor = TypeIdentVisitor {
        predicate,
        found: false,
    };
    visitor.visit_type(ty);
    visitor.found
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
