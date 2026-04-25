use typeward::prelude::*;
use typeward_macros::Parse;

fn ws_into_inner<T>(value: Ws<T>) -> T {
    value.into_inner()
}

fn non_zero(value: i64) -> Result<std::num::NonZeroI64, &'static str> {
    std::num::NonZeroI64::new(value).ok_or("expected non-zero integer")
}

#[derive(Debug, PartialEq, Parse)]
struct NamedPair {
    left: Ws<i64>,
    right: Ws<Null>,
}

#[derive(Debug, PartialEq, Parse)]
struct TuplePair(Ws<True>, Ws<i64>);

#[derive(Debug, PartialEq, Parse)]
enum Value {
    Null(Ws<Null>),
    Number(Ws<i64>),
    Identifier(Ws<IdentifierString>),
}

#[derive(Debug, PartialEq, Parse)]
enum CompositeValue {
    Pair { left: Ws<True>, value: Ws<i64> },
    Word(Ws<IdentifierString>),
}

#[derive(Debug, PartialEq, Parse)]
struct Wrapped<T>
where
    T: Clone,
{
    inner: Ws<T>,
    end: Ws<Null>,
}

#[derive(Debug, PartialEq, Parse)]
#[parse(crate = typeward)]
struct ExplicitCratePath {
    value: Ws<i64>,
}

#[derive(Debug, PartialEq, Parse)]
#[allow(non_snake_case)]
struct DocStringsAndAttributes {
    /// Docstrings are allowed,
    /// as well as attributes like `#[allow(non_snake_case)]`.
    vAlue: Ws<i64>,
}

#[derive(Debug, PartialEq)]
struct NonParseString(String);

#[derive(Debug, PartialEq, Parse)]
struct MappedFields {
    #[parse(from(Ws<String>, |ws| NonParseString(ws.into_inner())))]
    word: NonParseString,
    #[parse(from(Ws<i64>, ws_into_inner))]
    number: i64,
}

