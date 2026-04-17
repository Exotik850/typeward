use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, DataUnion, Path};

use super::{
    ParseGenerics,
    fields::{self, FieldShape},
};

pub(crate) fn expand_struct_body(
    data_struct: &DataStruct,
    crate_path: &Path,
    parse_generics: &ParseGenerics,
) -> syn::Result<TokenStream> {
    let plan = fields::build_fields_parse_plan(
        &data_struct.fields,
        crate_path,
        parse_generics,
        "__typeward_struct_field",
    )?;

    let parse_tokens = plan.parse_tokens;
    let constructor = fields::struct_constructor(&plan.shape, &plan.bindings);

    Ok(quote! {
        #parse_tokens
        let __typeward_value = #constructor;
        Ok((__typeward_value, __typeward_remaining))
    })
}

pub(crate) fn expand_union_body(
    data_union: &DataUnion,
    crate_path: &Path,
    parse_generics: &ParseGenerics,
) -> syn::Result<TokenStream> {
    let union_fields = syn::Fields::Named(data_union.fields.clone());
    let plan = fields::build_fields_parse_plan(
        &union_fields,
        crate_path,
        parse_generics,
        "__typeward_union_field",
    )?;

    let field_names = match &plan.shape {
        FieldShape::Named(names) if !names.is_empty() => names,
        _ => {
            return Err(syn::Error::new_spanned(
                &data_union.fields,
                "Parse can only be derived for unions with named fields",
            ));
        }
    };

    let parse_tokens = plan.parse_tokens;
    let constructor = fields::union_constructor(field_names, &plan.bindings);

    Ok(quote! {
        #parse_tokens
        let __typeward_value = #constructor;
        Ok((__typeward_value, __typeward_remaining))
    })
}
