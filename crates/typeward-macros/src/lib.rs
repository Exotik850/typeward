mod attrs;
mod expand;

#[proc_macro_derive(Parse, attributes(parse))]
pub fn derive_parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand::derive_parse(input)
}
