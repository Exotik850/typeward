use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Ident, Path};

use super::{
    ParseGenerics,
    fields::{self, constructor},
};

pub(crate) fn expand_enum_body(
    enum_ident: &Ident,
    data_enum: &DataEnum,
    crate_path: &Path,
    parse_generics: &ParseGenerics,
) -> syn::Result<TokenStream> {
    if data_enum.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            enum_ident,
            "Parse cannot be derived for enums with no variants",
        ));
    }

    let variant_plans: Vec<_> = data_enum
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let plan = fields::build_fields_parse_plan(
                &variant.fields,
                crate_path,
                parse_generics,
                &format!("__typeward_variant_{index}"),
            )?;
            let variant_ident = &variant.ident;
            let value_expr = constructor(Some(variant_ident), &plan.shape, &plan.bindings);
            Ok((plan, value_expr))
        })
        .collect::<syn::Result<Vec<_>>>()?;

    if variant_plans.len() == 1 {
        let (plan, value_expr) = &variant_plans[0];
        return Ok(expand_single_variant(plan, value_expr));
    }

    Ok(expand_multi_variant(
        &variant_plans,
        crate_path,
        parse_generics,
    ))
}

fn expand_single_variant(plan: &fields::ParsedFields, value_expr: &TokenStream) -> TokenStream {
    let parse_tokens = &plan.parse_tokens;

    quote! {
        #parse_tokens
        let __typeward_value = #value_expr;
        Ok((__typeward_value, __typeward_remaining))
    }
}

fn expand_multi_variant(
    variant_plans: &[(fields::ParsedFields, TokenStream)],
    crate_path: &Path,
    parse_generics: &ParseGenerics,
) -> TokenStream {
    let input_ident = &parse_generics.input_ident;
    let lifetime = &parse_generics.lifetime;

    let parser_types: Vec<_> = variant_plans
        .iter()
        .map(|(plan, _)| fields::parser_type_from_field_plans(&plan.field_plans, crate_path))
        .collect();

    // Each variant binds to a single identifier, then unpacks inside the value expression
    let parsed_idents: Vec<_> = variant_plans
        .iter()
        .enumerate()
        .map(|(index, _)| quote::format_ident!("__typeward_parsed_{index}"))
        .collect();

    let patterns = variant_plans
        .iter()
        .zip(&parsed_idents)
        .map(|((plan, _), parsed_ident)| {
            if plan.bindings.is_empty() {
                quote!(_)
            } else {
                quote!(#parsed_ident)
            }
        });

    let value_exprs = variant_plans.iter().zip(&parsed_idents).enumerate().map(
        |(index, ((plan, direct_expr), parsed_ident))| match plan.bindings.len() {
            0 => quote!(#direct_expr),
            _ => {
                let parsed_value = quote!(#parsed_ident);
                let parsed_binding_prefix = format!("__typeward_variant_{index}_parsed_field");
                let binding_setup_tokens = fields::build_binding_transform_tokens(
                    &plan.bindings,
                    &plan.field_plans,
                    &parsed_value,
                    crate_path,
                    &parsed_binding_prefix,
                );

                quote!({
                    #binding_setup_tokens
                    #direct_expr
                })
            }
        },
    );

    quote! {
        let (__typeward_or_value, __typeward_remaining) =
            <#crate_path::or!(#(#parser_types),*) as #crate_path::parse::Parse<#lifetime, #input_ident>>::parse_with_context(input, context)?;
        let __typeward_value = #crate_path::or_match!(
            __typeward_or_value,
            #(#patterns => #value_exprs),*
        );
        Ok((__typeward_value, __typeward_remaining))
    }
}
