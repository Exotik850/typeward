use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, Path};

use super::{
    ParseGenerics,
    fields::{self},
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
