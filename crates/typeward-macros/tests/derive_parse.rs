use typeward::prelude::*;

#[derive(Debug, PartialEq, Parse)]
struct NamedPair {
    left: Ws<i64>,
    right: Ws<KwNull>,
}

#[derive(Debug, PartialEq, Parse)]
struct TuplePair(Ws<KwTrue>, Ws<i64>);

#[derive(Debug, PartialEq, Parse)]
enum Value {
    Null(Ws<KwNull>),
    Number(Ws<i64>),
    Identifier(Ws<IdentifierString>),
}

#[derive(Debug, PartialEq, Parse)]
enum CompositeValue {
    Pair { left: Ws<KwTrue>, value: Ws<i64> },
    Word(Ws<IdentifierString>),
}

#[derive(Debug, PartialEq, Parse)]
struct Wrapped<T>
where
    T: Clone,
{
    inner: Ws<T>,
    end: Ws<KwNull>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Parse)]
union NumericAndNull {
    number: Ws<i64>,
    null: Ws<KwNull>,
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

#[test]
fn derive_parse_named_struct_uses_and_semantics() {
    let parsed = parse_complete::<NamedPair>("42 null").unwrap();
    assert_eq!(parsed.left, 42);
    assert_eq!(parsed.right, KwNull);
}

#[test]
fn derive_parse_tuple_struct_uses_and_semantics() {
    let parsed = parse_complete::<TuplePair>("true 7").unwrap();
    assert_eq!(parsed.0, KwTrue);
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
fn derive_parse_union_parses_all_fields_like_and() {
    let (parsed, rest) = NumericAndNull::parse("10 null tail").unwrap();
    assert_eq!(rest, " tail");

    // SAFETY: The derive implementation initializes the `number` field.
    let number = unsafe { parsed.number };
    assert_eq!(number, 10);
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
