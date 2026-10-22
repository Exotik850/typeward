mod attrs;
mod expand;

#[proc_macro_derive(Parse, attributes(parse))]
/// Derives `typeward::parse::Parse` for structs and enums.
///
/// The generated parser follows field order for structs and tries enum
/// variants in declaration order. Use `#[parse(recursive)]` to opt into
/// recursive parser definitions.
pub fn derive_parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand::derive_parse(input)
}