#[derive(Debug, PartialEq, Parse)]
enum MappedValue {
    Number(#[parse(from(Ws<i64>, |ws| ws.into_inner()))] i64),
    Word(#[parse(from(Ws<String>, |ws| NonParseString(ws.into_inner())))] NonParseString),
}

#[derive(Debug, PartialEq, Parse)]
struct WsAttributeNamed {
    #[parse(ws)]
    left: i64,
    #[parse(ws)]
    right: Null,
}

#[derive(Debug, PartialEq, Parse)]
struct WsAttributeTuple(#[parse(ws)] True, #[parse(ws)] i64);

#[derive(Debug, PartialEq, Parse)]
enum WsAttributeEnum {
    Pair {
        #[parse(ws)]
        left: True,
        #[parse(ws)]
        value: i64,
    },
    Word(#[parse(ws)] IdentifierString),
}

#[derive(Debug, PartialEq, Parse)]
struct WithGeneric<T>
// where
//     T: Parse,
{
    value: Ws<T>,
}

#[derive(Debug, PartialEq, Parse)]
struct MapOnlyField {
    #[parse(map(|value: i64| value + 1))]
    value: i64,
}

#[derive(Debug, PartialEq, Parse)]
struct TryMappedField {
    #[parse(from(i64), try_map(non_zero))]
    value: std::num::NonZeroI64,
}

#[derive(Debug, PartialEq, Parse)]
struct MapperPipelineField {
    #[parse(from(Ws<String>), map(|ws| ws.into_inner()), map(NonParseString))]
    value: NonParseString,
}

#[derive(Debug, PartialEq, Parse)]
struct FromTypeOnlyField {
    #[parse(from(Ws<i64>))]
    value: Ws<i64>,
}

#[test]
fn derive_parse_named_struct_uses_and_semantics() {
    let parsed = parse_complete::<NamedPair>("42 null").unwrap();
    assert_eq!(parsed.left, 42);
    assert_eq!(parsed.right, Null);
}

#[test]
fn derive_parse_tuple_struct_uses_and_semantics() {
    let parsed = parse_complete::<TuplePair>("true 7").unwrap();
    assert_eq!(parsed.0, True);
    assert_eq!(parsed.1, 7);
}

#[test]
fn derive_parse_enum_uses_or_semantics() {
    let number = parse_complete::<Value>("99").unwrap();
    assert!(matches!(number, Value::Number(number) if number == 99));

    let ident = parse_complete::<Value>("name_1").unwrap();
    assert!(matches!(ident, Value::Identifier(value) if value.value == "name_1"));

    let null = parse_complete::<Value>("null").unwrap();
    assert!(matches!(null, Value::Null(_)));
}

#[test]
fn derive_parse_enum_combines_and_and_or() {
    let pair = parse_complete::<CompositeValue>("true 7").unwrap();
    assert!(matches!(pair, CompositeValue::Pair { left: _, value } if value == 7));

    let word = parse_complete::<CompositeValue>("alpha_9").unwrap();
    assert!(matches!(word, CompositeValue::Word(value) if value.value == "alpha_9"));
}

#[test]
fn derive_parse_generic_struct_adds_parse_bounds() {
    let parsed = parse_complete::<Wrapped<i64>>("15 null").unwrap();
    assert_eq!(parsed.inner, 15);
}

#[test]
fn derive_parse_accepts_explicit_crate_attribute() {
    let parsed = parse_complete::<ExplicitCratePath>("5").unwrap();
    assert_eq!(parsed.value, 5);
}

#[test]
fn derive_parse_allows_docstrings_and_attributes() {
    let parsed = parse_complete::<DocStringsAndAttributes>("8").unwrap();
    assert_eq!(parsed.vAlue, 8);
}

#[test]
fn derive_parse_field_from_attribute_supports_closure_and_function_mappers() {
    let parsed = parse_complete::<MappedFields>("   hello  42").unwrap();
    assert_eq!(parsed.word, NonParseString("hello".into()));
    assert_eq!(parsed.number, 42);
}

#[test]
fn derive_parse_field_from_attribute_works_for_enum_variants() {
    let word = parse_complete::<MappedValue>("   alpha").unwrap();
    assert!(matches!(word, MappedValue::Word(value) if value == NonParseString("alpha".into())));

    let number = parse_complete::<MappedValue>("   11").unwrap();
    assert!(matches!(number, MappedValue::Number(value) if value == 11));
}

#[test]
fn derive_parse_with_generic_field_from_attribute() {
    let parsed = parse_complete::<WithGeneric<Ws<String>>>("   hello").unwrap();
    assert_eq!(*parsed.value.into_inner(), "hello");
}

#[test]
fn derive_parse_field_ws_attribute_for_named_struct() {
    let parsed = parse_complete::<WsAttributeNamed>("   42   null").unwrap();
    assert_eq!(parsed.left, 42);
    assert_eq!(parsed.right, Null);
}

#[test]
fn derive_parse_field_ws_attribute_for_tuple_struct() {
    let parsed = parse_complete::<WsAttributeTuple>("   true   7").unwrap();
    assert_eq!(parsed.0, True);
    assert_eq!(parsed.1, 7);
}

#[test]
fn derive_parse_field_ws_attribute_for_enum_variants() {
    let pair = parse_complete::<WsAttributeEnum>("   true   7").unwrap();
    assert!(matches!(pair, WsAttributeEnum::Pair { left: _, value } if value == 7));

    let word = parse_complete::<WsAttributeEnum>("   alpha_9").unwrap();
    assert!(matches!(word, WsAttributeEnum::Word(value) if value.value == "alpha_9"));
}

#[test]
fn derive_parse_field_map_attribute_applies_post_parse_transform() {
    let parsed = parse_complete::<MapOnlyField>("41").unwrap();
    assert_eq!(parsed.value, 42);
}

#[test]
fn derive_parse_field_try_map_attribute_supports_fallible_transform() {
    let parsed = parse_complete::<TryMappedField>("7").unwrap();
    assert_eq!(parsed.value.get(), 7);

    let err = parse_complete::<TryMappedField>("0").unwrap_err();
    assert!(err.to_string().contains("expected non-zero integer"));
}

#[test]
fn derive_parse_field_map_pipeline_supports_composition() {
    let parsed = parse_complete::<MapperPipelineField>("  hello").unwrap();
    assert_eq!(parsed.value, NonParseString("hello".into()));
}

#[test]
fn derive_parse_field_from_type_only_is_supported() {
    let parsed = parse_complete::<FromTypeOnlyField>("   5").unwrap();
    assert_eq!(parsed.value, 5);
}
